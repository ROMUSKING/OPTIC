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

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/diagnostics/{name}"))
}

#[test]
fn check_json_invalid_grade_matches_fixture() {
    let assert = opticc()
        .args([
            "check",
            "--json",
            &example("invalid_grade.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("\"ok\""));
    assert!(out.contains("GRA-110"));
    assert!(out.contains("evidence"));
    let path = fixture("invalid_grade.json");
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, &out).expect("write json golden");
    } else if path.exists() {
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(out.trim(), expected.trim());
    }
}

#[test]
fn check_json_invalid_alias_has_witness_fields() {
    let assert = opticc()
        .args([
            "check",
            "--json",
            &example("invalid_alias.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("ALI-201"));
    assert!(out.contains("conflicting_regions"));
    assert!(out.contains("ranked_fixes") || out.contains("minimal_fix_options"));
}
