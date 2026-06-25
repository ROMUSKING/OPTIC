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
    let start = match line.find('[') {
        Some(s) => s,
        None => return vec![], // empty-col / N=0 robustness (matches parse_tuples_for_column)
    };
    let end = match line.find(']') {
        Some(e) => e,
        None => return vec![],
    };
    if end <= start {
        return vec![]; // malformed (e.g. ] before [) guard
    }
    let seg = &line[start + 1..end];
    if seg.trim().is_empty() {
        return vec![];
    }
    seg.split(',')
        .filter_map(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.parse::<f32>().unwrap_or_else(|_| panic!("f32")))
            }
        })
        .collect()
}

fn parse_positions_line(out: &str, label: &str) -> Vec<(f32, f32)> {
    parse_tuples_for_column(out, label, "positions")
}

fn parse_velocities_line(out: &str, label: &str) -> Vec<(f32, f32)> {
    parse_tuples_for_column(out, label, "velocities")
}

/// Common robust extractor for Vec2 columns from debug {:?} lines in before:/after:.
/// Handles multiple tuple columns (e.g. positions + velocities). Panics on missing per harness style.
fn parse_tuples_for_column(out: &str, label: &str, col: &str) -> Vec<(f32, f32)> {
    let line = out
        .lines()
        .find(|l| l.contains(label) && l.contains(col))
        .unwrap_or_else(|| panic!("missing {col} in {label}"));
    let key = format!("{}:", col);
    let start = if let Some(k) = line.find(&key) {
        if let Some(b) = line[k..].find('[') {
            k + b + 1
        } else {
            // key present (line matched col) but no '[' after it: empty-col case (e.g. "col: []" edge or non-array).
            // Do NOT fallback to 0 (would grab data from earlier columns' ( tuples). Treat as empty.
            return vec![];
        }
    } else {
        0
    };
    let rest = &line[start..];
    let end = rest.find(']').unwrap_or(rest.len());
    let seg = &rest[..end];
    let mut pairs = vec![];
    for chunk in seg.split('(').skip(1) {
        if let Some(e) = chunk.find(')') {
            let inner = &chunk[..e];
            let mut nums = inner.split(',').map(|s| match s.trim().parse::<f32>() {
                Ok(v) => v,
                Err(_) => panic!("f32"),
            });
            let x = match nums.next() {
                Some(v) => v,
                None => panic!("x"),
            };
            let y = match nums.next() {
                Some(v) => v,
                None => panic!("y"),
            };
            pairs.push((x, y));
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
    assert_eq!(
        parse_entities_line(&out, "before:"),
        vec![100.0, 80.0, 50.0]
    );
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
    assert!(
        rust.contains("if let Some"),
        "partial preview must use if-let"
    );
    assert!(
        rust.contains("Some(cursor"),
        "partial preview must wrap Some(...)"
    );
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
fn run_tap_health_mutates_with_observability_hook() {
    let assert = opticc()
        .args(["run", &example("tap_health.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let after = parse_entities_line(&out, "after:");
    assert_eq!(after, vec![90.0, 70.0, 40.0]);
    assert!(out.contains("RUN VERIFIED"));

    let transpile = opticc()
        .args(["transpile", &example("tap_health.opt").to_string_lossy()])
        .assert()
        .success();
    let rust_path = std::path::PathBuf::from("tap_health.rs");
    let rust = std::fs::read_to_string(&rust_path).expect("read transpiled rust");
    let _ = transpile;
    let _ = std::fs::remove_file(&rust_path);
    assert!(rust.contains("// optic(tap): health_probe"));
}

#[test]
fn run_record_health_mutates_with_observability_hook() {
    let assert = opticc()
        .args(["run", &example("record_health.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let after = parse_entities_line(&out, "after:");
    assert_eq!(after, vec![90.0, 70.0, 40.0]);
    assert!(out.contains("RUN VERIFIED"));

    let transpile = opticc()
        .args(["transpile", &example("record_health.opt").to_string_lossy()])
        .assert()
        .success();
    let rust_path = std::path::PathBuf::from("record_health.rs");
    let rust = std::fs::read_to_string(&rust_path).expect("read transpiled rust");
    let _ = transpile;
    let _ = std::fs::remove_file(&rust_path);
    assert!(rust.contains("// optic(record): health_decay"));
}

#[test]
fn run_traversal_get_prints_values() {
    let assert = opticc()
        .args(["run", &example("traversal_get.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert_eq!(parse_get_lines(&out), vec![100.0, 80.0, 50.0]);
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_traversal_set_mutates() {
    let assert = opticc()
        .args(["run", &example("traversal_set.opt").to_string_lossy()])
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
fn run_all_healths_traversal_mutates() {
    let assert = opticc()
        .args(["run", &example("all_healths.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_entities_line(&out, "before:");
    let after = parse_entities_line(&out, "after:");
    assert_eq!(before, vec![100.0, 80.0, 50.0]);
    assert_eq!(after, vec![90.0, 70.0, 40.0]);
    assert!(out.contains("RUN VERIFIED"));
    let transpile = opticc()
        .args(["transpile", &example("all_healths.opt").to_string_lossy()])
        .assert()
        .success();
    let rust_path = std::path::PathBuf::from("all_healths.rs");
    let rust = std::fs::read_to_string(&rust_path).expect("read transpiled rust");
    let _ = transpile;
    let _ = std::fs::remove_file(&rust_path);
    assert!(rust.contains("// optic(traversal): AllHealths"));
    assert!(rust.contains("// simd-eligible"));
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
            let mut nums = inner.split(',').map(|s| match s.trim().parse::<f32>() {
                Ok(v) => v,
                Err(_) => panic!("f32"),
            });
            let x = match nums.next() {
                Some(v) => v,
                None => panic!("x"),
            };
            let y = match nums.next() {
                Some(v) => v,
                None => panic!("y"),
            };
            pairs.push((x, y));
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

#[test]
fn run_game_entity_sim_fused_product_and_tap() {
    let assert = opticc()
        .args(["run", &example("game_entity_sim.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![95.0, 75.0, 45.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    // pos + const via product with health: after pos (1,1)(2,2)(3,3)
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
    let before_v = parse_velocities_line(&out, "before:");
    let after_v = parse_velocities_line(&out, "after:");
    // vel column present (richer data) but not touched by this query
    assert_eq!(before_v, after_v);
    assert!(out.contains("RUN VERIFIED"));
    // tap hook in prefix (comment in emitted Rust, not runtime stdout)

    // emitted shape coverage for product fusion (edge for new complex)
    let transpile = opticc()
        .args([
            "transpile",
            &example("game_entity_sim.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("game_entity_sim.rs").expect("read transpiled");
    let _ = transpile;
    let _ = std::fs::remove_file("game_entity_sim.rs");
    assert!(
        rust.contains("// optic(fused): ["),
        "product should emit fused provenance comment"
    );
}

#[test]
fn run_prism_via_let_mixed_grade_decls() {
    let assert = opticc()
        .args([
            "run",
            &example("mixed_prism_traversal.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before = parse_entities_line(&out, "before:");
    let after = parse_entities_line(&out, "after:");
    assert_eq!(before, vec![100.0, 80.0, 50.0]);
    assert_eq!(after, vec![90.0, 70.0, 40.0]);
    // prism via let (with mixed-grade decls/lets in source); single active query
    assert!(out.contains("RUN VERIFIED"));
}

#[test]
fn run_reusable_and_taps_multi_let_hooks() {
    let assert = opticc()
        .args(["run", &example("reusable_and_taps.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![95.0, 75.0, 45.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(0.1, 0.0), (1.1, 1.0), (2.1, 2.0)]);
    assert!(out.contains("RUN VERIFIED"));
    // hooks prefix on product query, multiple lets exercised
    assert!(out.contains("before:") && out.contains("after:"));

    // marker evidence (expanded this delta)
    let src = std::fs::read_to_string(example("reusable_and_taps.opt")).expect("read src"); // test-only
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_rich_entity_update_multi_column_product() {
    let assert = opticc()
        .args(["run", &example("rich_entity_update.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    // health untouched (richer data decl exercised)
    assert_eq!(before_h, after_h);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    // product p + v ; v=(0,0)(1,1)(2,2) => pos after (0,0)(2,2)(4,4) ; sentinel (4.0,4.0) distinguishes post-arith
    assert_eq!(after_p, vec![(0.0, 0.0), (2.0, 2.0), (4.0, 4.0)]);
    assert!(out.contains("RUN VERIFIED"));

    // emitted shape for product map (tuple return + stores)
    let transpile = opticc()
        .args([
            "transpile",
            &example("rich_entity_update.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("rich_entity_update.rs").expect("read");
    let _ = transpile;
    let _ = std::fs::remove_file("rich_entity_update.rs");
    assert!(rust.contains("// optic(fused): ["));
    // marker evidence for canonical runtime example (added this delta)
    let src = std::fs::read_to_string(example("rich_entity_update.opt")).expect("read src");
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_triple_product_fusion_3way_product() {
    let assert = opticc()
        .args([
            "run",
            &example("triple_product_fusion.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![95.0, 75.0, 45.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
    let before_v = parse_velocities_line(&out, "before:");
    let after_v = parse_velocities_line(&out, "after:");
    // vel untouched (exercises product arity without mutating all columns)
    assert_eq!(before_v, after_v);
    assert!(out.contains("RUN VERIFIED"));

    // emitted shape coverage for 3-arity product
    let transpile = opticc()
        .args([
            "transpile",
            &example("triple_product_fusion.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("triple_product_fusion.rs").expect("read transpiled");
    let _ = transpile;
    let _ = std::fs::remove_file("triple_product_fusion.rs");
    assert!(
        rust.contains("// optic(fused): ["),
        "product should emit fused provenance comment"
    );
    // marker evidence (added to 2 more .opt this delta per PLAN)
    let src = std::fs::read_to_string(example("triple_product_fusion.opt")).expect("read src");
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_let_reuse_pipeline_binary_product() {
    let assert = opticc()
        .args(["run", &example("let_reuse_pipeline.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![90.0, 70.0, 40.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(0.5, 0.5), (1.5, 1.5), (2.5, 2.5)]);
    assert!(out.contains("RUN VERIFIED"));
    // let reuse + product exercised
    assert!(out.contains("before:") && out.contains("after:"));

    // emitted shape (uniform with siblings)
    let transpile = opticc()
        .args([
            "transpile",
            &example("let_reuse_pipeline.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("let_reuse_pipeline.rs").expect("read");
    let _ = transpile;
    let _ = std::fs::remove_file("let_reuse_pipeline.rs");
    assert!(rust.contains("// optic(fused): ["));
    // marker evidence for canonical runtime example (added this delta)
    let src = std::fs::read_to_string(example("let_reuse_pipeline.opt")).expect("read src");
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_tapped_multi_system_hooks_product() {
    let assert = opticc()
        .args(["run", &example("tapped_multi_system.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![95.0, 75.0, 45.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
    assert!(out.contains("RUN VERIFIED"));
    // prefix hooks on product, multiple lets; emitted hook comments
    assert!(out.contains("before:") && out.contains("after:"));

    let transpile = opticc()
        .args([
            "transpile",
            &example("tapped_multi_system.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("tapped_multi_system.rs").expect("read transpiled");
    let _ = transpile;
    let _ = std::fs::remove_file("tapped_multi_system.rs");
    assert!(rust.contains("// optic(fused): ["));
    assert!(rust.contains("// optic(tap): game_probe"));
    assert!(rust.contains("// optic(record): game_record"));
    // self-host marker evidence (source comment; not emitted in .rs per codegen)
    let src = std::fs::read_to_string(example("tapped_multi_system.opt")).expect("read src");
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_game_loop_pipeline_let_product_tap() {
    let assert = opticc()
        .args(["run", &example("game_loop_pipeline.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![95.0, 75.0, 45.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
    let before_v = parse_velocities_line(&out, "before:");
    let after_v = parse_velocities_line(&out, "after:");
    // vel untouched (richer multi-col data exercised)
    assert_eq!(before_v, after_v);
    assert!(out.contains("RUN VERIFIED"));
    // let reuse + product + prefix tap
    assert!(out.contains("before:") && out.contains("after:"));

    let _ = std::fs::remove_file("game_loop_pipeline.rs"); // cwd hygiene pre-write (per repeated sibling pattern in run_*)
    let transpile = opticc()
        .args([
            "transpile",
            &example("game_loop_pipeline.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("game_loop_pipeline.rs").expect("read transpiled");
    let _ = transpile;
    let _ = std::fs::remove_file("game_loop_pipeline.rs");
    assert!(rust.contains("// optic(fused): ["));
    assert!(rust.contains("// optic(tap): loop_tap"));
    // marker evidence (added to 2 more .opt this delta per PLAN)
    let src = std::fs::read_to_string(example("game_loop_pipeline.opt")).expect("read src");
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_multi_system_fusion_3arity_record() {
    let assert = opticc()
        .args(["run", &example("multi_system_fusion.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![95.0, 75.0, 45.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
    let before_v = parse_velocities_line(&out, "before:");
    let after_v = parse_velocities_line(&out, "after:");
    // vel untouched (exercises 3-arity product via chained lets without mutating all)
    assert_eq!(before_v, after_v);
    assert!(out.contains("RUN VERIFIED"));
    // let reuse + product + prefix record; before/after for harness
    assert!(out.contains("before:") && out.contains("after:"));

    let _ = std::fs::remove_file("multi_system_fusion.rs"); // cwd hygiene pre-write (per repeated sibling pattern in run_*)
    let transpile = opticc()
        .args([
            "transpile",
            &example("multi_system_fusion.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("multi_system_fusion.rs").expect("read transpiled");
    let _ = transpile;
    let _ = std::fs::remove_file("multi_system_fusion.rs");
    assert!(rust.contains("// optic(fused): ["));
    assert!(rust.contains("// optic(record): fusion_record"));
    // additional self-host prep marker evidence for this canonical runtime example
    // marker/arity evidence tests centralized in run_* + parse_*_boundary for future growth (see tapped sibling too)
    let src = std::fs::read_to_string(example("multi_system_fusion.opt")).expect("read src");
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_multi_let_pipeline_let_product_tap() {
    let assert = opticc()
        .args(["run", &example("multi_let_pipeline.opt").to_string_lossy()])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![90.0, 70.0, 40.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(2.0, 2.0), (3.0, 3.0), (4.0, 4.0)]);
    let before_v = parse_velocities_line(&out, "before:");
    let after_v = parse_velocities_line(&out, "after:");
    // vel untouched (declared only; exercises chained let reuse + product without mutating all cols)
    assert_eq!(before_v, after_v);
    assert!(out.contains("RUN VERIFIED"));
    // let reuse + product + prefix tap
    assert!(out.contains("before:") && out.contains("after:"));

    let _ = std::fs::remove_file("multi_let_pipeline.rs"); // cwd hygiene pre-write (per repeated sibling pattern in run_*; repeated in recent pipeline-style run_*; pre-existing in older)
    let transpile = opticc()
        .args([
            "transpile",
            &example("multi_let_pipeline.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("multi_let_pipeline.rs").expect("read transpiled");
    let _ = transpile;
    let _ = std::fs::remove_file("multi_let_pipeline.rs");
    assert!(rust.contains("// optic(fused): ["));
    assert!(rust.contains("// optic(tap): multi_let_tap"));
    // marker evidence for new canonical (also asserted in source run harness)
    let src = std::fs::read_to_string(example("multi_let_pipeline.opt")).expect("read src");
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_arith_fusion_pipeline_let_product_tap() {
    let assert = opticc()
        .args([
            "run",
            &example("arith_fusion_pipeline.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![85.0, 65.0, 35.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(3.0, 3.0), (4.0, 4.0), (5.0, 5.0)]);
    let before_v = parse_velocities_line(&out, "before:");
    let after_v = parse_velocities_line(&out, "after:");
    // vel untouched (declared only; exercises 3-arity let fusion + tuple arith without mutating all cols)
    assert_eq!(before_v, after_v);
    assert!(out.contains("RUN VERIFIED"));
    // let product fusion + prefix tap; boundary for 3-arity
    assert!(out.contains("before:") && out.contains("after:"));

    let _ = std::fs::remove_file("arith_fusion_pipeline.rs"); // cwd hygiene pre-write (per repeated sibling pattern in run_*; repeated in recent pipeline-style run_*; pre-existing in older)
    let transpile = opticc()
        .args([
            "transpile",
            &example("arith_fusion_pipeline.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("arith_fusion_pipeline.rs").expect("read transpiled"); // test-only .expect (sibling precedent in all run_*; harness paths)
    let _ = transpile;
    let _ = std::fs::remove_file("arith_fusion_pipeline.rs");
    assert!(rust.contains("// optic(fused): ["));
    assert!(rust.contains("// optic(tap): arith_tap"));
    // marker evidence (new canonical runtime complex; integration via run_* + harness suffices per sibling precedent e.g. triple/multi_let)
    let src = std::fs::read_to_string(example("arith_fusion_pipeline.opt")).expect("read src"); // test-only .expect (sibling precedent; marker evidence)
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn run_tuple_fusion_pipeline_let_product_tap() {
    let assert = opticc()
        .args([
            "run",
            &example("tuple_fusion_pipeline.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    let before_h = parse_entities_line(&out, "before:");
    let after_h = parse_entities_line(&out, "after:");
    assert_eq!(before_h, vec![100.0, 80.0, 50.0]);
    assert_eq!(after_h, vec![75.0, 55.0, 25.0]);
    let before_p = parse_positions_line(&out, "before:");
    let after_p = parse_positions_line(&out, "after:");
    assert_eq!(before_p, vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]);
    assert_eq!(after_p, vec![(4.0, 4.0), (5.0, 5.0), (6.0, 6.0)]);
    let before_v = parse_velocities_line(&out, "before:");
    let after_v = parse_velocities_line(&out, "after:");
    // vel untouched (declared only; exercises let fusion + tuple arith without mutating all cols)
    assert_eq!(before_v, after_v);
    assert!(out.contains("RUN VERIFIED"));
    // let product fusion + prefix tap; boundary for 3-arity
    assert!(out.contains("before:") && out.contains("after:"));

    let _ = std::fs::remove_file("tuple_fusion_pipeline.rs"); // cwd hygiene pre-write (per repeated sibling pattern in run_*; repeated in recent pipeline-style run_*; pre-existing in older)
    let transpile = opticc()
        .args([
            "transpile",
            &example("tuple_fusion_pipeline.opt").to_string_lossy(),
        ])
        .assert()
        .success();
    let rust = std::fs::read_to_string("tuple_fusion_pipeline.rs").expect("read transpiled"); // test-only .expect (sibling precedent in all run_*; harness paths)
    let _ = transpile;
    let _ = std::fs::remove_file("tuple_fusion_pipeline.rs");
    assert!(rust.contains("// optic(fused): ["));
    assert!(rust.contains("// optic(tap): tuple_tap"));
    // marker evidence (new canonical runtime complex; integration via run_* + harness suffices per sibling precedent e.g. triple/multi_let)
    let src = std::fs::read_to_string(example("tuple_fusion_pipeline.opt")).expect("read src"); // test-only .expect (sibling precedent; marker evidence)
    assert!(src.contains("self-host prep marker"));
}

#[test]
fn arity_mismatch_negative_coverage() {
    // explicit runtime arity-mismatch negative (build on cgi005 + 3-arity runs); dedicated harness case for positive/negative coverage per plan
    let assert = opticc()
        .args(["check", &example("cgi005_arity_mismatch.opt").to_string_lossy()])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(out.contains("CGI") || out.contains("arity") || assert.get_output().stderr.len() > 0);
}

#[test]
fn parse_tuples_helper_boundary() {
    // covers multi-col, empty col (N=0), normal
    let syn = r#"before: Entities { positions: [(0.0, 0.0), (1.0, 1.0)], velocities: [] }"#;
    assert_eq!(
        parse_tuples_for_column(syn, "before:", "positions"),
        vec![(0.0, 0.0), (1.0, 1.0)]
    );
    assert_eq!(
        parse_tuples_for_column(syn, "before:", "velocities"),
        vec![]
    );

    // N=0 for first col + different arity/count
    let n0_first = r#"after: Entities { positions: [], velocities: [(9.0, 9.0)] }"#;
    assert_eq!(
        parse_tuples_for_column(n0_first, "after:", "positions"),
        vec![]
    );
    // arity edge note (extended for 3/4-col products per plan; synthetic for future 4-col; exercised by tuple_fusion 3-arity + new run_*)
    assert_eq!(
        parse_tuples_for_column(n0_first, "after:", "velocities"),
        vec![(9.0, 9.0)]
    );

    // both empty (0 elems)
    let both_empty = r#"before: Entities { positions: [], velocities: [] }"#;
    assert_eq!(
        parse_tuples_for_column(both_empty, "before:", "positions"),
        vec![]
    );
    assert_eq!(
        parse_tuples_for_column(both_empty, "before:", "velocities"),
        vec![]
    );

    // malformed (no proper tuples) yields empty (no crash on split/parse)
    let malformed = r#"x: positions: [ (0.0"#;
    assert_eq!(
        parse_tuples_for_column(malformed, "x:", "positions"),
        vec![]
    );

    // key-present no-[ after col: returns [] early (avoids start=0 pollution from prior col tuples); see else{0} for absent-key case only
    let no_bracket_col = r#"before: Entities { positions: [(1.0, 1.0)], velocities: noarray }"#;
    assert_eq!(
        parse_tuples_for_column(no_bracket_col, "before:", "velocities"),
        vec![]
    );

    // absent-key (contains col but no "col:" key) hits else{0} start=0 (exercises fallback; safe [] here as no prior tuples)
    let absent_key = r#"before: Entities { positionsdata [] }"#;
    assert_eq!(
        parse_tuples_for_column(absent_key, "before:", "positions"),
        vec![]
    );

    // arity edge (exercises parse_tuples on product maps from 3-arity runtime complex e.g. delivered arith_fusion_pipeline)
    let arity3_mix =
        r#"after: Entities { healths: [1.0], positions: [(9.0,9.0)], velocities: [] }"#;
    assert_eq!(
        parse_tuples_for_column(arity3_mix, "after:", "positions"),
        vec![(9.0, 9.0)]
    );

    // missing col panics (harness contract, not tested here to avoid expect in unit)
}

#[test]
fn parse_entities_line_boundary() {
    // N=0 / empty data robustness (product arity + harness quality).
    // Full runtime N=0 entities exec via .opt not present (see PLAN immediate next + fixtures/README carve-out (N=0 codegen deferred; synthetic + boundary only; no NOTE yet));
    // handled via synthetic/minimal path + boundary units + richer arity edges (parse coverage for clean-tree N=0 cases; delivered arith_fusion + prior).
    let empty = r#"before: Entities { healths: [] }"#;
    assert_eq!(parse_entities_line(empty, "before:"), vec![]);

    let n0_after = r#"after: Entities { healths: [] }"#;
    assert_eq!(parse_entities_line(n0_after, "after:"), vec![]);

    // normal still works
    let norm = r#"before: Entities { healths: [100.0, 80.0, 50.0] }"#;
    assert_eq!(
        parse_entities_line(norm, "before:"),
        vec![100.0, 80.0, 50.0]
    );

    // guard for end <= start (malformed ] before [) + multi-col empty synthetic
    let malformed = r#"before: Entities { healths: ]100[ }"#;
    assert_eq!(parse_entities_line(malformed, "before:"), vec![]);
    let multi_empty = r#"before: Entities { healths: [], positions: [], velocities: [ (0,0) ] }"#;
    assert_eq!(parse_entities_line(multi_empty, "before:"), vec![]);

    // richer arity edge: explicit 3-col N=0 for product/let fusion boundary (synthetic exec path)
    let n0_3col = r#"before: Entities { healths: [], positions: [], velocities: [] }"#;
    assert_eq!(parse_entities_line(n0_3col, "before:"), vec![]);
    let n0_3col_after =
        r#"after: Entities { healths: [], positions: [(0.0,0.0)], velocities: [] }"#;
    assert_eq!(parse_entities_line(n0_3col_after, "after:"), vec![]);

    // ]-before-[ guard + no-`]` (None) arm coverage; start=None returns [] (empty-col/N=0)
    let end_before_start = r#"before: Entities { healths: foo]bar[ }"#;
    assert_eq!(parse_entities_line(end_before_start, "before:"), vec![]);
    let no_close = r#"after: Entities { healths: [1.0, 2.0"#;
    assert_eq!(parse_entities_line(no_close, "after:"), vec![]);

    // explicit no-`[` case for None arm (empty-col/N=0 guard; no slice)
    let no_bracket = r#"before: Entities { healths: foo bar }"#;
    assert_eq!(parse_entities_line(no_bracket, "before:"), vec![]);

    // isolated boundary case using delivered arith_fusion sample (35.0 health)
    let arith_sample = r#"after: Entities { healths: [85.0, 65.0, 35.0] }"#;
    assert_eq!(
        parse_entities_line(arith_sample, "after:"),
        vec![85.0, 65.0, 35.0]
    );
}
