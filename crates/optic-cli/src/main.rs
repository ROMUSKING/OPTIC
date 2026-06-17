//! opticc — narrow v0 compiler CLI (appendix B command surface).

use anyhow::Context;
use clap::{Parser, Subcommand};
use optic_cgir::CgirGraph;
use optic_syntax::{parse, SourceId};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

#[derive(Parser, Debug)]
#[command(
    name = "opticc",
    version,
    about = "Optic narrow v0 compiler (book implementation)"
)]
struct Cli {
    /// Show full cargo build stderr (default redacts absolute paths)
    #[arg(long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse -> HIR -> type/grade/alias check (M0–M2)
    Check {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Full pipeline transpile (syntax->hir->typeck->cgir->opt->codegen)
    Transpile {
        file: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Dump tokens (M0)
    DumpTokens { file: PathBuf },
    /// Dump AST (M0 goldens)
    DumpAst { file: PathBuf },
    /// Dump HIR and summaries (M1)
    DumpHir { file: PathBuf },
    /// Dump CGIR (M3/M4; --before-fusion for pre-opt graph)
    DumpCgir {
        file: PathBuf,
        #[arg(long)]
        before_fusion: bool,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        node: Option<u32>,
    },
    /// Transpile + verified execution harness (M5/M6)
    Run { file: PathBuf },
    /// Explain a diagnostic code (appendix B stub)
    Explain { code: String },
    /// Environment / toolchain sanity check (appendix B stub)
    Doctor,
    /// Dump OpticSummary for an optic or CGIR node (appendix B stub)
    DumpSummary {
        file: PathBuf,
        #[arg(long)]
        node: Option<u32>,
    },
    /// Run acceptance harnesses and compare to baselines (appendix B stub)
    Bench {
        #[arg(long)]
        update: bool,
    },
    /// Update golden fixtures after review (appendix B)
    SnapshotUpdate {
        #[arg(long)]
        confirm: bool,
    },
}

/// Maximum `.opt` source size (4 MiB) to avoid OOM on hostile inputs.
const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;
/// Bench timing tolerance multiplier vs committed baseline.
const BENCH_TOLERANCE_MULT: u128 = 5;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check { file, json } => {
            let src = read_source(&file)?;
            match compile_check(&src) {
                Ok(()) => {
                    if json {
                        println!("{{\"ok\":true}}");
                    } else {
                        println!("OK (full check): {}", file.display());
                    }
                }
                Err(diags) => {
                    if json {
                        eprintln!("{}", optic_diagnostics::diagnostics_to_json(&diags));
                    } else {
                        for d in &diags {
                            eprintln!("{}", optic_diagnostics::emit_human(d));
                        }
                    }
                    std::process::exit(1);
                }
            }
        }
        Commands::Transpile { file, out } => {
            let src = read_source(&file)?;
            let emitted = compile_emit(&src)?;
            let out_path = safe_output_path(&file, out)?;
            fs::write(&out_path, &emitted)?;
            println!("transpiled -> {}", out_path.display());
        }
        Commands::DumpTokens { file } => {
            let src = read_source(&file)?;
            print!("{}", optic_syntax::dump_tokens(&src, SourceId(1)));
        }
        Commands::DumpAst { file } => {
            let src = read_source(&file)?;
            let prog = parse_or_exit(&src)?;
            println!("{}", optic_syntax::dump_ast(&prog));
        }
        Commands::DumpHir { file } => {
            let src = read_source(&file)?;
            let hir = lower_or_exit(&src)?;
            println!("{}", optic_hir::dump_hir(&hir));
        }
        Commands::DumpCgir {
            file,
            before_fusion,
            check,
            node,
        } => {
            let src = read_source(&file)?;
            let graph = compile_cgir(&src, before_fusion)?;
            if check {
                optic_cgir::verify(&graph).map_err(|e| anyhow::anyhow!(e))?;
                println!("CGIR verify: OK");
            }
            if let Some(n) = node {
                if let Some(nd) = graph.nodes.get(n as usize) {
                    println!("{nd:#?}");
                    if let Some(p) = graph.provenance_index.get(&n) {
                        println!("provenance: {p:#?}");
                    }
                } else {
                    anyhow::bail!("node {n} not found");
                }
            } else {
                println!("{}", optic_cgir::dump_pretty(&graph));
            }
        }
        Commands::Run { file } => {
            let src = read_source(&file)?;
            let emitted = compile_emit(&src)?;
            run_verification_harness(&emitted, &file, cli.verbose)?;
        }
        Commands::Explain { code } => {
            println!("{}", explain_code(&code));
        }
        Commands::Doctor => {
            doctor_check()?;
        }
        Commands::DumpSummary { file, node } => {
            let src = read_source(&file)?;
            dump_summary(&src, node)?;
        }
        Commands::Bench { update } => {
            bench_examples(update, cli.verbose)?;
        }
        Commands::SnapshotUpdate { confirm } => {
            if !confirm {
                anyhow::bail!("refusing snapshot update without --confirm");
            }
            snapshot_update_goldens()?;
        }
    }
    Ok(())
}

fn safe_output_path(file: &Path, out: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    let out_path = out.unwrap_or_else(|| file.with_extension("rs"));
    if out_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        anyhow::bail!(
            "output path {} must not contain '..'; use --out with a safe path",
            out_path.display()
        );
    }
    Ok(out_path)
}

const TRUSTED_TOOL_DIRS: &[&str] = &["/usr/bin", "/bin"];

fn tool_lookup_path() -> String {
    TRUSTED_TOOL_DIRS.join(":")
}

fn resolve_tool_bin(program: &str) -> PathBuf {
    static CACHE: OnceLock<std::collections::HashMap<String, PathBuf>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut map = std::collections::HashMap::new();
            for name in ["cargo", "rustc", "which"] {
                if let Some(path) = locate_tool_bin(name) {
                    map.insert(name.into(), path);
                }
            }
            map
        })
        .get(program)
        .cloned()
        .unwrap_or_else(|| PathBuf::from(program))
}

fn locate_tool_bin(program: &str) -> Option<PathBuf> {
    for dir in TRUSTED_TOOL_DIRS {
        let candidate = Path::new(dir).join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Ok(rustup_home) = std::env::var("RUSTUP_HOME") {
        let toolchains = Path::new(&rustup_home).join("toolchains");
        if let Ok(entries) = fs::read_dir(&toolchains) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("bin").join(program);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn trusted_tool_path() -> String {
    let mut dirs: Vec<String> = tool_lookup_path().split(':').map(str::to_string).collect();
    for name in ["cargo", "rustc"] {
        if let Some(path) = locate_tool_bin(name) {
            if let Some(parent) = path.parent() {
                let s = parent.to_string_lossy().to_string();
                if !dirs.contains(&s) {
                    dirs.insert(0, s);
                }
            }
        }
    }
    dirs.join(":")
}

/// Isolated subprocess env: HOME/CARGO_HOME/RUSTUP_HOME in work_home; toolchain via symlinked toolchains.
fn sandbox_command(program: &str, work_home: &Path) -> Command {
    let cargo_home = work_home.join("cargo-home");
    let rustup_home = work_home.join("rustup-home");
    let _ = fs::create_dir_all(&cargo_home);
    let _ = fs::create_dir_all(&rustup_home);
    let mut cmd = Command::new(resolve_tool_bin(program));
    if let Ok(rustup_home_env) = std::env::var("RUSTUP_HOME") {
        let parent_toolchains = Path::new(&rustup_home_env).join("toolchains");
        let link = rustup_home.join("toolchains");
        if parent_toolchains.exists() {
            let _ = std::fs::remove_dir_all(&link);
            #[cfg(unix)]
            let _ = std::os::unix::fs::symlink(&parent_toolchains, &link);
        }
    }
    cmd.env_clear()
        .env("PATH", trusted_tool_path())
        .env("HOME", work_home)
        .env("CARGO_HOME", &cargo_home)
        .env("RUSTUP_HOME", &rustup_home)
        .env("RUSTUP_TOOLCHAIN", "stable");
    cmd
}

fn snapshot_update_goldens() -> anyhow::Result<()> {
    println!("Updating goldens (OPTIC_UPDATE_GOLDEN=1)...");
    let work = tempfile::tempdir().context("sandbox dir for snapshot update")?;
    let status = sandbox_command("cargo", work.path())
        .env("OPTIC_UPDATE_GOLDEN", "1")
        .args(["test", "-p", "optic-syntax", "golden_", "--", "--quiet"])
        .status()
        .context("optic-syntax golden update")?;
    if !status.success() {
        anyhow::bail!("optic-syntax golden update failed");
    }
    let status = sandbox_command("cargo", work.path())
        .env("OPTIC_UPDATE_GOLDEN", "1")
        .args(["test", "-p", "optic-cli", "golden_cgir", "--", "--quiet"])
        .status()
        .context("optic-cli cgir golden update")?;
    if !status.success() {
        anyhow::bail!("optic-cli cgir golden update failed");
    }
    let status = sandbox_command("cargo", work.path())
        .env("OPTIC_UPDATE_GOLDEN", "1")
        .args([
            "test",
            "-p",
            "optic-cli",
            "diagnostics_json",
            "--",
            "--quiet",
        ])
        .status()
        .context("optic-cli diagnostics json golden update")?;
    if !status.success() {
        anyhow::bail!("diagnostics json golden update failed");
    }
    let status = sandbox_command("cargo", work.path())
        .env("OPTIC_UPDATE_GOLDEN", "1")
        .args(["test", "-p", "optic-hir", "golden_hir", "--", "--quiet"])
        .status()
        .context("optic-hir golden update")?;
    if !status.success() {
        anyhow::bail!("optic-hir golden update failed");
    }
    bench_examples(true, false)?;
    println!("Goldens updated. Review diffs before commit.");
    Ok(())
}

fn explain_code(code: &str) -> String {
    let (title, rule, phase) = match code {
        "GRA-110" => (
            "declared grade tighter than inferred",
            "CacheGrade annotation must be >= inferred distinct-region count (ch9.9.3)",
            "grade",
        ),
        "GRA-104" => (
            "sequential composition exceeds cache bound",
            ">>> cache grades combine with sat_add (ch9.9.4)",
            "grade",
        ),
        "ALI-201" | "ALI-101" => (
            "alias conflict in product composition",
            "put_reads hazards across product arms (ch9)",
            "alias",
        ),
        "PAR-001" => (
            "parse error",
            "surface syntax does not match appendix D EBNF",
            "parse",
        ),
        "CGI-001" | "CGI-002" => (
            "CGIR invariant violation",
            "graph wiring / unresolved optic (ch10)",
            "cgir",
        ),
        "CGI-003" => (
            "unsupported expression in query body",
            "map/set value uses unsupported surface forms (v0)",
            "type",
        ),
        "CGI-004" => (
            "fusion or CGIR verify failed",
            "post-fusion graph invariant violated (ch10)",
            "fusion",
        ),
        "CGI-005" => (
            "codegen failed",
            "emitted Rust would not compile or tuple arity mismatch (ch11)",
            "codegen",
        ),
        "RES-001" => (
            "name resolution failed",
            "unknown optic or unresolved binding (ch8)",
            "resolve",
        ),
        _ => (
            "unknown code",
            "no catalog entry yet; see optic-diagnostics",
            "unknown",
        ),
    };
    format!("{code}: {title}\nphase: {phase}\nrule: {rule}\nnext: optic check <file.opt> --json")
}

fn doctor_check() -> anyhow::Result<()> {
    let rustc = Command::new("rustc").arg("--version").output();
    let cargo = Command::new("cargo").arg("--version").output();
    match (&rustc, &cargo) {
        (Ok(r), Ok(c)) if r.status.success() && c.status.success() => {
            println!("rustc: {}", String::from_utf8_lossy(&r.stdout).trim());
            println!("cargo: {}", String::from_utf8_lossy(&c.stdout).trim());
        }
        _ => anyhow::bail!("rustc/cargo not available"),
    }
    let runtime = validated_runtime_crate_path()?;
    println!("optic-runtime: OK ({})", runtime.display());
    println!("doctor: OK");
    Ok(())
}

fn dump_summary(src: &str, node: Option<u32>) -> anyhow::Result<()> {
    let graph = compile_cgir(src, false)?;
    if let Some(n) = node {
        if let Some(nd) = graph.nodes.get(n as usize) {
            if let optic_cgir::CgirNode::OpticLeaf { summary, name, .. } = nd {
                println!("summary for node {n} ({name}): {summary:#?}");
            } else {
                println!("node {n}: {nd:#?}");
            }
        } else {
            anyhow::bail!("node {n} not found");
        }
    } else {
        let hir = lower_or_exit(src)?;
        for item in &hir.items {
            if let optic_hir::HirItem::Optic { decl, summary } = item {
                println!("{}: {summary:#?}", decl.name.node);
            }
        }
    }
    Ok(())
}

fn redact_build_stderr(stderr: &str) -> String {
    stderr
        .lines()
        .map(|line| {
            let mut out = line.to_string();
            for token in line.split(|c: char| {
                c.is_whitespace() || matches!(c, '(' | ')' | '"' | '\'' | ',' | ':')
            }) {
                if token.starts_with('/') && token.len() > 1 {
                    if let Some(fname) = Path::new(token).file_name() {
                        let redacted = format!("[path]/{}", fname.to_string_lossy());
                        out = out.replace(token, &redacted);
                    }
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_build_failure(stderr: &str, verbose: bool) -> String {
    if verbose {
        stderr.to_string()
    } else {
        let redacted = redact_build_stderr(stderr);
        if redacted.lines().count() > 12 {
            let head: Vec<_> = redacted.lines().take(6).collect();
            let tail: Vec<_> = redacted
                .lines()
                .rev()
                .take(4)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            format!(
                "{}\n... (use --verbose for full cargo output) ...\n{}",
                head.join("\n"),
                tail.join("\n")
            )
        } else {
            redacted
        }
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn validated_runtime_crate_path() -> anyhow::Result<PathBuf> {
    let runtime_rel = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../optic-runtime");
    let lib = runtime_rel.join("src/lib.rs");
    if !lib.exists() {
        anyhow::bail!("optic-runtime not found at {}", runtime_rel.display());
    }
    let canonical = fs::canonicalize(&runtime_rel)
        .with_context(|| format!("canonicalize {}", runtime_rel.display()))?;
    let workspace = fs::canonicalize(workspace_root())?;
    if !canonical.starts_with(&workspace) {
        anyhow::bail!(
            "optic-runtime path {} escapes workspace root {}",
            canonical.display(),
            workspace.display()
        );
    }
    Ok(canonical)
}

fn bench_examples(update: bool, verbose: bool) -> anyhow::Result<()> {
    let examples = [
        "health_decay.opt",
        "health_position.opt",
        "health_get.opt",
        "health_set.opt",
    ];
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bench");
    if update {
        fs::create_dir_all(&bench_dir)?;
    }
    for ex in examples {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(ex);
        let src = read_source(&path)?;
        let start = std::time::Instant::now();
        let emitted = compile_emit(&src)?;
        let compile_ms = start.elapsed().as_millis().max(1);
        let run_start = std::time::Instant::now();
        run_verification_harness(&emitted, &path, verbose)?;
        let run_ms = run_start.elapsed().as_millis().max(1);
        let line = format!("{ex}: compile_ms={compile_ms} run_ms={run_ms} ok=1\n");
        let baseline = bench_dir.join(ex.replace(".opt", ".txt"));
        if update {
            fs::write(&baseline, &line)?;
            println!("updated {}", baseline.display());
        } else if baseline.exists() {
            let expected = fs::read_to_string(&baseline)?;
            if !expected.contains("ok=1") {
                anyhow::bail!("baseline failed for {ex}");
            }
            if let (Some((ec, er)), Some((nc, nr))) =
                (parse_bench_line(&expected), parse_bench_line(&line))
            {
                let compile_limit = ec * BENCH_TOLERANCE_MULT;
                let run_limit = er * BENCH_TOLERANCE_MULT;
                if nc > compile_limit || nr > run_limit {
                    anyhow::bail!(
                        "bench regression for {ex} (tolerance {BENCH_TOLERANCE_MULT}x): \
                         baseline compile_ms={ec} run_ms={er}, got compile_ms={nc} run_ms={nr}; \
                         limits compile_ms<={compile_limit} run_ms<={run_limit}"
                    );
                }
            }
            println!("{line}within tolerance ({BENCH_TOLERANCE_MULT}x baseline; compile/run ms)");
        } else {
            println!("{line}(no baseline; use --update)");
        }
    }
    Ok(())
}

fn parse_bench_line(s: &str) -> Option<(u128, u128)> {
    let compile = s
        .split("compile_ms=")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    let run = s
        .split("run_ms=")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some((compile, run))
}

fn read_source(file: &Path) -> anyhow::Result<String> {
    let meta = fs::metadata(file).with_context(|| format!("stat {}", file.display()))?;
    if meta.len() > MAX_SOURCE_BYTES {
        anyhow::bail!(
            "source {} exceeds {} byte limit",
            file.display(),
            MAX_SOURCE_BYTES
        );
    }
    fs::read_to_string(file).with_context(|| format!("read {}", file.display()))
}

fn parse_or_exit(src: &str) -> anyhow::Result<optic_syntax::Program> {
    let sid = SourceId(1);
    parse(src, sid).map_err(|errs| {
        for e in &errs {
            eprintln!("parse: {}", e.message);
        }
        anyhow::anyhow!("parse failed ({} errors)", errs.len())
    })
}

fn lower_or_exit(src: &str) -> anyhow::Result<optic_hir::HirProgram> {
    let prog = parse_or_exit(src)?;
    optic_hir::lower(prog).map_err(|errs| {
        for e in &errs {
            eprintln!("resolve: {}", e.message);
        }
        anyhow::anyhow!("hir lower failed")
    })
}

fn lower_to_diags(errs: Vec<optic_syntax::ParseError>) -> Vec<optic_diagnostics::Diagnostic> {
    errs.into_iter()
        .map(|e| optic_diagnostics::resolve_diag(e.span, e.message))
        .collect()
}

fn compile_check(src: &str) -> Result<(), Vec<optic_diagnostics::Diagnostic>> {
    let prog = parse(src, SourceId(1)).map_err(|errs| {
        errs.into_iter()
            .map(|e| optic_diagnostics::parse_diag(e.span, e.message))
            .collect::<Vec<_>>()
    })?;
    let hir = optic_hir::lower(prog).map_err(lower_to_diags)?;
    let typed = optic_typeck::check(hir)?;
    let cg = optic_cgir::build(&typed).map_err(|diags| diags)?;
    let graph =
        optic_opt::optimize(cg).map_err(|e| vec![optic_diagnostics::fusion_verify_diag(&e)])?;
    optic_cgir::verify(&graph).map_err(|e| vec![optic_diagnostics::fusion_verify_diag(&e)])?;
    optic_codegen_rust::emit(&graph, "optic_runtime")
        .map_err(|e| vec![optic_diagnostics::codegen_failed_diag(&e)])?;
    Ok(())
}

fn compile_cgir(src: &str, before_fusion: bool) -> anyhow::Result<CgirGraph> {
    let prog = parse_or_exit(src)?;
    let hir = optic_hir::lower(prog).map_err(|errs| {
        for e in &errs {
            eprintln!("resolve: {}", e.message);
        }
        anyhow::anyhow!("hir lower failed")
    })?;
    let typed = optic_typeck::check(hir).map_err(|diags| {
        for d in &diags {
            eprintln!("{}", optic_diagnostics::emit_human(d));
        }
        anyhow::anyhow!("type check failed ({} diagnostics)", diags.len())
    })?;
    let cg = optic_cgir::build(&typed).map_err(|diags| {
        for d in &diags {
            eprintln!("{}", optic_diagnostics::emit_human(d));
        }
        anyhow::anyhow!("cgir build failed")
    })?;
    if before_fusion {
        Ok(cg)
    } else {
        optic_opt::optimize(cg).map_err(|e| anyhow::anyhow!("fusion verify failed: {e}"))
    }
}

fn compile_emit(src: &str) -> anyhow::Result<String> {
    let graph = compile_cgir(src, false)?;
    optic_codegen_rust::emit(&graph, "optic_runtime")
        .map_err(|e| anyhow::anyhow!("codegen failed: {e}"))
}

fn run_verification_harness(emitted: &str, file: &Path, verbose: bool) -> anyhow::Result<()> {
    let tmp = tempfile::tempdir().context("create temp dir for verification harness")?;
    let vdir = tmp.path();
    let runtime = validated_runtime_crate_path()?;
    fs::write(
        vdir.join("Cargo.toml"),
        format!(
            "[package]\nname=\"v\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[dependencies]\noptic-runtime = {{ path = \"{}\" }}\n[[bin]]\nname=\"v\"\npath=\"main.rs\"\n",
            runtime.display()
        ),
    )?;
    fs::write(vdir.join("main.rs"), emitted)?;
    let manifest = vdir.join("Cargo.toml");
    let out = sandbox_command("cargo", vdir)
        .args(["run", "--quiet", "--manifest-path"])
        .arg(&manifest)
        .current_dir(vdir)
        .output()
        .context("execute cargo run in verification harness")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        eprintln!("{}", format_build_failure(&stderr, verbose));
        anyhow::bail!("verification harness failed");
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    println!("{stdout}");

    let path = file.to_string_lossy();
    let verified = if path.contains("health_position") {
        stdout.contains("99.0") && stdout.contains("0.1")
    } else if path.contains("health_decay") {
        stdout.contains("90.0") && stdout.contains("70.0")
    } else if path.contains("health_set") {
        stdout.contains("42.0")
    } else if path.contains("health_get") {
        stdout.contains("get:")
    } else {
        false
    };

    if verified {
        println!(
            "RUN VERIFIED ({})",
            file.file_name().unwrap().to_string_lossy()
        );
    } else {
        anyhow::bail!(
            "verification predicate did not match output for {}",
            file.display()
        );
    }
    Ok(())
}
