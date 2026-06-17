use assert_cmd::Command;

fn opticc() -> Command {
    Command::cargo_bin("opticc").unwrap()
}

#[test]
fn bench_within_tolerance_exits_zero() {
    let assert = opticc().arg("bench").assert().success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        out.contains("within tolerance") && out.contains("5x baseline"),
        "bench output must document tolerance gate: {out}"
    );
}
