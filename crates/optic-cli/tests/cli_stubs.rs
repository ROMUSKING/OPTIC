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
fn explain_gra104() {
    opticc()
        .args(["explain", "GRA-104"])
        .assert()
        .success()
        .stdout(predicates::str::contains("sequential composition"));
}

#[test]
fn doctor_ok() {
    opticc()
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("doctor: OK"));
}

#[test]
fn dump_summary_lists_optics() {
    opticc()
        .args([
            "dump-summary",
            &example("health_decay.opt").to_string_lossy(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("HealthView"));
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
        .stdout(predicates::str::contains("CGIR verify: OK"));
}
