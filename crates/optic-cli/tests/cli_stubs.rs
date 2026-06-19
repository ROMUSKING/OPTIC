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
fn explain_fus501() {
    opticc()
        .args(["explain", "FUS-501"])
        .assert()
        .success()
        .stdout(predicates::str::contains("intermediate escapes"));
}

#[test]
fn explain_fus502() {
    opticc()
        .args(["explain", "FUS-502"])
        .assert()
        .success()
        .stdout(predicates::str::contains("legality precondition"));
}

#[test]
fn explain_cgi_codes() {
    for code in ["CGI-003", "CGI-004", "CGI-005", "RES-001"] {
        opticc()
            .args(["explain", code])
            .assert()
            .success()
            .stdout(predicates::str::contains(code));
    }
}

#[test]
fn explain_typ_codes() {
    for code in ["TYP-001", "TYP-002", "TYP-003", "TYP-004", "TYP-010"] {
        opticc()
            .args(["explain", code])
            .assert()
            .success()
            .stdout(predicates::str::contains(code));
    }
}

#[test]
fn explain_grade_healthview() {
    opticc()
        .args([
            "explain-grade",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "HealthView",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("inferred: cache=2"));
}

#[test]
fn explain_grade_json_healthview() {
    opticc()
        .args([
            "explain-grade",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "HealthView",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"ok\": true"))
        .stdout(predicates::str::contains("\"optic\": \"HealthView\""));
}

#[test]
fn explain_grade_invalid_grade_file_shows_inferred() {
    opticc()
        .args([
            "explain-grade",
            &example("invalid_grade.opt").to_string_lossy(),
            "--node",
            "BadCache",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("inferred: cache=3"));
}

#[test]
fn explain_grade_unknown_node_fails_exp001() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "MissingOptic",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("EXP-001"));
}

#[test]
fn explain_grade_json_unknown_node_emits_json() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "MissingOptic",
            "--json",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("\"ok\": false"));
    assert!(stderr.contains("EXP-001"));
    assert!(stderr.contains("candidates"));
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/diagnostics/{name}"))
}

#[test]
fn explain_grade_json_badcache_matches_golden() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("invalid_grade.opt").to_string_lossy(),
            "--node",
            "BadCache",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("parse json");
    assert_eq!(v["ok"], true);
    assert_eq!(v["grade"]["optic"], "BadCache");
    let path = fixture("explain_grade_badcache.json");
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{}\n", stdout.trim())).expect("write golden");
    } else {
        let expected = std::fs::read_to_string(&path).expect("read explain_grade badcache golden");
        let exp: serde_json::Value = serde_json::from_str(expected.trim()).expect("parse golden");
        assert_eq!(v["grade"], exp["grade"]);
    }
}

#[test]
fn explain_grade_json_nested_let_matches_golden() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("nested_position.opt").to_string_lossy(),
            "--node",
            "nested",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("parse json");
    assert_eq!(v["ok"], true);
    assert_eq!(v["grade"]["optic"], "nested");
    let path = fixture("explain_grade_nested.json");
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{}\n", stdout.trim())).expect("write golden");
    } else {
        let expected = std::fs::read_to_string(&path).expect("read explain_grade nested golden");
        let exp: serde_json::Value = serde_json::from_str(expected.trim()).expect("parse golden");
        assert_eq!(v["grade"], exp["grade"]);
    }
}

#[test]
fn explain_grade_json_healthview_matches_golden() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "HealthView",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect("parse json");
    assert_eq!(v["ok"], true);
    assert_eq!(v["grade"]["optic"], "HealthView");
    let path = fixture("explain_grade_healthview.json");
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{}\n", stdout.trim())).expect("write golden");
    } else {
        let expected = std::fs::read_to_string(&path).expect("read explain_grade golden");
        let exp: serde_json::Value = serde_json::from_str(expected.trim()).expect("parse golden");
        assert_eq!(v["grade"], exp["grade"]);
    }
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
