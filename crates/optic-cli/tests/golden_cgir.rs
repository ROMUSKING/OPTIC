use assert_cmd::Command;
use std::path::PathBuf;

fn opticc() -> Command {
    Command::cargo_bin("opticc").unwrap()
}

fn fixture_path(subdir: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/{subdir}/{name}.txt"))
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn assert_golden(subdir: &str, example_name: &str, before_fusion: bool) {
    let mut cmd = opticc();
    cmd.arg("dump-cgir").arg(example(example_name));
    if before_fusion {
        cmd.arg("--before-fusion");
    }
    let out = cmd.assert().success();
    let actual = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let stem = example_name.trim_end_matches(".opt");
    let path = fixture_path(subdir, stem);
    if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture dir");
        }
        std::fs::write(&path, &actual).expect("write golden");
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {} (OPTIC_UPDATE_GOLDEN=1)", path.display()));
    assert_eq!(
        actual, expected,
        "golden mismatch for {example_name} ({subdir})"
    );

    let mut check = opticc();
    check
        .arg("dump-cgir")
        .arg(example(example_name))
        .arg("--check");
    if before_fusion {
        check.arg("--before-fusion");
    }
    check.assert().success();
}

#[test]
fn golden_cgir_pre_health_decay() {
    assert_golden("cgir/pre", "health_decay.opt", true);
}

#[test]
fn golden_cgir_post_health_decay() {
    assert_golden("cgir/post", "health_decay.opt", false);
}

#[test]
fn golden_cgir_pre_health_position() {
    assert_golden("cgir/pre", "health_position.opt", true);
}

#[test]
fn golden_cgir_post_health_position() {
    assert_golden("cgir/post", "health_position.opt", false);
}

#[test]
fn golden_cgir_pre_nested_position() {
    assert_golden("cgir/pre", "nested_position.opt", true);
}

#[test]
fn golden_cgir_post_nested_position() {
    assert_golden("cgir/post", "nested_position.opt", false);
}

#[test]
fn golden_cgir_pre_health_get() {
    assert_golden("cgir/pre", "health_get.opt", true);
}

#[test]
fn golden_cgir_post_health_get() {
    assert_golden("cgir/post", "health_get.opt", false);
}

#[test]
fn golden_cgir_pre_health_set() {
    assert_golden("cgir/pre", "health_set.opt", true);
}

#[test]
fn golden_cgir_post_health_set() {
    assert_golden("cgir/post", "health_set.opt", false);
}

#[test]
fn golden_cgir_pre_compose_decay() {
    assert_golden("cgir/pre", "compose_decay.opt", true);
}

#[test]
fn golden_cgir_post_compose_decay() {
    assert_golden("cgir/post", "compose_decay.opt", false);
}

#[test]
fn golden_cgir_pre_compose_triple() {
    assert_golden("cgir/pre", "compose_triple.opt", true);
}

#[test]
fn golden_cgir_post_compose_triple() {
    assert_golden("cgir/post", "compose_triple.opt", false);
}

#[test]
fn golden_cgir_pre_nested_field_triple() {
    assert_golden("cgir/pre", "nested_field_triple.opt", true);
}

#[test]
fn golden_cgir_post_nested_field_triple() {
    assert_golden("cgir/post", "nested_field_triple.opt", false);
}

#[test]
fn golden_cgir_pre_alive_filter() {
    assert_golden("cgir/pre", "alive_filter.opt", true);
}

#[test]
fn golden_cgir_post_alive_filter() {
    assert_golden("cgir/post", "alive_filter.opt", false);
}

#[test]
fn golden_cgir_pre_prism_get() {
    assert_golden("cgir/pre", "prism_get.opt", true);
}

#[test]
fn golden_cgir_post_prism_get() {
    assert_golden("cgir/post", "prism_get.opt", false);
}

#[test]
fn golden_cgir_pre_prism_set() {
    assert_golden("cgir/pre", "prism_set.opt", true);
}

#[test]
fn golden_cgir_post_prism_set() {
    assert_golden("cgir/post", "prism_set.opt", false);
}

#[test]
fn golden_cgir_pre_partial_prism() {
    assert_golden("cgir/pre", "partial_prism.opt", true);
}

#[test]
fn golden_cgir_post_partial_prism() {
    assert_golden("cgir/post", "partial_prism.opt", false);
}

#[test]
fn golden_cgir_pre_all_healths() {
    assert_golden("cgir/pre", "all_healths.opt", true);
}

#[test]
fn golden_cgir_post_all_healths() {
    assert_golden("cgir/post", "all_healths.opt", false);
}

#[test]
fn golden_cgir_pre_traversal_get() {
    assert_golden("cgir/pre", "traversal_get.opt", true);
}

#[test]
fn golden_cgir_post_traversal_get() {
    assert_golden("cgir/post", "traversal_get.opt", false);
}

#[test]
fn golden_cgir_pre_traversal_set() {
    assert_golden("cgir/pre", "traversal_set.opt", true);
}

#[test]
fn golden_cgir_post_traversal_set() {
    assert_golden("cgir/post", "traversal_set.opt", false);
}

#[test]
fn golden_cgir_pre_tap_health() {
    assert_golden("cgir/pre", "tap_health.opt", true);
}

#[test]
fn golden_cgir_post_tap_health() {
    assert_golden("cgir/post", "tap_health.opt", false);
}

#[test]
fn golden_cgir_pre_record_health() {
    assert_golden("cgir/pre", "record_health.opt", true);
}

#[test]
fn golden_cgir_post_record_health() {
    assert_golden("cgir/post", "record_health.opt", false);
}

#[test]
fn golden_cgir_pre_tap_record_chain() {
    assert_golden("cgir/pre", "tap_record_chain.opt", true);
}

#[test]
fn golden_cgir_post_tap_record_chain() {
    assert_golden("cgir/post", "tap_record_chain.opt", false);
}

#[test]
fn golden_cgir_pre_compose_tap() {
    assert_golden("cgir/pre", "compose_tap.opt", true);
}

#[test]
fn golden_cgir_post_compose_tap() {
    assert_golden("cgir/post", "compose_tap.opt", false);
}

// Follows exact pattern of game/mixed/reusable/rich/triple/let/tapped/game_loop/multi: CGIR only (per fixtures/README runtime carve-out).
#[test]
fn golden_cgir_pre_game_entity_sim() {
    assert_golden("cgir/pre", "game_entity_sim.opt", true);
}

#[test]
fn golden_cgir_post_game_entity_sim() {
    assert_golden("cgir/post", "game_entity_sim.opt", false);
}

#[test]
fn golden_cgir_pre_mixed_prism_traversal() {
    assert_golden("cgir/pre", "mixed_prism_traversal.opt", true);
}

#[test]
fn golden_cgir_post_mixed_prism_traversal() {
    assert_golden("cgir/post", "mixed_prism_traversal.opt", false);
}

#[test]
fn golden_cgir_pre_reusable_and_taps() {
    assert_golden("cgir/pre", "reusable_and_taps.opt", true);
}

#[test]
fn golden_cgir_post_reusable_and_taps() {
    assert_golden("cgir/post", "reusable_and_taps.opt", false);
}

#[test]
fn golden_cgir_pre_rich_entity_update() {
    assert_golden("cgir/pre", "rich_entity_update.opt", true);
}

#[test]
fn golden_cgir_post_rich_entity_update() {
    assert_golden("cgir/post", "rich_entity_update.opt", false);
}

// Follows exact pattern of game/mixed/reusable/rich/triple/let/tapped/game_loop/multi: CGIR only (per fixtures/README runtime carve-out).
#[test]
fn golden_cgir_pre_triple_product_fusion() {
    assert_golden("cgir/pre", "triple_product_fusion.opt", true);
}

#[test]
fn golden_cgir_post_triple_product_fusion() {
    assert_golden("cgir/post", "triple_product_fusion.opt", false);
}

#[test]
fn golden_cgir_pre_let_reuse_pipeline() {
    assert_golden("cgir/pre", "let_reuse_pipeline.opt", true);
}

#[test]
fn golden_cgir_post_let_reuse_pipeline() {
    assert_golden("cgir/post", "let_reuse_pipeline.opt", false);
}

#[test]
fn golden_cgir_pre_tapped_multi_system() {
    assert_golden("cgir/pre", "tapped_multi_system.opt", true);
}

#[test]
fn golden_cgir_post_tapped_multi_system() {
    assert_golden("cgir/post", "tapped_multi_system.opt", false);
}

// Follows exact pattern of game/mixed/reusable/rich/triple/let/tapped/game_loop/multi: CGIR only (per fixtures/README runtime carve-out).
#[test]
fn golden_cgir_pre_game_loop_pipeline() {
    assert_golden("cgir/pre", "game_loop_pipeline.opt", true);
}

#[test]
fn golden_cgir_post_game_loop_pipeline() {
    assert_golden("cgir/post", "game_loop_pipeline.opt", false);
}

#[test]
fn golden_cgir_pre_multi_system_fusion() {
    assert_golden("cgir/pre", "multi_system_fusion.opt", true);
}

#[test]
fn golden_cgir_post_multi_system_fusion() {
    assert_golden("cgir/post", "multi_system_fusion.opt", false);
}

// Follows exact pattern of game/mixed/reusable/rich/triple/let/tapped/game_loop/multi/multi_let: CGIR only (per fixtures/README runtime carve-out).
#[test]
fn golden_cgir_pre_multi_let_pipeline() {
    assert_golden("cgir/pre", "multi_let_pipeline.opt", true);
}

#[test]
fn golden_cgir_post_multi_let_pipeline() {
    assert_golden("cgir/post", "multi_let_pipeline.opt", false);
}

// Follows exact pattern of game/mixed/reusable/rich/triple/let/tapped/game_loop/multi/multi_let/arith: CGIR only (per fixtures/README runtime carve-out).
#[test]
fn golden_cgir_pre_arith_fusion_pipeline() {
    assert_golden("cgir/pre", "arith_fusion_pipeline.opt", true);
}

#[test]
fn golden_cgir_post_arith_fusion_pipeline() {
    assert_golden("cgir/post", "arith_fusion_pipeline.opt", false);
}

// Follows exact pattern of game/mixed/reusable/rich/triple/let/tapped/game_loop/multi/multi_let/arith: CGIR only (per fixtures/README runtime carve-out).
#[test]
fn golden_cgir_pre_tuple_fusion_pipeline() {
    assert_golden("cgir/pre", "tuple_fusion_pipeline.opt", true);
}

#[test]
fn golden_cgir_post_tuple_fusion_pipeline() {
    assert_golden("cgir/post", "tuple_fusion_pipeline.opt", false);
}
