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

fn parse_entities_line(out: &str, label: &str) -> Vec<f32> {
    let line = out
        .lines()
        .find(|l| l.starts_with(label))
        .unwrap_or_else(|| panic!("missing {label} line in output"));
    let start = line.find('[').expect("healths array");
    let end = line.find(']').expect("healths array end");
    line[start + 1..end]
        .split(',')
        .map(|s| s.trim().parse().expect("f32"))
        .collect()
}

fn parse_positions_line(out: &str, label: &str) -> Vec<(f32, f32)> {
    let line = out
        .lines()
        .find(|l| l.contains(label) && l.contains("positions"))
        .unwrap_or_else(|| panic!("missing positions in {label}"));
    let mut pairs = vec![];
    for chunk in line.split('(').skip(1) {
        if let Some(end) = chunk.find(')') {
            let inner = &chunk[..end];
            let mut nums = inner
                .split(',')
                .map(|s| s.trim().parse::<f32>().expect("f32"));
            pairs.push((nums.next().expect("x"), nums.next().expect("y")));
        }
    }
    pairs
}

#[test]
fn run_alive_filter_prism_mutates() {
    let assert = opticc()
        .args(["run", &example("alive_filter.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_entities_line(&out, "before:");
    let after = parse_entities_line(&out, "after:");
    assert_eq!(before, vec![100.0, 80.0, 50.0]);
    assert_eq!(after, vec![90.0, 70.0, 40.0]);
    assert!(out.contains("RUN VERIFIED"));

    // Total-preview prism map must not use tautological `if let Some(x) = Some(...)`.
    let transpile = opticc()
        .args(["transpile", &example("alive_filter.opt").to_string_lossy()])
        .assert()
        .success();
    let rust_path = std::path::PathBuf::from("alive_filter.rs");
    let rust = std::fs::read_to_string(&rust_path).expect("read transpiled rust");
    let _ = transpile;
    let _ = std::fs::remove_file(&rust_path);
    assert!(
        !rust.contains("if let Some"),
        "total preview must not emit if-let Some guard"
    );
    assert!(
        !rust.contains("Some(cursor"),
        "total preview must not double-wrap Some(...)"
    );
    assert!(
        rust.contains("let _healths = cursor_0.arena.healths[cursor_0.id]"),
        "total preview should bind field read directly"
    );
}

#[test]
fn run_prism_get_prints_values() {
    let assert = opticc()
        .args(["run", &example("prism_get.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(parse_get_lines(&out), vec![100.0, 80.0, 50.0]);
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_prism_set_mutates() {
    let assert = opticc()
        .args(["run", &example("prism_set.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_entities_line(&out, "before:");
    let after = parse_entities_line(&out, "after:");
    assert_eq!(before, vec![100.0, 80.0, 50.0]);
    assert_eq!(after, vec![42.0, 42.0, 42.0]);
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_partial_prism_emits_if_let_some() {
    let assert = opticc()
        .args(["run", &example("partial_prism.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(parse_entities_line(&out, "before:"), vec![100.0, 80.0, 50.0]);
    assert_eq!(parse_entities_line(&out, "after:"), vec![90.0, 70.0, 40.0]);
    assert!(out.contains("RUN VERIFIED"));
    let transpile = opticc()
        .args(["transpile", &example("partial_prism.opt").to_string_lossy()])
        .assert()
        .success();
    let rust_path = std::path::PathBuf::from("partial_prism.rs");
    let rust = std::fs::read_to_string(&rust_path).expect("read transpiled rust");
    let _ = transpile;
    let _ = std::fs::remove_file(&rust_path);
    assert!(rust.contains("if let Some"), "partial preview must use if-let");
    assert!(rust.contains("Some(cursor"), "partial preview must wrap Some(...)");
}

#[test]
fn run_health_decay_mutates() {
    let assert = opticc()
        .args(["run", &example("health_decay.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_entities_line(&out, "before:");
    let after = parse_entities_line(&out, "after:");
    assert_eq!(before, vec![100.0, 80.0, 50.0]);
    assert_eq!(after, vec![90.0, 70.0, 40.0]);
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_health_position_mutates_both_columns() {
    let assert = opticc()
        .args(["run", &example("health_position.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![99.0, 79.0, 49.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(0.1, 0.0), (1.1, 1.0), (2.1, 2.0)]);
    assert!(out.contains("RUN VERIFIED"));
}

fn parse_get_lines(out: &str) -> Vec<f32> {
    out.lines()
        .filter_map(|l| l.strip_prefix("get: "))
        .map(|s| s.trim().parse().expect("get f32"))
        .collect()
}

#[test]
fn run_health_get_prints_values() {
    let assert = opticc()
        .args(["run", &example("health_get.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let gets = parse_get_lines(&out);
    assert_eq!(gets, vec![100.0, 80.0, 50.0]);
    let before = parse_entities_line(&out, "before:");
    let after = parse_entities_line(&out, "after:");
    assert_eq!(before, after, "get query must not mutate healths");
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_compose_triple_mutates() {
    let assert = opticc()
        .args(["run", &example("compose_triple.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_entities_line(&out, "before:");
    let after = parse_entities_line(&out, "after:");
    assert_eq!(before, vec![100.0, 80.0, 50.0]);
    assert!((after[0] - 98.333336).abs() < 0.001);
    assert!((after[1] - 78.333336).abs() < 0.001);
    assert!((after[2] - 48.333332).abs() < 0.001);
    assert!(out.contains("RUN VERIFIED"));
}

fn parse_transform_positions_line(out: &str, label: &str) -> Vec<(f32, f32)> {
    let line = out
        .lines()
        .find(|l| l.contains(label) && l.contains("transforms"))
        .unwrap_or_else(|| panic!("missing transforms in {label}"));
    let mut pairs = vec![];
    for chunk in line.split('(').skip(1) {
        if let Some(end) = chunk.find(')') {
            let inner = &chunk[..end];
            let mut nums = inner
                .split(',')
                .map(|s| s.trim().parse::<f32>().expect("f32"));
            pairs.push((nums.next().expect("x"), nums.next().expect("y")));
        }
    }
    pairs
}

fn parse_tag_values_line(out: &str, label: &str) -> Vec<f32> {
    let line = out
        .lines()
        .find(|l| l.contains(label) && l.contains("tag:"))
        .unwrap_or_else(|| panic!("missing tag values in {label}"));
    line.split("tag:")
        .skip(1)
        .filter_map(|chunk| {
            chunk
                .split(|c: char| !c.is_ascii_digit() && c != '.')
                .find(|s| !s.is_empty())
                .and_then(|s| s.parse::<f32>().ok())
        })
        .collect()
}

#[test]
fn run_nested_field_triple_mutates() {
    let assert = opticc()
        .args(["run", &example("nested_field_triple.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_tag_values_line(&out, "before:");
    let after = parse_tag_values_line(&out, "after:");
    assert_eq!(before, vec![0.0, 0.0, 0.0]);
    assert_eq!(after, vec![0.1, 0.1, 0.1]);
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_nested_position_mutates() {
    let assert = opticc()
        .args(["run", &example("nested_position.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_transform_positions_line(&out, "before:");
    let after = parse_transform_positions_line(&out, "after:");
    assert_eq!(before, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after, vec![(0.1, 0.1), (1.1, 1.1), (2.1, 2.1)]);
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_compose_decay_mutates() {
    let assert = opticc()
        .args(["run", &example("compose_decay.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_entities_line(&out, "before:");
    let after = parse_entities_line(&out, "after:");
    assert_eq!(before, vec![100.0, 80.0, 50.0]);
    assert_eq!(after, vec![95.0, 75.0, 45.0]);
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_health_set_mutates() {
    let assert = opticc()
        .args(["run", &example("health_set.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let after = parse_entities_line(&out, "after:");
    assert_eq!(after, vec![42.0, 42.0, 42.0]);
    assert!(out.contains("RUN VERIFIED"));
}
