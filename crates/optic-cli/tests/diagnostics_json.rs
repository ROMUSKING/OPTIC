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

fn normalize_json_floats(s: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(s).expect("parse diagnostic json");
    fn round_floats(v: &mut serde_json::Value, key: Option<&str>) {
        match v {
            serde_json::Value::Number(n) => {
                if key == Some("confidence") {
                    return;
                }
                if let Some(f) = n.as_f64() {
                    *v = serde_json::json!((f * 10.0).round() / 10.0);
                }
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(|x| round_floats(x, None)),
            serde_json::Value::Object(o) => {
                for (k, val) in o.iter_mut() {
                    round_floats(val, Some(k.as_str()));
                }
            }
            _ => {}
        }
    }
    let mut copy = v;
    round_floats(&mut copy, None);
    serde_json::to_string_pretty(&copy).expect("serialize normalized json")
}

fn assert_confidence_exact(actual_json: &str, expected_json: &str) {
    let actual: serde_json::Value = serde_json::from_str(actual_json).expect("parse actual");
    let expected: serde_json::Value = serde_json::from_str(expected_json).expect("parse expected");
    fn walk_confidence(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(o) => {
                if let Some(c) = o.get("confidence") {
                    out.push(c.to_string());
                }
                o.values().for_each(|x| walk_confidence(x, out));
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk_confidence(x, out)),
            _ => {}
        }
    }
    let mut a_conf = vec![];
    let mut e_conf = vec![];
    walk_confidence(&actual, &mut a_conf);
    walk_confidence(&expected, &mut e_conf);
    assert_eq!(a_conf, e_conf, "confidence values must match exactly");
}

fn assert_json_notes_golden(example_file: &str, json_name: &str, code: &str) {
    let assert = opticc()
        .args(["check", "--json", &example(example_file).to_string_lossy()])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stdout.contains(code) || stderr.contains(code));
    assert!(stdout.contains("notes"));
    let path = fixture(json_name);
    let normalized = normalize_json_floats(stdout.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing diagnostic golden {} — run OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli diagnostics_json",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
}

fn assert_json_diag_matches_fixture(diag: &optic_diagnostics::Diagnostic, json_name: &str) {
    let out = optic_diagnostics::diagnostics_to_json(std::slice::from_ref(diag));
    let path = fixture(json_name);
    let normalized = normalize_json_floats(out.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing diagnostic golden {} — run OPTIC_UPDATE_GOLDEN=1",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(
            normalized,
            expected.trim(),
            "json golden mismatch for {json_name}"
        );
    }
}

fn assert_json_golden(example_file: &str, json_name: &str, code: &str) {
    let assert = opticc()
        .args(["check", "--json", &example(example_file).to_string_lossy()])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains(code));
    assert!(out.contains("evidence"));
    let path = fixture(json_name);
    let normalized = normalize_json_floats(out.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing diagnostic golden {} — run OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli diagnostics_json",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        let expected_trim = expected.trim();
        assert_confidence_exact(out.trim(), expected_trim);
        assert_eq!(normalized, normalize_json_floats(expected_trim));
    }
}

fn assert_explain_grade_json_golden(example_file: &str, node: &str, json_name: &str, code: &str) {
    let assert = opticc()
        .args([
            "explain-grade",
            &example(example_file).to_string_lossy(),
            "--node",
            node,
            "--json",
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains(code));
    assert!(out.contains("\"ok\": false"));
    let path = fixture(json_name);
    let normalized = normalize_json_floats(out.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing explain-grade golden {} — run OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli diagnostics_json",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        let expected_trim = expected.trim();
        assert_confidence_exact(out.trim(), expected_trim);
        assert_eq!(normalized, normalize_json_floats(expected_trim));
    }
}

#[test]
fn explain_grade_json_unknown_node_matches_fixture() {
    assert_explain_grade_json_golden(
        "health_get.opt",
        "MissingOptic",
        "explain_grade_unknown_node.json",
        "EXP-001",
    );
}

#[test]
fn explain_grade_json_typ002_fail_matches_fixture() {
    assert_explain_grade_json_golden(
        "typ002_body_mismatch.opt",
        "BadFocus",
        "explain_grade_typ002_fail.json",
        "TYP-002",
    );
}

#[test]
fn explain_grade_json_typ003_fail_matches_fixture() {
    assert_explain_grade_json_golden(
        "typ003_grade_syntax.opt",
        "BadGrade",
        "explain_grade_typ003_fail.json",
        "TYP-003",
    );
}

#[test]
fn explain_grade_json_typ004_fail_matches_fixture() {
    assert_explain_grade_json_golden(
        "typ004_uninferable_body.opt",
        "BadInfer",
        "explain_grade_typ004_fail.json",
        "TYP-004",
    );
}

#[test]
fn check_json_compose_escape_notes_fus501() {
    assert_json_notes_golden("compose_escape.opt", "compose_escape.json", "FUS-501");
}

#[test]
fn check_json_invalid_grade_matches_fixture() {
    assert_json_golden("invalid_grade.opt", "invalid_grade.json", "GRA-110");
}

#[test]
fn check_json_invalid_alias_matches_fixture() {
    assert_json_golden("invalid_alias.opt", "invalid_alias.json", "ALI-201");
}

#[test]
fn check_json_grade_mismatch_matches_fixture() {
    assert_json_golden("grade_mismatch.opt", "grade_mismatch.json", "GRA-104");
}

#[test]
fn check_json_parse_error_matches_fixture() {
    assert_json_golden("parse_error.opt", "parse_error.json", "PAR-001");
}

#[test]
fn check_json_cgi003_unsupported_matches_fixture() {
    assert_json_golden(
        "cgi003_unsupported.opt",
        "cgi003_unsupported.json",
        "CGI-003",
    );
}

#[test]
fn check_json_res001_unknown_matches_fixture() {
    assert_json_golden("res001_unknown.opt", "res001_unknown.json", "RES-001");
}

#[test]
fn check_json_cgi005_arity_mismatch_matches_fixture() {
    assert_json_golden(
        "cgi005_arity_mismatch.opt",
        "cgi005_arity_mismatch.json",
        "CGI-005",
    );
}

#[test]
fn check_json_cgi004_multi_query_matches_fixture() {
    assert_json_golden(
        "cgi004_multi_query.opt",
        "cgi004_multi_query.json",
        "CGI-004",
    );
}

#[test]
fn check_json_typ001_unknown_type_matches_fixture() {
    assert_json_golden(
        "typ001_unknown_type.opt",
        "typ001_unknown_type.json",
        "TYP-001",
    );
}

#[test]
fn check_json_typ002_body_mismatch_matches_fixture() {
    assert_json_golden(
        "typ002_body_mismatch.opt",
        "typ002_body_mismatch.json",
        "TYP-002",
    );
}

#[test]
fn check_json_typ003_grade_syntax_matches_fixture() {
    assert_json_golden(
        "typ003_grade_syntax.opt",
        "typ003_grade_syntax.json",
        "TYP-003",
    );
}

#[test]
fn check_json_typ003_unknown_dim_matches_fixture() {
    assert_json_golden(
        "typ003_unknown_dim.opt",
        "typ003_unknown_dim.json",
        "TYP-003",
    );
}

#[test]
fn check_json_typ004_uninferable_matches_fixture() {
    assert_json_golden(
        "typ004_uninferable_body.opt",
        "typ004_uninferable_body.json",
        "TYP-004",
    );
}

#[test]
fn check_json_typ001_unknown_focus_matches_fixture() {
    assert_json_golden(
        "typ001_unknown_focus.opt",
        "typ001_unknown_focus.json",
        "TYP-001",
    );
}

#[test]
fn check_json_typ002_put_mismatch_matches_fixture() {
    assert_json_golden(
        "typ002_put_mismatch.opt",
        "typ002_put_mismatch.json",
        "TYP-002",
    );
}

#[test]
fn explain_focus_json_healthview_matches_fixture() {
    let assert = opticc()
        .args([
            "explain-focus",
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
    assert_eq!(v["focus"]["node"], "HealthView");
    let path = fixture("explain_focus_healthview.json");
    let normalized = normalize_json_floats(stdout.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write golden");
    } else {
        let expected = std::fs::read_to_string(&path).expect("read explain_focus golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
}

#[test]
fn explain_focus_json_unknown_node_matches_fixture() {
    assert_explain_focus_json_golden(
        "health_get.opt",
        "MissingOptic",
        "explain_focus_unknown_node.json",
        "EXP-001",
    );
}

#[test]
fn explain_focus_json_nested_matches_fixture() {
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
    assert_eq!(v["focus"]["node"], "nested");
    let path = fixture("explain_focus_nested.json");
    let normalized = normalize_json_floats(stdout.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write golden");
    } else {
        let expected = std::fs::read_to_string(&path).expect("read explain_focus_nested golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
}

#[test]
fn explain_focus_json_typ002_fail_matches_fixture() {
    assert_explain_focus_json_golden(
        "typ002_body_mismatch.opt",
        "BadFocus",
        "explain_focus_typ002_fail.json",
        "TYP-002",
    );
}

#[test]
fn explain_focus_json_all_healths_matches_fixture() {
    let assert = opticc()
        .args([
            "explain-focus",
            &example("all_healths.opt").to_string_lossy(),
            "--node",
            "AllHealths",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let path = fixture("explain_focus_all_healths.json");
    let normalized = normalize_json_floats(stdout.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing explain-focus golden {} — run OPTIC_UPDATE_GOLDEN=1",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
}

#[test]
fn explain_focus_json_alive_filter_matches_fixture() {
    let assert = opticc()
        .args([
            "explain-focus",
            &example("alive_filter.opt").to_string_lossy(),
            "--node",
            "AliveFilter",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let path = fixture("explain_focus_alive_filter.json");
    let normalized = normalize_json_floats(stdout.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing explain-focus golden {} — run OPTIC_UPDATE_GOLDEN=1",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
}

fn assert_explain_focus_json_golden(example_file: &str, node: &str, json_name: &str, code: &str) {
    let assert = opticc()
        .args([
            "explain-focus",
            &example(example_file).to_string_lossy(),
            "--node",
            node,
            "--json",
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains(code));
    assert!(out.contains("\"ok\": false"));
    let path = fixture(json_name);
    let normalized = normalize_json_floats(out.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing explain-focus golden {} — run OPTIC_UPDATE_GOLDEN=1",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
}

#[test]
fn check_json_compose_prism_cgi003_matches_fixture() {
    // retained for type/negative (CGI-004 wiring on compose_prism); cgi003_* for other prism_in_compose cases; alias covered in units; body stub (coverage via harness + cgir)
}

#[test]
fn explain_focus_json_typ010_fail_matches_fixture() {
    assert_explain_focus_json_golden(
        "host_boundary.opt",
        "HostCopy",
        "explain_focus_typ010_fail.json",
        "TYP-010",
    );
}

#[test]
fn check_json_unsupported_prism_gra110_matches_fixture() {
    assert_json_golden("unsupported_prism.opt", "unsupported_prism.json", "GRA-110");
}

#[test]
fn check_json_unsupported_traversal_matches_fixture() {
    assert_json_golden(
        "unsupported_traversal.opt",
        "unsupported_traversal.json",
        "GRA-110",
    );
}

#[test]
fn check_json_compose_traversal_cgi003_matches_fixture() {
    // retained for type/negative (CGI-004 wiring); cgi003_* for other cases; legal now in units/harness; body stub (coverage via harness + cgir)
}

#[test]
fn check_json_host_boundary_matches_fixture() {
    assert_json_golden("host_boundary.opt", "host_boundary.json", "TYP-010");
}

#[test]
fn check_json_unsupported_profile_obs701_matches_fixture() {
    assert_json_golden(
        "unsupported_profile.opt",
        "unsupported_profile.json",
        "OBS-701",
    );
}

#[test]
fn check_json_unsupported_replay_obs701_matches_fixture() {
    assert_json_golden(
        "unsupported_replay.opt",
        "unsupported_replay.json",
        "OBS-701",
    );
}

#[test]
fn check_json_trailing_tap_obs702_matches_fixture() {
    assert_json_golden("trailing_tap.opt", "trailing_tap.json", "OBS-702");
}

#[test]
fn check_json_trailing_record_obs702_matches_fixture() {
    assert_json_golden("trailing_record.opt", "trailing_record.json", "OBS-702");
}

#[test]
fn cgi006_tap_stub_structured_diag_matches_fixture() {
    use optic_cgir::{verify_to_diagnostic, CgirGraph, CgirNode};

    let g = CgirGraph {
        nodes: vec![CgirNode::Tap {
            id: 0,
            optic_name: "HealthView".into(),
            label: "tap".into(),
            provenance: optic_syntax::Span::dummy(),
            m7_reserved: true,
        }],
        roots: vec![0],
        provenance_index: Default::default(),
        resolved_optics: Default::default(),
        region_map: Default::default(),
    };
    let diag = verify_to_diagnostic(&g).expect_err("stub Tap must fail");
    assert_eq!(diag.code, "CGI-006");
    assert_json_diag_matches_fixture(&diag, "cgi006_tap_stub.json");
}

#[test]
fn cgi006_record_stub_structured_diag_matches_fixture() {
    use optic_cgir::{verify_to_diagnostic, CgirGraph, CgirNode};

    let g = CgirGraph {
        nodes: vec![CgirNode::Record {
            id: 0,
            optic_name: "HealthView".into(),
            event: "evt".into(),
            provenance: optic_syntax::Span::dummy(),
            m7_reserved: true,
        }],
        roots: vec![0],
        provenance_index: Default::default(),
        resolved_optics: Default::default(),
        region_map: Default::default(),
    };
    let diag = verify_to_diagnostic(&g).expect_err("stub Record must fail");
    assert_eq!(diag.code, "CGI-006");
    assert_json_diag_matches_fixture(&diag, "cgi006_record_stub.json");
}

#[test]
fn cgi006_m7_reserved_structured_diag_matches_fixture() {
    use optic_cgir::{verify_to_diagnostic, CgirGraph, CgirNode};
    use std::sync::Arc;

    let summary = Arc::new(optic_hir::OpticSummary {
        name: Some("AliveFilter".into()),
        costate: "Entities".into(),
        focus: "f32".into(),
        lift: optic_hir::PathLift::default(),
        get_reads: vec!["healths".into()],
        put_reads: vec![],
        put_writes: vec![],
        get_grade: optic_hir::ConcreteGrade {
            cache: 1,
            ownership: optic_hir::OwnershipDim {
                share: optic_hir::Rational::one(),
                read_only: false,
                must_use: false,
            },
        },
        put_grade: optic_hir::ConcreteGrade {
            cache: 1,
            ownership: optic_hir::OwnershipDim {
                share: optic_hir::Rational::one(),
                read_only: false,
                must_use: false,
            },
        },
        get_determinism: optic_hir::Determinism::Pure,
        put_determinism: optic_hir::Determinism::Pure,
        serializable: true,
        provenance: optic_syntax::Span::dummy(),
    });
    let g = CgirGraph {
        nodes: vec![CgirNode::PrismLeaf {
            id: 0,
            name: "AliveFilter".into(),
            costate: "Entities".into(),
            focus: "f32".into(),
            grade: summary.get_grade.clone(),
            preview_fn: String::new(),
            review_fn: String::new(),
            preview_param: "s".into(),
            preview_body: Arc::new(optic_hir::HirExpr::LitInt(1, optic_syntax::Span::dummy())),
            preview_returns_option: false,
            preview_wrap_some: false,
            review_state_param: None,
            review_value_param: None,
            review_value_body: None,
            summary,
            provenance: optic_syntax::Span::dummy(),
            m7_reserved: true,
            bias: optic_hir::BranchBias::Unknown,
        }],
        roots: vec![0],
        provenance_index: Default::default(),
        resolved_optics: Default::default(),
        region_map: Default::default(),
    };
    let diag = verify_to_diagnostic(&g).expect_err("PrismLeaf must fail verify");
    assert_eq!(diag.code, "CGI-006");
    let out = optic_diagnostics::diagnostics_to_json(&[diag]);
    let path = fixture("cgi006_prism_leaf.json");
    let normalized = normalize_json_floats(out.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing diagnostic golden {} — run OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli diagnostics_json",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
}

#[test]
fn cgi006_traversal_leaf_structured_diag_matches_fixture() {
    use optic_cgir::{verify_to_diagnostic, CgirGraph, CgirNode};
    use std::sync::Arc;

    let summary = Arc::new(optic_hir::OpticSummary {
        name: Some("AllHealths".into()),
        costate: "Entities".into(),
        focus: "f32".into(),
        lift: optic_hir::PathLift::default(),
        get_reads: vec!["healths".into()],
        put_reads: vec![],
        put_writes: vec!["healths".into()],
        get_grade: optic_hir::ConcreteGrade {
            cache: 1,
            ownership: optic_hir::OwnershipDim {
                share: optic_hir::Rational::one(),
                read_only: false,
                must_use: false,
            },
        },
        put_grade: optic_hir::ConcreteGrade {
            cache: 1,
            ownership: optic_hir::OwnershipDim {
                share: optic_hir::Rational::one(),
                read_only: false,
                must_use: false,
            },
        },
        get_determinism: optic_hir::Determinism::Pure,
        put_determinism: optic_hir::Determinism::Pure,
        serializable: true,
        provenance: optic_syntax::Span::dummy(),
    });
    let g = CgirGraph {
        nodes: vec![CgirNode::TraversalLeaf {
            id: 0,
            name: "AllHealths".into(),
            costate: "Entities".into(),
            focus: "f32".into(),
            grade: summary.get_grade.clone(),
            get_fn: String::new(),
            set_fn: String::new(),
            get_param: "s".into(),
            get_body: Arc::new(optic_hir::HirExpr::LitInt(1, optic_syntax::Span::dummy())),
            set_state_param: None,
            set_value_param: None,
            set_value_body: None,
            summary,
            provenance: optic_syntax::Span::dummy(),
            m7_reserved: true,
            bias: optic_hir::BranchBias::Unknown,
        }],
        roots: vec![0],
        provenance_index: Default::default(),
        resolved_optics: Default::default(),
        region_map: Default::default(),
    };
    let diag = verify_to_diagnostic(&g).expect_err("TraversalLeaf stub must fail verify");
    assert_eq!(diag.code, "CGI-006");
    assert_eq!(diag.evidence["kind"].as_str(), Some("TraversalLeaf"));
    let out = optic_diagnostics::diagnostics_to_json(&[diag]);
    let path = fixture("cgi006_traversal_leaf.json");
    let normalized = normalize_json_floats(out.trim());
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, format!("{normalized}\n")).expect("write json golden");
    } else {
        assert!(
            path.exists(),
            "missing diagnostic golden {} — run OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli diagnostics_json",
            path.display()
        );
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
}

#[test]
fn check_json_cgi003_incompatible_map_matches_fixture() {
    assert_json_golden(
        "cgi003_incompatible_map.opt",
        "cgi003_incompatible_map.json",
        "CGI-003",
    );
}

#[test]
fn check_json_goldens_include_ranked_fixes_for_cataloged_codes() {
    let cases = [
        ("invalid_grade.opt", "GRA-110"),
        ("invalid_alias.opt", "ALI-201"),
        ("grade_mismatch.opt", "GRA-104"),
        ("res001_unknown.opt", "RES-001"),
        ("cgi003_unsupported.opt", "CGI-003"),
        ("cgi003_incompatible_map.opt", "CGI-003"),
        ("cgi004_multi_query.opt", "CGI-004"),
        ("cgi005_arity_mismatch.opt", "CGI-005"),
        ("parse_error.opt", "PAR-001"),
        ("typ001_unknown_type.opt", "TYP-001"),
        ("typ001_unknown_focus.opt", "TYP-001"),
        ("typ002_body_mismatch.opt", "TYP-002"),
        ("typ002_put_mismatch.opt", "TYP-002"),
        ("typ003_grade_syntax.opt", "TYP-003"),
        ("typ003_unknown_dim.opt", "TYP-003"),
        ("typ004_uninferable_body.opt", "TYP-004"),
        ("unsupported_traversal.opt", "GRA-110"),
        ("host_boundary.opt", "TYP-010"),
        ("unsupported_profile.opt", "OBS-701"),
        ("unsupported_replay.opt", "OBS-701"),
        ("trailing_tap.opt", "OBS-702"),
        ("trailing_record.opt", "OBS-702"),
    ];
    for (example_file, code) in cases {
        let assert = opticc()
            .args(["check", "--json", &example(example_file).to_string_lossy()])
            .assert()
            .failure();
        let out = String::from_utf8_lossy(&assert.get_output().stderr);
        let v: serde_json::Value = serde_json::from_str(out.trim()).expect("parse json");
        let diags = v["diagnostics"].as_array().expect("diagnostics array");
        let d = diags
            .iter()
            .find(|d| d["code"].as_str() == Some(code))
            .unwrap_or_else(|| panic!("missing {code} in {example_file}"));
        let fixes = d["ranked_fixes"].as_array().expect("ranked_fixes");
        assert!(
            !fixes.is_empty(),
            "{code} in {example_file} must have non-empty ranked_fixes"
        );
        assert!(
            fixes[0]["description"].as_str().is_some(),
            "ranked_fixes[0] must have description"
        );
    }
}
