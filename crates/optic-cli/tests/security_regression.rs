//! Security regression tests (path limits, size caps, diagnostic hygiene).

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

fn opticc() -> Command {
    Command::cargo_bin("opticc").unwrap()
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn write_temp_opt(src: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("inject.opt");
    std::fs::write(&path, src).expect("write temp opt");
    (dir, path)
}

#[test]
fn rejects_tap_multiline_string_injection() {
    let src = r#"
data Entities { healths: SoA<f32> }
optic H: GradedOptic<Entities, f32, _> {
    get s => s.healths[s.id]
    put (s,v) => { s.healths[s.id] = v }
}
fn main() {
    entities.query(H).tap("x
include!(\"pwn\")").map(|h| h);
}
"#;
    let (_dir, path) = write_temp_opt(src);
    opticc()
        .args(["check", "--json", &path.to_string_lossy()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("PAR-001").or(predicates::str::contains("control")));
}

#[test]
fn rejects_tap_include_escape_in_label() {
    let src = r#"
data Entities { healths: SoA<f32> }
optic H: GradedOptic<Entities, f32, _> {
    get s => s.healths[s.id]
    put (s,v) => { s.healths[s.id] = v }
}
fn main() {
    entities.query(H).tap("a\ninclude!(\"x\")").map(|h| h);
}
"#;
    let (_dir, path) = write_temp_opt(src);
    let assert = opticc()
        .args(["check", &path.to_string_lossy()])
        .assert()
        .failure();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&assert.get_output().stderr),
        String::from_utf8_lossy(&assert.get_output().stdout)
    );
    assert!(
        combined.contains("PAR-001")
            || combined.contains("control")
            || combined.contains("invalid"),
        "must reject injection attempt: {combined}"
    );
}

#[test]
fn rejects_transpile_output_path_with_parent_dir() {
    let out = PathBuf::from("nested/../escape.rs");
    opticc()
        .args([
            "transpile",
            &example("health_decay.opt").to_string_lossy(),
            "--out",
            &out.to_string_lossy(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("must not contain '..'"));
}

#[test]
fn rejects_oversized_source_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("huge.opt");
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create huge.opt");
    const LIMIT: u64 = 4 * 1024 * 1024;
    f.seek(SeekFrom::Start(LIMIT + 1)).expect("seek");
    f.write_all(&[b' ']).expect("write byte");
    f.flush().expect("flush");
    opticc()
        .args(["check", &path.to_string_lossy()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("byte limit"));
}

#[test]
fn parse_error_json_includes_ranked_fixes() {
    let assert = opticc()
        .args([
            "check",
            "--json",
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/parse_error.opt")
                .to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    let v: serde_json::Value = serde_json::from_str(out.trim()).expect("json");
    let fixes = v["diagnostics"][0]["ranked_fixes"]
        .as_array()
        .expect("ranked_fixes");
    assert!(!fixes.is_empty());
    assert!(fixes[0]["description"].as_str().is_some());
}

#[test]
fn doctor_validates_runtime_crate_path() {
    opticc()
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("optic-runtime: OK"));
}

#[test]
fn rejects_deeply_nested_parens_without_stack_overflow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("deep.opt");
    let depth = 700usize;
    let mut src = String::from("fn main() { entities.query(H).map(|x| ");
    src.push_str(&"(".repeat(depth));
    src.push('x');
    src.push_str(&")".repeat(depth));
    src.push_str("); }");
    std::fs::write(&path, &src).expect("write deep.opt");
    opticc()
        .args(["check", "--json", &path.to_string_lossy()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("PAR-001"));
}

#[test]
fn rejects_deeply_nested_soa_type_without_stack_overflow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("deep_soa.opt");
    let depth = 700usize;
    let mut inner = String::from("f32");
    for _ in 0..depth {
        inner = format!("SoA<{inner}>");
    }
    let src = format!("data Entities {{ healths: {inner} }}\n");
    std::fs::write(&path, &src).expect("write deep_soa.opt");
    opticc()
        .args(["check", "--json", &path.to_string_lossy()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("PAR-001"));
}

#[test]
fn rejects_deep_optic_compose_chain_without_stack_overflow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("deep_compose.opt");
    let depth = 700usize;
    let mut src = String::from(
        "data Entities { healths: SoA<f32> }\n\
         optic A: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }\n\
         optic B: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }\n\
         let deep = ",
    );
    src.push_str(&format!("{}A", "B >>> ".repeat(depth)));
    src.push_str(";\nfn main() { entities.query(deep).map(|x| x); }\n");
    std::fs::write(&path, &src).expect("write deep_compose.opt");
    opticc()
        .args(["check", "--json", &path.to_string_lossy()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("compose depth limit exceeded"));
}

#[test]
fn verbose_flag_accepted_on_run() {
    opticc()
        .args([
            "run",
            "--verbose",
            &example("health_get.opt").to_string_lossy(),
        ])
        .assert()
        .success();
}
