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
