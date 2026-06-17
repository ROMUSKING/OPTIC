use assert_cmd::Command;
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
fn check_positive_decay() {
    opticc()
        .args(["check", &example("health_decay.opt").to_string_lossy()])
        .assert()
        .success();
}

#[test]
fn check_negative_alias() {
    let assert = opticc()
        .args(["check", &example("invalid_alias.opt").to_string_lossy()])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("ALI-201"));
    assert!(out.contains("conflicting_regions"));
}

#[test]
fn check_negative_grade_decl() {
    let assert = opticc()
        .args(["check", &example("invalid_grade.opt").to_string_lossy()])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("GRA-110"));
}

#[test]
fn check_negative_grade_compose() {
    let assert = opticc()
        .args(["check", &example("grade_mismatch.opt").to_string_lossy()])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("GRA-104"));
}

#[test]
fn check_json_failure_emits_json() {
    let assert = opticc()
        .args([
            "check",
            "--json",
            &example("invalid_alias.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("\"ok\": false") || out.contains("\"ok\":false"));
    assert!(out.contains("ALI-201"));
}

#[test]
fn dump_cgir_verify_decay() {
    opticc()
        .args([
            "dump-cgir",
            &example("health_decay.opt").to_string_lossy(),
            "--check",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("QueryMap"));
}
