use assert_cmd::Command;
use std::path::PathBuf;
use std::process::Command as StdCommand;

fn opticc() -> Command {
    Command::cargo_bin("opticc").unwrap()
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

#[test]
fn check_positive_decay() {
    opticc()
        .args(["check", &example("health_decay.opt").to_string_lossy()])
        .assert()
        .success();
}

#[test]
fn check_negative_alias() {
    let assert = opticc()
        .args(["check", &example("invalid_alias.opt").to_string_lossy()])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("ALI-201"));
    assert!(out.contains("conflicting_regions"));
}

#[test]
fn check_negative_grade_decl() {
    let assert = opticc()
        .args(["check", &example("invalid_grade.opt").to_string_lossy()])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("GRA-110"));
}

#[test]
fn check_negative_grade_compose() {
    let assert = opticc()
        .args(["check", &example("grade_mismatch.opt").to_string_lossy()])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("GRA-104"));
}

#[test]
fn check_fails_cgi005_on_product_map_arity_mismatch() {
    let assert = opticc()
        .args([
            "check",
            &example("cgi005_arity_mismatch.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("CGI-005"));
}

#[test]
fn transpile_health_decay_writes_rust() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("out.rs");
    opticc()
        .args([
            "transpile",
            &example("health_decay.opt").to_string_lossy(),
            "--out",
            &out.to_string_lossy(),
        ])
        .assert()
        .success();
    let content = std::fs::read_to_string(&out).expect("read emitted");
    assert!(content.contains("run_example"));
    assert!(content.contains("cursor_0.arena.healths"));
}

#[test]
fn transpile_health_decay_compiles_with_cargo_check() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let vdir = tmp.path();
    let main_rs = vdir.join("main.rs");
    opticc()
        .args([
            "transpile",
            &example("health_decay.opt").to_string_lossy(),
            "--out",
            &main_rs.to_string_lossy(),
        ])
        .assert()
        .success();
    let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../optic-runtime");
    std::fs::write(
        vdir.join("Cargo.toml"),
        format!(
            "[package]\nname=\"v\"\nversion=\"0.1.0\"\nedition=\"2021\"\n[dependencies]\noptic-runtime = {{ path = \"{}\" }}\n[[bin]]\nname=\"v\"\npath=\"main.rs\"\n",
            runtime.display()
        ),
    )
    .expect("write manifest");
    let manifest = vdir.join("Cargo.toml");
    let status = StdCommand::new("cargo")
        .args(["check", "--quiet", "--manifest-path"])
        .arg(&manifest)
        .current_dir(vdir)
        .status()
        .expect("cargo check");
    assert!(
        status.success(),
        "transpiled health_decay must pass cargo check"
    );
}

#[test]
fn check_json_failure_emits_json() {
    let assert = opticc()
        .args([
            "check",
            "--json",
            &example("invalid_alias.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("\"ok\": false") || out.contains("\"ok\":false"));
    assert!(out.contains("ALI-201"));
}
