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
fn run_health_decay_mutates() {
    let assert = opticc()
        .args(["run", &example("health_decay.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(out.contains("90.0"));
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_health_position_mutates_both_columns() {
    let assert = opticc()
        .args(["run", &example("health_position.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(out.contains("99.0"));
    assert!(out.contains("0.1"));
    assert!(out.contains("RUN VERIFIED"));
}
