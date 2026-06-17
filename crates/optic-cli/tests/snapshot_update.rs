use assert_cmd::Command;

fn opticc() -> Command {
    Command::cargo_bin("opticc").unwrap()
}

#[test]
fn snapshot_update_requires_confirm() {
    opticc()
        .arg("snapshot-update")
        .assert()
        .failure()
        .stderr(predicates::str::contains("without --confirm"));
}

#[test]
fn snapshot_update_refuse_message_mentions_confirm_flag() {
    opticc()
        .arg("snapshot-update")
        .assert()
        .failure()
        .stderr(predicates::str::contains("--confirm"));
}

#[test]
#[ignore = "mutates fixtures; run manually: opticc snapshot-update --confirm"]
fn snapshot_update_with_confirm_runs() {
    opticc()
        .args(["snapshot-update", "--confirm"])
        .assert()
        .success();
}
