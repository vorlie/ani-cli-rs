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

#[test]
fn showcase_search_is_fixture_backed_and_hidden_from_help() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args([
            "--demo-mode",
            "--provider",
            "allanime",
            "search",
            "starfall",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "showcase:starfall-atelier\tStarfall Atelier (12 episodes)",
        ));

    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--demo-mode").not());
}

#[test]
fn showcase_exposes_deterministic_episodes_and_quality_metadata() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args([
            "--demo-mode",
            "episodes",
            "showcase:starfall-atelier",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"1\""))
        .stdout(predicate::str::contains("\"12\""));

    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args([
            "--demo-mode",
            "--provider",
            "anikoto",
            "links",
            "anikoto:showcase-starfall-atelier",
            "1",
            "--quality",
            "720p",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"resolution\": \"720p\""))
        .stdout(predicate::str::contains("\"label\": \"English\""))
        .stdout(predicate::str::contains("showcase.invalid"));
}

#[test]
fn showcase_adult_filter_is_explicit() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["--demo-mode", "search", "velvet"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["--demo-mode", "search", "velvet", "--allow-adult"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Velvet Nebula"));
}
