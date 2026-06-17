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
    fn round_floats(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    *v = serde_json::json!((f * 10.0).round() / 10.0);
                }
            }
            serde_json::Value::Array(a) => a.iter_mut().for_each(round_floats),
            serde_json::Value::Object(o) => o.values_mut().for_each(round_floats),
            _ => {}
        }
    }
    let mut copy = v;
    round_floats(&mut copy);
    serde_json::to_string_pretty(&copy).expect("serialize normalized json")
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
    } else if path.exists() {
        let expected = std::fs::read_to_string(&path).expect("read json golden");
        assert_eq!(normalized, normalize_json_floats(expected.trim()));
    }
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
