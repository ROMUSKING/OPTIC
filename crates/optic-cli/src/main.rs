//! opticc — narrow v0 compiler CLI (appendix B command surface).

use anyhow::Context;
use clap::{Parser, Subcommand};
use optic::{
    compile_cgir, compile_check, compile_emit, dump_ast_src, dump_hir_src, explain_focus_from_src,
    explain_grade_from_src, lower_src, SourceId,
};

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
        node: Option<String>,
    },
    /// Transpile + verified execution harness (M5/M6)
    Run { file: PathBuf },
    /// Explain a diagnostic code (appendix B)
    Explain { code: String },
    /// Show normalized grade after inference for a named optic (appendix B)
    ExplainGrade {
        file: PathBuf,
        #[arg(long)]
        node: String,
        #[arg(long)]
        json: bool,
    },
    /// Show PathLift / root-path form for a named optic or let binding (appendix B)
    ExplainFocus {
        file: PathBuf,
        #[arg(long)]
        node: String,
        #[arg(long)]
        json: bool,
    },
    /// Environment / toolchain sanity check; optional file runs `check` (appendix B)
    Doctor { file: Option<PathBuf> },
    /// Dump OpticSummary for an optic/let name or CGIR node id (appendix B)
    DumpSummary {
        file: PathBuf,
        #[arg(long)]
        node: Option<String>,
    },
    /// Run acceptance harnesses and compare to baselines (appendix B)
    Bench {
        file: Option<PathBuf>,
        #[arg(long)]
        update: bool,
    },
    /// Profile (OBS-701 in narrow v0; placeholder per appendix B / ch14.5)
    Profile { file: PathBuf },
    /// Replay (OBS-701 in narrow v0; placeholder per appendix B / ch14.5)
    Replay { file: PathBuf },
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
                Ok(outcome) => {
                    for note in &outcome.fusion_notes {
                        eprintln!("{}", optic_diagnostics::emit_human(note));
                    }
                    if json {
                        println!(
                            "{}",
                            optic_diagnostics::check_ok_json(&outcome.fusion_notes)
                        );
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
            let emitted = compile_emit_or_exit(&src)?;
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
            match dump_ast_src(&src) {
                Ok(out) => println!("{out}"),
                Err(diags) => return Err(emit_pipeline_errors(&diags)),
            }
        }
        Commands::DumpHir { file } => {
            let src = read_source(&file)?;
            match dump_hir_src(&src) {
                Ok(out) => println!("{out}"),
                Err(diags) => return Err(emit_pipeline_errors(&diags)),
            }
        }
        Commands::DumpCgir {
            file,
            before_fusion,
            check,
            node,
        } => {
            let src = read_source(&file)?;
            let outcome = compile_cgir_or_exit(&src, before_fusion)?;
            let graph = outcome.graph;
            if check {
                optic_cgir::verify_to_diagnostic(&graph).map_err(|d| emit_pipeline_errors(&[d]))?;
                println!("CGIR verify: OK");
            }
            if let Some(n) = node {
                let id = resolve_dump_cgir_node(&graph, &n)?;
                print!("{}", optic_cgir::dump_node_pretty(&graph, id));
            } else {
                println!("{}", optic_cgir::dump_pretty(&graph));
            }
        }
        Commands::Run { file } => {
            let src = read_source(&file)?;
            let emitted = compile_emit_or_exit(&src)?;
            run_verification_harness(&emitted, &file, cli.verbose)?;
        }
        Commands::Explain { code } => {
            println!("{}", explain_code(&code));
        }
        Commands::ExplainGrade { file, node, json } => {
            let src = read_source(&file)?;
            explain_grade_cmd(&src, &node, json)?;
        }
        Commands::ExplainFocus { file, node, json } => {
            let src = read_source(&file)?;
            explain_focus_cmd(&src, &node, json)?;
        }
        Commands::Doctor { file } => {
            doctor_check(file.as_deref())?;
        }
        Commands::DumpSummary { file, node } => {
            let src = read_source(&file)?;
            dump_summary(&src, node.as_deref())?;
        }
        Commands::Bench { file, update } => {
            if let Some(path) = file {
                bench_single_file(&path, update, cli.verbose)?;
            } else {
                bench_examples(update, cli.verbose)?;
            }
        }
        Commands::Profile { file } => {
            let src = read_source(&file)?;
            // delegates to compile which surfaces OBS-701; runtime hooks are stubs
            match compile_check(&src) {
                Ok(_) => println!("profile: no OBS-701 (unexpected)"),
                Err(diags) => {
                    for d in &diags {
                        if d.code == "OBS-701" {
                            eprintln!("{}", optic_diagnostics::emit_human(d));
                        }
                    }
                    println!("profile/replay deferred (OBS-701) in narrow v0");
                }
            }
        }
        Commands::Replay { file } => {
            let src = read_source(&file)?;
            match compile_check(&src) {
                Ok(_) => println!("replay: no OBS-701 (unexpected)"),
                Err(diags) => {
                    for d in &diags {
                        if d.code == "OBS-701" {
                            eprintln!("{}", optic_diagnostics::emit_human(d));
                        }
                    }
                    println!("profile/replay deferred (OBS-701) in narrow v0");
                }
            }
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
    let out_path = out.unwrap_or_else(|| {
        file.file_name()
            .map(|n| PathBuf::from(n).with_extension("rs"))
            .unwrap_or_else(|| PathBuf::from("out.rs"))
    });
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
    // Pin to build-time RUSTUP_HOME only — never caller-controlled env at runtime.
    if let Some(rustup_home) = option_env!("RUSTUP_HOME") {
        let toolchains = Path::new(rustup_home).join("toolchains");
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
///
/// # Error contract
/// Panics (via expect) on directory creation failure. This is intentional fail-fast behavior
/// for the trusted local harness/sandbox model (used by run/bench/doctor/snapshot). Callers
/// (which are test/CLI paths) expect a clean writable env or early abort. Changing to Result
/// would require updating multiple call sites and is out of scope for smallest targeted fix.
/// Documented to address prior review feedback on error model.
fn sandbox_command(program: &str, work_home: &Path) -> Command {
    let cargo_home = work_home.join("cargo-home");
    let rustup_home = work_home.join("rustup-home");
    fs::create_dir_all(&cargo_home).expect("create sandbox cargo-home");
    fs::create_dir_all(&rustup_home).expect("create sandbox rustup-home");
    let mut cmd = Command::new(resolve_tool_bin(program));
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
    let status = sandbox_command("cargo", work.path())
        .env("OPTIC_UPDATE_GOLDEN", "1")
        .args([
            "test",
            "-p",
            "optic-codegen-rust",
            "golden_rust",
            "--",
            "--quiet",
        ])
        .status()
        .context("optic-codegen-rust golden update")?;
    if !status.success() {
        anyhow::bail!("optic-codegen-rust golden update failed");
    }
    bench_examples(true, false)?;
    println!("Goldens updated. Review diffs before commit.");
    Ok(())
}

fn explain_code(code: &str) -> String {
    if code == "TYP-010" {
        return explain_typ010().to_string();
    }
    if code == "TYP-003" {
        return explain_typ003().to_string();
    }
    if code == "CGI-003" {
        return explain_cgi003().to_string();
    }
    if code == "CGI-006" {
        return explain_cgi006().to_string();
    }
    if code == "OBS-701" {
        return explain_obs701().to_string();
    }
    if code == "OBS-702" {
        return explain_obs702().to_string();
    }
    if code == "OBS-703" {
        return explain_obs703().to_string();
    }
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
            "surface syntax does not match appendix D EBNF (includes MAX_PARSE_DEPTH=512 recursion cap)",
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
        "HIR-101" => (
            "duplicate SoA costate data declaration",
            "v0 supports only one data decl with SoA<> columns (ch8)",
            "resolve",
        ),
        "FUS-501" => (
            "compose fusion blocked — intermediate escapes",
            "map body captures an intermediate outside map_param (ch10); keep unfused form",
            "fusion",
        ),
        "FUS-502" => (
            "compose fusion blocked — legality precondition",
            "focus/costate mismatch, impurity, or non-leaf compose child (ch10); keep unfused form",
            "fusion",
        ),
        "TYP-001" => (
            "unknown type",
            "costate or focus type not declared in program (ch9 type universe)",
            "type",
        ),
        "TYP-002" => (
            "type mismatch in optic body",
            "get/put body type does not match declared focus (ch9)",
            "type",
        ),
        "TYP-003" => (
            "invalid grade annotation syntax",
            "malformed OwnershipGrade rational or unknown grade dimension (ch6/9)",
            "type",
        ),
        "TYP-004" => (
            "cannot infer optic body type",
            "get/put body uses a form the v0 type checker cannot infer",
            "type",
        ),
        "EXP-001" => (
            "explain-grade/focus: unknown optic name",
            "no optic or let binding matches --node",
            "type",
        ),
        _ => (
            "unknown code",
            "no catalog entry yet; see optic-diagnostics",
            "unknown",
        ),
    };
    format!("{code}: {title}\nphase: {phase}\nrule: {rule}\nnext: opticc check <file.opt> --json")
}

fn explain_typ003() -> &'static str {
    r#"TYP-003: invalid optic clause combination / grade syntax
phase: type
rule: mutually exclusive optic clause sets and malformed grade annotations

clause_mix (feature=clause_mix in evidence):
  - GradedOptic: get/put only (no preview/review)
  - GradedPrism: preview/review only (no get/put)
  - GradedTraversal: get/put only (no preview/review)

grade syntax:
  - CacheGrade<N>, LinearGrade, AffineGrade, SharedGrade
  - OwnershipGrade<num/den> rational fractions

examples:
  - GradedTraversal + preview → TYP-003 fragment=preview
  - GradedTraversal + review → TYP-003 fragment=review
  - GradedPrism + get → TYP-003 fragment=get

related: opticc explain TYP-004 (missing required clause body)

next: opticc check file.opt --json"#
}

fn explain_cgi003() -> &'static str {
    r#"CGI-003: unsupported expression in query / compose chain
phase: cgir
rule: narrow v0 rejects unsupported optic bodies and M7 leaf placement in compose

compose chain (evidence.reason):
  - prism_in_compose — PrismLeaf in >>> chain (examples/compose_prism.opt)
  - traversal_in_compose — TraversalLeaf in >>> chain (examples/compose_traversal.opt)

map/set body:
  - unsupported surface forms in query map bodies
  - incompatible map chain fusion

fusion note: compose+prism/traversal may also surface as FUS-502 with same reason keys.

examples:
  - examples/compose_prism.opt (prism_in_compose)
  - examples/compose_traversal.opt (traversal_in_compose)

next: opticc check file.opt --json"#
}

fn explain_typ010() -> &'static str {
    r#"TYP-010: unsupported in narrow v0
phase: type
rule: unsafe optic and extern/foreign host boundaries rejected before CGIR

rejected surface:
  - unsafe optic boundaries
  - extern / foreign host declarations

supported (M7):
  - GradedPrism optics (preview/review) — examples/alive_filter.opt
  - GradedTraversal optics (get/put v0 surface) — examples/all_healths.opt
  - PrismLeaf / TraversalLeaf CGIR lowering + Rust codegen (m7_reserved=false)

still deferred:
  - traverse/update surface syntax (book ch13; v0 uses get/put clauses)
  - profile / replay query methods (OBS-701)

examples:
  - examples/alive_filter.opt (positive prism)
  - examples/all_healths.opt (positive traversal)
  - examples/host_boundary.opt (TYP-010 negative)

docs:
  - docs/observability-v0.md (tap/record v0 comment hooks; profile/replay OBS-701)
  - docs/effect-coeffect-v0.md
  - docs/v0-executable-spec.md

related: opticc explain CGI-006 (CGIR verify rejects unstubs M7/M8 reserved nodes)

next: opticc check <file.opt> --json"#
}

fn explain_obs703() -> &'static str {
    r#"OBS-703: invalid observability hook label
phase: type
rule: tap/record/profile/replay hook strings must satisfy narrow v0 label policy

policy:
  - single-line ASCII labels: [A-Za-z0-9_.-]
  - max 128 bytes
  - only \" escape in source literals
  - no control characters or newlines

parse vs type:
  - parser rejects invalid literals at parse time (PAR-001)
  - typeck re-validates decoded labels as defense-in-depth (OBS-703)

docs:
  - docs/observability-v0.md (hook string policy)

next: opticc check <file.opt> --json"#
}

fn explain_obs702() -> &'static str {
    r#"OBS-702: observability hook must precede query methods
phase: type
rule: .tap/.record must appear before .get/.set/.map in narrow v0 (prefix-only)

supported ordering:
  entities.query(Optic).tap("label").record("evt").map(|x| ...)

rejected:
  entities.query(Optic).map(|x| ...).tap("label")

negative witnesses:
  - examples/trailing_tap.opt
  - examples/trailing_record.opt

docs:
  - docs/observability-v0.md

next: opticc check <file.opt> --json"#
}

fn explain_obs701() -> &'static str {
    r#"OBS-701: unsupported observability query method
phase: type
rule: profile/replay query methods are deferred in narrow v0

supported (M8 scaffolding):
  - .tap("label") on query chains — examples/tap_health.opt
  - .record("event") on query chains — examples/record_health.opt
  - Tap / Record CGIR with m7_reserved=false + // optic(tap|record): comment hooks

still deferred:
  - .profile(...) / .replay(...) query methods
  - full profile/replay CLI (appendix B placeholders)

negative witnesses:
  - examples/unsupported_profile.opt
  - examples/unsupported_replay.opt

docs:
  - docs/observability-v0.md

related: opticc explain CGI-006 (stub Tap/Record with m7_reserved=true)

next: opticc check <file.opt> --json"#
}

fn explain_cgi006() -> &'static str {
    r#"CGI-006: M7/M8 reserved CGIR node materialized
phase: cgir
rule: unstubs M7/M8 reserved variants must not appear in narrow v0 graphs

reserved variants:
  - PrismLeaf with m7_reserved=true (stub; use m7_reserved=false for lowered prisms)
  - TraversalLeaf with m7_reserved=true (stub; use m7_reserved=false for lowered traversals)
  - Tap / Record with m7_reserved=true (stub placeholders; rejected via CGI-006)

properly lowered PrismLeaf / TraversalLeaf / Tap / Record (m7_reserved=false) are allowed after M7/M8 lowering.

surface still rejected earlier via TYP-010 for unsafe optic / host syntax.

docs:
  - docs/observability-v0.md
  - docs/effect-coeffect-v0.md

related: opticc explain TYP-010 (surface rejection before CGIR)

next: opticc dump-cgir file.opt --check"#
}

fn resolve_dump_cgir_node(
    graph: &optic_cgir::CgirGraph,
    node: &str,
) -> anyhow::Result<optic_cgir::NodeId> {
    if node.len() > optic_cgir::MAX_NODE_NAME_BYTES {
        anyhow::bail!(
            "node name exceeds {} bytes",
            optic_cgir::MAX_NODE_NAME_BYTES
        );
    }
    match optic_cgir::resolve_cgir_node(graph, node) {
        Ok(id) => Ok(id),
        Err(optic_cgir::ResolveCgirNodeError::NameTooLong) => anyhow::bail!(
            "node name exceeds {} bytes",
            optic_cgir::MAX_NODE_NAME_BYTES
        ),
        Err(optic_cgir::ResolveCgirNodeError::UnknownName { candidates }) => {
            Err(emit_pipeline_errors(&[
                optic_diagnostics::explain_unknown_node_diag(
                    optic_syntax::Span::dummy(),
                    node,
                    &candidates,
                ),
            ]))
        }
        Err(optic_cgir::ResolveCgirNodeError::UnknownId { id }) => {
            anyhow::bail!("node id {id} not found")
        }
        Err(optic_cgir::ResolveCgirNodeError::StaleName { name, id }) => {
            anyhow::bail!("stale resolved_optics entry: name `{name}` maps to missing node id {id}")
        }
    }
}

fn validate_node_name(node: &str) -> anyhow::Result<()> {
    if node.len() > optic_cgir::MAX_NODE_NAME_BYTES {
        anyhow::bail!(
            "node name exceeds {} bytes",
            optic_cgir::MAX_NODE_NAME_BYTES
        );
    }
    Ok(())
}

fn hir_binding_candidates(hir: &optic_hir::HirProgram) -> Vec<String> {
    let mut candidates = vec![];
    for item in &hir.items {
        match item {
            optic_hir::HirItem::Optic { decl, .. } => candidates.push(decl.name.node.clone()),
            optic_hir::HirItem::Let { name, .. } => candidates.push(name.clone()),
            optic_hir::HirItem::Extern(_) => {} // per ch22/appI/PLAN (passes as optics; S0 for S1; 3-ring)
            _ => {}
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

fn doctor_check(file: Option<&Path>) -> anyhow::Result<()> {
    // Use sandbox_command (env_clear + PATH + homes created inside) for exact parity with run/bench harness (dedup logic)
    // Error paths now use .context()? (rustc --version etc) for propagation; non-success still generic bail. See Issue 6 review.
    let work = tempfile::tempdir().context("doctor temp dir")?;
    let rustc = sandbox_command("rustc", work.path())
        .arg("--version")
        .output()
        .context("rustc --version")?;
    let cargo = sandbox_command("cargo", work.path())
        .arg("--version")
        .output()
        .context("cargo --version")?;
    match (rustc.status.success(), cargo.status.success()) {
        (true, true) => {}
        _ => anyhow::bail!("rustc/cargo not available"),
    }
    println!("rustc: {}", String::from_utf8_lossy(&rustc.stdout).trim());
    println!("cargo: {}", String::from_utf8_lossy(&cargo.stdout).trim());
    let runtime = validated_runtime_crate_path()?;
    println!("optic-runtime: OK ({})", runtime.display());
    if let Some(path) = file {
        let src = read_source(path)?;
        match compile_check(&src) {
            Ok(outcome) => {
                for note in &outcome.fusion_notes {
                    eprintln!("{}", optic_diagnostics::emit_human(note));
                }
                println!("check: OK ({})", path.display());
                if !outcome.fusion_notes.is_empty() {
                    println!(
                        "notes: {} fusion diagnostic(s) on stderr",
                        outcome.fusion_notes.len()
                    );
                }
                println!(
                    "next: opticc explain-grade {} --node <optic>",
                    path.display()
                );
                println!(
                    "next: opticc explain-focus {} --node <optic>",
                    path.display()
                );
            }
            Err(diags) => {
                for d in &diags {
                    eprintln!("{}", optic_diagnostics::emit_human(d));
                }
                if let Some(first) = diags.first() {
                    println!("suggest: opticc explain {}", first.code);
                    if let Some(fix) = first.ranked_fixes.first() {
                        println!("fix: {}", fix.description);
                    }
                    let optic_name = first
                        .evidence
                        .get("optic")
                        .or_else(|| first.evidence.get("name"))
                        .or_else(|| first.evidence.get("binding"))
                        .and_then(|v| v.as_str());
                    if let Some(name) = optic_name {
                        println!(
                            "suggest: opticc explain-grade {} --node {} --json",
                            path.display(),
                            name
                        );
                        println!(
                            "suggest: opticc explain-focus {} --node {} --json",
                            path.display(),
                            name
                        );
                    } else if first.code.starts_with("GRA-") || first.code.starts_with("TYP-") {
                        println!(
                            "suggest: opticc explain-grade {} --node <optic> --json",
                            path.display()
                        );
                        println!(
                            "suggest: opticc explain-focus {} --node <optic> --json",
                            path.display()
                        );
                    }
                }
                anyhow::bail!(
                    "doctor: check failed for {} ({} diagnostics)",
                    path.display(),
                    diags.len()
                );
            }
        }
    }
    println!("doctor: OK");
    Ok(())
}

fn explain_grade_cmd(src: &str, node: &str, json: bool) -> anyhow::Result<()> {
    validate_node_name(node)?;
    match explain_grade_from_src(src, node) {
        Ok(report) => {
            if json {
                let value = serde_json::to_value(&report).context("serialize grade report")?;
                println!("{}", optic_diagnostics::explain_grade_ok_json(&value));
            } else {
                println!("optic: {}", report.optic);
                let decl_alias = report.declared.ownership_alias.as_deref().unwrap_or("-");
                println!(
                    "declared: cache={} ({}) ownership={} alias={} read_only={} must_use={}",
                    report.declared.cache,
                    report.declared.cache_source,
                    report.declared.ownership_share,
                    decl_alias,
                    report.declared.read_only,
                    report.declared.must_use
                );
                println!(
                    "inferred: cache={} ({}) ownership={} read_only={} must_use={}",
                    report.inferred.cache,
                    report.inferred.cache_source,
                    report.inferred.ownership_share,
                    report.inferred.read_only,
                    report.inferred.must_use
                );
                println!("regions:");
                println!("  get_reads: {:?}", report.regions.get_reads);
                println!("  put_reads: {:?}", report.regions.put_reads);
                println!("  put_writes: {:?}", report.regions.put_writes);
            }
            Ok(())
        }
        Err(diags) => {
            if json {
                eprintln!("{}", optic_diagnostics::diagnostics_to_json(&diags));
                std::process::exit(1);
            } else {
                for d in &diags {
                    eprintln!("{}", optic_diagnostics::emit_human(d));
                }
                anyhow::bail!("explain-grade failed ({} diagnostics)", diags.len());
            }
        }
    }
}

fn explain_focus_cmd(src: &str, node: &str, json: bool) -> anyhow::Result<()> {
    validate_node_name(node)?;
    match explain_focus_from_src(src, node) {
        Ok(report) => {
            if json {
                let value = serde_json::to_value(&report).context("serialize focus report")?;
                println!("{}", optic_diagnostics::explain_focus_ok_json(&value));
            } else {
                println!("node: {}", report.node);
                println!("costate: {}", report.costate);
                println!("focus: {}", report.focus);
                println!("path_lift.prefix: {:?}", report.path_lift_prefix);
                println!("root_path: {}", report.root_path);
                if !report.focus_fields.is_empty() {
                    println!("focus_fields:");
                    for ff in &report.focus_fields {
                        println!("  {ff}");
                    }
                }
            }
            Ok(())
        }
        Err(diags) => {
            if json {
                eprintln!("{}", optic_diagnostics::diagnostics_to_json(&diags));
                std::process::exit(1);
            } else {
                for d in &diags {
                    eprintln!("{}", optic_diagnostics::emit_human(d));
                }
                anyhow::bail!("explain-focus failed ({} diagnostics)", diags.len());
            }
        }
    }
}

fn dump_summary(src: &str, node: Option<&str>) -> anyhow::Result<()> {
    if let Some(n) = node {
        if n.len() > optic_cgir::MAX_NODE_NAME_BYTES {
            anyhow::bail!(
                "node name exceeds {} bytes",
                optic_cgir::MAX_NODE_NAME_BYTES
            );
        }
        // Name lookup first so optic names like "42" are not mistaken for CGIR node ids.
        let hir = lower_src(src).map_err(|diags| emit_pipeline_errors(&diags))?;
        for item in &hir.items {
            match item {
                optic_hir::HirItem::Optic { decl, summary } if decl.name.node == n => {
                    println!("{}: {summary:#?}", decl.name.node);
                    return Ok(());
                }
                optic_hir::HirItem::Let { name, summary, .. } if name == n => {
                    println!("{name}: {summary:#?}");
                    return Ok(());
                }
                _ => {}
            }
        }
        if let Ok(id) = n.parse::<u32>() {
            let outcome = compile_cgir_or_exit(src, false)?;
            let graph = outcome.graph;
            if let Some(nd) = optic_cgir::find_node_by_id(&graph, id) {
                if let optic_cgir::CgirNode::OpticLeaf { summary, name, .. } = nd {
                    println!("summary for node {id} ({name}): {summary:#?}");
                } else {
                    println!("node {id}: {nd:#?}");
                }
                if let Some(p) = graph.provenance_index.get(&id) {
                    println!("provenance: {p:#?}");
                }
            } else {
                anyhow::bail!("node id {id} not found");
            }
            return Ok(());
        }
        let candidates = hir_binding_candidates(&hir);
        return Err(emit_pipeline_errors(&[
            optic_diagnostics::explain_unknown_node_diag(
                optic_syntax::Span::dummy(),
                n,
                &candidates,
            ),
        ]));
    }
    let hir = lower_src(src).map_err(|diags| emit_pipeline_errors(&diags))?;
    for item in &hir.items {
        match item {
            optic_hir::HirItem::Optic { decl, summary } => {
                println!("{}: {summary:#?}", decl.name.node);
            }
            optic_hir::HirItem::Let { name, summary, .. } => {
                println!("{name}: {summary:#?}");
            }
            optic_hir::HirItem::Extern(_) => {} // per ch22/appI/PLAN (passes as optics; S0 for S1; 3-ring)
            _ => {}
        }
    }
    Ok(())
}

fn bench_single_file(file: &Path, update: bool, verbose: bool) -> anyhow::Result<()> {
    let src = read_source(file)?;
    let ex = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("example.opt");
    let start = std::time::Instant::now();
    let emitted = compile_emit_or_exit(&src)?;
    let compile_ms = start.elapsed().as_millis().max(1);
    let run_start = std::time::Instant::now();
    run_verification_harness(&emitted, file, verbose)?;
    let run_ms = run_start.elapsed().as_millis().max(1);
    let line = format!("{ex}: compile_ms={compile_ms} run_ms={run_ms} ok=1\n");
    let bench_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bench");
    let baseline = bench_dir.join(ex.replace(".opt", ".txt"));
    if update {
        fs::create_dir_all(&bench_dir)?;
        fs::write(&baseline, &line)?;
        println!("updated {}", baseline.display());
    } else if baseline.exists() {
        let expected = fs::read_to_string(&baseline)?;
        if let (Some((ec, er)), Some((nc, nr))) =
            (parse_bench_line(&expected), parse_bench_line(&line))
        {
            let compile_limit = ec * BENCH_TOLERANCE_MULT;
            let run_limit = er * BENCH_TOLERANCE_MULT;
            if nc > compile_limit || nr > run_limit {
                anyhow::bail!(
                    "bench regression for {ex} (tolerance {BENCH_TOLERANCE_MULT}x): \
                     baseline compile_ms={ec} run_ms={er}, got compile_ms={nc} run_ms={nr}"
                );
            }
        }
        println!("{line}within tolerance ({BENCH_TOLERANCE_MULT}x baseline; compile/run ms)");
    } else {
        println!("{line}(no baseline; use --update)");
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
        "alive_filter.opt",
        "all_healths.opt",
        "tap_health.opt",
        "record_health.opt",
        "health_position.opt",
        "health_get.opt",
        "health_set.opt",
        "compose_decay.opt",
        "compose_triple.opt",
        "nested_position.opt",
        "nested_field_triple.opt",
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
        let emitted = compile_emit_or_exit(&src)?;
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
    use std::io::Read;
    let f = fs::File::open(file).with_context(|| format!("open {}", file.display()))?;
    let mut buf = Vec::new();
    f.take(MAX_SOURCE_BYTES.saturating_add(1))
        .read_to_end(&mut buf)
        .with_context(|| format!("read {}", file.display()))?;
    if buf.len() as u64 > MAX_SOURCE_BYTES {
        anyhow::bail!(
            "source {} exceeds {} byte limit",
            file.display(),
            MAX_SOURCE_BYTES
        );
    }
    String::from_utf8(buf).with_context(|| format!("utf8 decode {}", file.display()))
}

fn emit_pipeline_errors(diags: &[optic_diagnostics::Diagnostic]) -> anyhow::Error {
    for d in diags {
        eprintln!("{}", optic_diagnostics::emit_human(d));
    }
    anyhow::anyhow!("compile pipeline failed ({} diagnostics)", diags.len())
}

fn compile_cgir_or_exit(src: &str, before_fusion: bool) -> anyhow::Result<optic::CgirOutcome> {
    compile_cgir(src, before_fusion).map_err(|diags| emit_pipeline_errors(&diags))
}

fn compile_emit_or_exit(src: &str) -> anyhow::Result<String> {
    compile_emit(src).map_err(|diags| emit_pipeline_errors(&diags))
}

fn verify_example_stdout(filename: &str, stdout: &str) -> bool {
    match filename {
        "health_position.opt" => stdout.contains("99.0") && stdout.contains("0.1"),
        "compose_triple.opt" => {
            stdout.contains("98.333") && stdout.contains("78.333") && stdout.contains("48.333")
        }
        "compose_decay.opt" => {
            stdout.contains("95.0") && stdout.contains("75.0") && stdout.contains("45.0")
        }
        "health_decay.opt" | "alive_filter.opt" | "partial_prism.opt" | "all_healths.opt"
        | "tap_health.opt" | "record_health.opt" => {
            stdout.contains("90.0") && stdout.contains("70.0")
        }
        "health_set.opt" | "prism_set.opt" | "traversal_set.opt" => stdout.contains("42.0"),
        "health_get.opt" | "prism_get.opt" | "traversal_get.opt" => stdout.contains("get:"),
        "nested_position.opt" => {
            stdout.contains("(0.1, 0.1)")
                && stdout.contains("(1.1, 1.1)")
                && stdout.contains("(2.1, 2.1)")
        }
        "nested_field_triple.opt" => {
            stdout.contains("tag: 0.1") && stdout.contains("before:") && stdout.contains("after:")
        }
        _ => false,
    }
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

    let filename = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let verified = verify_example_stdout(filename, &stdout);

    if verified {
        println!(
            "RUN VERIFIED ({})",
            file.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| file.display().to_string())
        );
    } else {
        anyhow::bail!(
            "verification predicate did not match output for {}",
            file.display()
        );
    }
    Ok(())
}
