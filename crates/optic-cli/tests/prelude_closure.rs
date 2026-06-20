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

fn assert_typ010_json(stderr: &str, feature: &str) {
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).expect("parse json");
    assert_eq!(v["ok"], false);
    let diags = v["diagnostics"].as_array().expect("diagnostics");
    let d = diags
        .iter()
        .find(|d| d["code"].as_str() == Some("TYP-010"))
        .unwrap_or_else(|| panic!("missing TYP-010 in {stderr}"));
    assert_eq!(d["evidence"]["feature"].as_str(), Some(feature));
    let fixes = d["ranked_fixes"].as_array().expect("ranked_fixes");
    assert!(!fixes.is_empty());
}

#[test]
fn check_unsupported_traversal_typ010() {
    let assert = opticc()
        .args([
            "check",
            "--json",
            &example("unsupported_traversal.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert_typ010_json(&stderr, "traversal");
}

#[test]
fn check_alive_filter_prism_ok() {
    opticc()
        .args(["check", &example("alive_filter.opt").to_string_lossy()])
        .assert()
        .success();
}

#[test]
fn check_host_boundary_typ010() {
    let assert = opticc()
        .args([
            "check",
            "--json",
            &example("host_boundary.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).expect("parse json");
    let diags = v["diagnostics"].as_array().expect("diagnostics");
    let typ010: Vec<_> = diags
        .iter()
        .filter(|d| d["code"].as_str() == Some("TYP-010"))
        .collect();
    assert_eq!(typ010.len(), 2);
    let features: std::collections::HashSet<_> = typ010
        .iter()
        .filter_map(|d| d["evidence"]["feature"].as_str())
        .collect();
    assert!(features.contains("foreign_decl"));
    assert!(features.contains("unsafe_optic"));
}

#[test]
fn dump_hir_alive_filter_prism() {
    opticc()
        .args(["dump-hir", &example("alive_filter.opt").to_string_lossy()])
        .assert()
        .success()
        .stdout(predicates::str::contains("AliveFilter"));
}

#[test]
fn dump_summary_alive_filter_by_name() {
    opticc()
        .args([
            "dump-summary",
            &example("alive_filter.opt").to_string_lossy(),
            "--node",
            "AliveFilter",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("AliveFilter"))
        .stdout(predicates::str::contains("lift"));
}

#[test]
fn explain_focus_nested_position() {
    opticc()
        .args([
            "explain-focus",
            &example("nested_position.opt").to_string_lossy(),
            "--node",
            "nested",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "root_path: entities.transforms[id].position",
        ))
        .stdout(predicates::str::contains(
            "path_lift.prefix: [\"position\"]",
        ));
}

#[test]
fn explain_focus_nested_position_json_golden() {
    let assert = opticc()
        .args([
            "explain-focus",
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
    assert_eq!(v["focus"]["root_path"], "entities.transforms[id].position");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/diagnostics/explain_focus_nested.json");
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{}\n", stdout.trim())).expect("write golden");
    } else {
        let expected = std::fs::read_to_string(&path).expect("read explain_focus_nested golden");
        assert_eq!(stdout.trim(), expected.trim());
    }
}

#[test]
fn explain_focus_typ002_blocks_on_target() {
    let assert = opticc()
        .args([
            "explain-focus",
            &example("typ002_body_mismatch.opt").to_string_lossy(),
            "--node",
            "BadFocus",
            "--json",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).expect("parse json");
    assert_eq!(v["ok"], false);
    let diags = v["diagnostics"].as_array().expect("diagnostics");
    assert!(diags.iter().any(|d| d["code"].as_str() == Some("TYP-002")));
}

#[test]
fn dump_summary_unknown_name_fails() {
    let assert = opticc()
        .args([
            "dump-summary",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "MissingOptic",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("EXP-001"));
    assert!(stderr.contains("MissingOptic"));
    assert!(stderr.contains("HealthView"));
    assert!(stderr.contains("candidates"));
}

#[test]
fn doctor_failed_check_suggests_fix_and_explain() {
    let assert = opticc()
        .args([
            "doctor",
            &example("typ002_body_mismatch.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("fix:"));
    assert!(stdout.contains("explain-grade"));
    assert!(stdout.contains("explain-focus"));
    assert!(stdout.contains("BadFocus"));
}

#[test]
fn explain_focus_json_healthview() {
    opticc()
        .args([
            "explain-focus",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "HealthView",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("\"ok\": true"))
        .stdout(predicates::str::contains("entities.healths[id]"));
}

#[test]
fn explain_focus_unknown_node_fails_exp001() {
    let assert = opticc()
        .args([
            "explain-focus",
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
fn dump_summary_by_optic_name() {
    opticc()
        .args([
            "dump-summary",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "HealthView",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("HealthView"))
        .stdout(predicates::str::contains("lift"));
}

#[test]
fn dump_summary_by_numeric_node_id() {
    opticc()
        .args([
            "dump-summary",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("summary for node 0"));
}

#[test]
fn explain_typ010_catalog() {
    let assert = opticc().args(["explain", "TYP-010"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("unsupported in narrow v0"));
    assert!(stdout.contains("GradedPrism"));
    assert!(stdout.contains("GradedTraversal"));
    assert!(stdout.contains("alive_filter.opt"));
    assert!(stdout.contains("unsupported_traversal.opt"));
    assert!(stdout.contains("host_boundary.opt"));
    assert!(stdout.contains("docs/observability-v0.md"));
    assert!(stdout.contains("docs/effect-coeffect-v0.md"));
    assert!(stdout.contains("PrismLeaf"));
    assert!(stdout.contains("TraversalLeaf"));
    assert!(stdout.contains("CGI-006"));
    assert!(stdout.contains("unsafe optic"));
    assert!(stdout.contains("extern"));
    assert!(stdout.contains("docs/v0-executable-spec.md"));
    assert!(stdout.contains("explain CGI-006"));
}

#[test]
fn dump_summary_unknown_numeric_id_fails() {
    let assert = opticc()
        .args([
            "dump-summary",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "99999",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("node id 99999 not found"));
    assert!(!stderr.contains("EXP-001"));
}

#[test]
fn dump_cgir_alive_filter_prism_leaf() {
    opticc()
        .args([
            "dump-cgir",
            &example("alive_filter.opt").to_string_lossy(),
            "--node",
            "AliveFilter",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("PrismLeaf(AliveFilter"))
        .stdout(predicates::str::contains("m7_reserved=false"));
}

#[test]
fn dump_cgir_by_optic_name() {
    opticc()
        .args([
            "dump-cgir",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "HealthView",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("node id="))
        .stdout(predicates::str::contains("OpticLeaf(HealthView)"))
        .stdout(predicates::str::contains("summary(HealthView)"));
}

#[test]
fn explain_grade_rejects_oversized_node_name() {
    let long = "x".repeat(4097);
    let assert = opticc()
        .args([
            "explain-grade",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            &long,
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("4096"));
}

#[test]
fn dump_cgir_unknown_name_fails_exp001() {
    let assert = opticc()
        .args([
            "dump-cgir",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "MissingOptic",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("EXP-001"));
    assert!(stderr.contains("MissingOptic"));
    assert!(stderr.contains("HealthView"));
    assert!(stderr.contains("candidates"));
}

#[test]
fn dump_cgir_by_numeric_node_id() {
    opticc()
        .args([
            "dump-cgir",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "0",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("OpticLeaf"))
        .stdout(predicates::str::contains("HealthView"));
}

#[test]
fn dump_cgir_unknown_numeric_id_fails() {
    let assert = opticc()
        .args([
            "dump-cgir",
            &example("health_get.opt").to_string_lossy(),
            "--node",
            "99999",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("node id 99999 not found"));
    assert!(!stderr.contains("EXP-001"));
}

#[test]
fn doctor_with_file_check_ok() {
    opticc()
        .args(["doctor", &example("health_get.opt").to_string_lossy()])
        .assert()
        .success()
        .stdout(predicates::str::contains("doctor: OK"))
        .stdout(predicates::str::contains("check: OK"))
        .stdout(predicates::str::contains("explain-grade"))
        .stdout(predicates::str::contains("explain-focus"));
}

#[test]
fn doctor_without_file_ok() {
    opticc()
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicates::str::contains("doctor: OK"));
}

#[test]
fn bench_single_file_health_get() {
    opticc()
        .args(["bench", &example("health_get.opt").to_string_lossy()])
        .assert()
        .success();
}
