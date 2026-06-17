//! Security regression tests (path limits, size caps, diagnostic hygiene).

use assert_cmd::Command;
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
