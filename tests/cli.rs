use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_lists_legacy_and_scriptable_interfaces() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--continue"))
        .stdout(predicate::str::contains("--allow-adult"))
        .stdout(predicate::str::contains("refresh-cipher-map"));
}

#[test]
fn invalid_mode_fails_before_network_for_episodes() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["episodes", "show", "--mode", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "translation type must be sub or dub",
        ));
}
