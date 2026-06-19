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
fn check_compose_focus_mismatch_fails_verify() {
    let assert = opticc()
        .args([
            "check",
            &example("compose_focus_mismatch.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("compose type wiring invalid")
            || stderr.contains("CGI-004")
            || stderr.contains("GRA-104"),
        "invalid compose must fail before silent unfused codegen: {stderr}"
    );
}

#[test]
fn check_compose_field_access_rejects_cgi003() {
    let assert = opticc()
        .args([
            "check",
            &example("compose_field_access.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("CGI-003")
            || stderr.contains("TYP-004")
            || stderr.contains("unsupported optic body in compose chain")
            || stderr.contains("cannot infer optic body type"),
        "field access in compose chain must fail early: {stderr}"
    );
}

#[test]
fn check_compose_escape_warns_fus501() {
    let assert = opticc()
        .args(["check", &example("compose_escape.opt").to_string_lossy()])
        .assert()
        .success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("FUS-501"));
    assert!(stderr.contains("intermediate value escapes"));
}

#[test]
fn check_positive_compose_triple() {
    opticc()
        .args(["check", &example("compose_triple.opt").to_string_lossy()])
        .assert()
        .success();
}

#[test]
fn check_positive_nested_position() {
    opticc()
        .args(["check", &example("nested_position.opt").to_string_lossy()])
        .assert()
        .success();
}

#[test]
fn check_positive_nested_field_triple() {
    opticc()
        .args([
            "check",
            &example("nested_field_triple.opt").to_string_lossy(),
        ])
        .assert()
        .success();
}

#[test]
fn run_compose_triple_verified() {
    opticc()
        .args(["run", &example("compose_triple.opt").to_string_lossy()])
        .assert()
        .success()
        .stdout(predicates::str::contains("RUN VERIFIED"));
}

#[test]
fn check_positive_compose_decay() {
    opticc()
        .args(["check", &example("compose_decay.opt").to_string_lossy()])
        .assert()
        .success();
}

#[test]
fn run_compose_decay_verified() {
    opticc()
        .args(["run", &example("compose_decay.opt").to_string_lossy()])
        .assert()
        .success()
        .stdout(predicates::str::contains("RUN VERIFIED"));
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
fn check_negative_typ001_unknown_type() {
    let assert = opticc()
        .args([
            "check",
            &example("typ001_unknown_type.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("TYP-001"));
}

#[test]
fn check_negative_typ002_body_mismatch() {
    let assert = opticc()
        .args([
            "check",
            &example("typ002_body_mismatch.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("TYP-002"));
}

#[test]
fn check_negative_typ003_grade_syntax() {
    let assert = opticc()
        .args([
            "check",
            &example("typ003_grade_syntax.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("TYP-003"));
}

#[test]
fn check_negative_typ003_unknown_dim() {
    let assert = opticc()
        .args([
            "check",
            &example("typ003_unknown_dim.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("TYP-003"));
}

#[test]
fn check_negative_typ004_uninferable_body() {
    let assert = opticc()
        .args([
            "check",
            &example("typ004_uninferable_body.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("TYP-004"));
}

#[test]
fn check_negative_typ001_unknown_focus() {
    let assert = opticc()
        .args([
            "check",
            &example("typ001_unknown_focus.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("TYP-001"));
}

#[test]
fn check_negative_typ002_put_mismatch() {
    let assert = opticc()
        .args([
            "check",
            &example("typ002_put_mismatch.opt").to_string_lossy(),
        ])
        .assert()
        .failure();
    let out = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(out.contains("TYP-002"));
}

#[test]
fn explain_grade_fails_typ001_on_target() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("typ001_unknown_type.opt").to_string_lossy(),
            "--node",
            "GhostView",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("TYP-001"));
}

#[test]
fn explain_grade_fails_typ002_on_target() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("typ002_body_mismatch.opt").to_string_lossy(),
            "--node",
            "BadFocus",
            "--json",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("TYP-002"));
    assert!(stderr.contains("\"ok\": false"));
}

#[test]
fn explain_grade_fails_typ003_on_target() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("typ003_grade_syntax.opt").to_string_lossy(),
            "--node",
            "BadGrade",
            "--json",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("TYP-003"));
    assert!(stderr.contains("\"ok\": false"));
}

#[test]
fn explain_grade_fails_typ004_on_target() {
    let assert = opticc()
        .args([
            "explain-grade",
            &example("typ004_uninferable_body.opt").to_string_lossy(),
            "--node",
            "BadInfer",
            "--json",
        ])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(stderr.contains("TYP-004"));
    assert!(stderr.contains("\"ok\": false"));
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
fn transpile_compose_decay_compiles_with_cargo_check() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let vdir = tmp.path();
    let main_rs = vdir.join("main.rs");
    opticc()
        .args([
            "transpile",
            &example("compose_decay.opt").to_string_lossy(),
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
        "transpiled compose_decay must pass cargo check"
    );
}

#[test]
fn transpile_nested_position_compiles_with_cargo_check() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let vdir = tmp.path();
    let main_rs = vdir.join("main.rs");
    opticc()
        .args([
            "transpile",
            &example("nested_position.opt").to_string_lossy(),
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
        "transpiled nested_position must pass cargo check"
    );
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
