use optic::compile_check;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/diagnostics/{name}"))
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

/// GRA-110: raise CacheGrade annotation per evidence annotated/inferred.
#[test]
fn agent_repair_gra110_evidence_drives_cache_patch() {
    let raw = std::fs::read_to_string(fixture("invalid_grade.json")).expect("read witness");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
    let d = v["diagnostics"]
        .as_array()
        .and_then(|a| a.iter().find(|d| d["code"].as_str() == Some("GRA-110")))
        .expect("GRA-110 witness");
    let annotated = d["evidence"]["annotated"].as_f64().expect("annotated") as u32;
    let inferred = d["evidence"]["inferred"].as_f64().expect("inferred") as u32;
    assert!(inferred > annotated);
    let src = std::fs::read_to_string(example("invalid_grade.opt")).expect("read source");
    let patched = src.replace(
        &format!("CacheGrade<{annotated}>"),
        &format!("CacheGrade<{inferred}>"),
    );
    assert_ne!(src, patched);
    assert!(compile_check(&patched).is_ok(), "patched source should pass check");
}

/// ALI-201: split parallel product into sequential compose per ranked_fix hint.
#[test]
fn agent_repair_ali201_evidence_drives_sequential_patch() {
    let raw = std::fs::read_to_string(fixture("invalid_alias.json")).expect("read witness");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
    let d = v["diagnostics"]
        .as_array()
        .and_then(|a| a.iter().find(|d| d["code"].as_str() == Some("ALI-201")))
        .expect("ALI-201 witness");
    let regions = d["evidence"]["conflicting_regions"]
        .as_array()
        .expect("conflicting_regions");
    assert!(!regions.is_empty());
    let top = d["ranked_fixes"][0]["description"]
        .as_str()
        .expect("ranked_fixes[0]");
    assert!(top.contains("sequential"), "top fix should suggest sequential passes");
    let src = std::fs::read_to_string(example("invalid_alias.opt")).expect("read source");
    let patched = src.replace("WriteHealth *** AlsoWriteHealth", "WriteHealth >>> AlsoWriteHealth");
    assert_ne!(src, patched);
    let err = compile_check(&patched).expect_err("sequential patch may surface other errors");
    assert!(
        !err.iter().any(|d| d.code == "ALI-201"),
        "sequential patch must clear ALI-201 (got: {:?})",
        err.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

/// Simulate applying ranked_fix evidence to a known source edit pattern; re-check passes.
#[test]
fn agent_repair_typ002_evidence_drives_source_patch() {
    let raw = std::fs::read_to_string(fixture("typ002_body_mismatch.json")).expect("read witness");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
    let d = v["diagnostics"]
        .as_array()
        .and_then(|a| a.iter().find(|d| d["code"].as_str() == Some("TYP-002")))
        .expect("TYP-002 witness");
    let expected = d["evidence"]["expected_type"]
        .as_str()
        .expect("expected_type");
    let actual = d["evidence"]["actual_type"].as_str().expect("actual_type");
    let optic = d["evidence"]["optic"].as_str().expect("optic");
    let src = std::fs::read_to_string(example("typ002_body_mismatch.opt")).expect("read source");
    assert!(src.contains(optic));
    assert!(src.contains(expected));
    let clause = d["evidence"]["clause"].as_str().expect("clause");
    assert_eq!(clause, "get");
    // Apply ranked_fix[1]: adjust get body to return declared focus type.
    let patched = src.replace("s.positions[s.id]", "s.healths[s.id]");
    assert_ne!(src, patched, "evidence must drive a concrete body edit");
    assert!(
        compile_check(&patched).is_ok(),
        "patched typ002 source should pass full check (expected={expected}, actual={actual})"
    );
}

/// Deterministic single-pass repair: TYP-002 ranked_fix[0] suggests focus/body alignment.
#[test]
fn agent_repair_typ002_ranked_fix_suggests_body_alignment() {
    let raw = std::fs::read_to_string(fixture("typ002_body_mismatch.json")).expect("read witness");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
    let diags = v["diagnostics"].as_array().expect("diagnostics");
    let d = diags
        .iter()
        .find(|d| d["code"].as_str() == Some("TYP-002"))
        .expect("TYP-002 witness");
    let fixes = d["ranked_fixes"].as_array().expect("ranked_fixes");
    let top = fixes[0]["description"]
        .as_str()
        .expect("ranked_fixes[0].description");
    assert!(
        top.contains("focus") || top.contains("body"),
        "TYP-002 top fix should target focus/body alignment, got: {top}"
    );
}