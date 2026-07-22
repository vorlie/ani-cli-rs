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
        .stdout(predicate::str::contains("--update"))
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

#[test]
fn download_help_distinguishes_show_ids_from_titles() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["download", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not an anime title"));
}

#[test]
fn help_documents_provider_selection() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--provider"))
        .stdout(predicate::str::contains("ANI_CLI_PROVIDER"));
}

#[test]
fn allanime_only_diagnostics_reject_anikoto() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["--provider", "anikoto", "debug"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AllAnime-only"));
}

#[test]
fn numeric_anikoto_ids_require_explicit_provider() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["--provider", "anikoto", "episodes", "not-a-valid-id"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid Anikoto show ID"));
}

#[test]
fn environment_can_select_anikoto() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .env("ANI_CLI_PROVIDER", "anikoto")
        .arg("debug")
        .assert()
        .failure()
        .stderr(predicate::str::contains("AllAnime-only"));
}

#[test]
fn prefixed_ids_auto_route_to_anikoto() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["episodes", "anikoto:not-base64"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid Anikoto show ID"));
}
