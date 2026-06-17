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
