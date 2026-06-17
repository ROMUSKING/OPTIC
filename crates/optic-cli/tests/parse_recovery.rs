use assert_cmd::Command;
use std::io::Write;
use tempfile::NamedTempFile;

fn opticc() -> Command {
    Command::cargo_bin("opticc").unwrap()
}

#[test]
fn check_fails_on_parse_error_with_valid_trailing_decl() {
    let mut tmp = NamedTempFile::new().expect("temp file");
    write!(tmp, "let bad = ;\ndata Entities {{ healths: SoA<f32> }}\n").expect("write");
    opticc()
        .args(["check", &tmp.path().to_string_lossy()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("PAR-001"));
}
