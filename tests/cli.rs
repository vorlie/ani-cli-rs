use assert_cmd::Command;
use predicates::prelude::*;
use ani_lib::{AniError, ErrorCode, ErrorReport, ErrorVerbosity};

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
        .stdout(predicate::str::contains("anikoto2"));
}

#[test]
fn invalid_mode_fails_before_network_for_episodes() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["episodes", "show", "--mode", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid input"));
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
        .stdout(predicate::str::contains("ANI_CLI_RS_PROVIDER"));
}

#[test]
fn removed_allanime_provider_is_rejected() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["--provider", "allanime", "search", "example"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "invalid value 'allanime' for '--provider <PROVIDER>'",
        ));
}

#[test]
fn invalid_default_anikoto_ids_fail_before_network() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["--provider", "anikoto", "episodes", "not-a-valid-id"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid input"));
}

#[test]
fn environment_can_select_anikoto() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .env("ANI_CLI_PROVIDER", "anikoto2")
        .args(["episodes", "not a slug"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid input"));
}

#[test]
fn prefixed_ids_auto_route_to_anikoto() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["episodes", "anikoto:not-base64"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid input"));

    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["episodes", "anikoto2:not-base64"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid input"));
}

#[test]
fn showcase_search_is_fixture_backed_and_hidden_from_help() {
    Command::cargo_bin("ani-cli-rs")
        .unwrap()
        .args(["--demo-mode", "--provider", "anikoto", "search", "starfall"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "anikoto:showcase-starfall-atelier\tStarfall Atelier (12 episodes)",
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

#[test]
fn error_codes_are_stable() {
    // Test that error codes maintain their IDs
    assert_eq!(ErrorCode::NoPlayableSources.id(), "ACL-3001");
    assert_eq!(ErrorCode::ProviderUnavailable.id(), "ACL-1001");
    assert_eq!(ErrorCode::NoSearchResults.id(), "ACL-2001");
    assert_eq!(ErrorCode::PlayerNotFound.id(), "ACL-5001");
    assert_eq!(ErrorCode::NoDownloadTool.id(), "ACL-4002");
}

#[test]
fn error_slugs_are_consistent() {
    // Test that error slugs are URL-friendly
    assert_eq!(ErrorCode::NoPlayableSources.slug(), "no-playable-sources");
    assert_eq!(ErrorCode::ProviderUnavailable.slug(), "provider-unavailable");
    assert_eq!(ErrorCode::NoSearchResults.slug(), "no-search-results");
}

#[test]
fn error_code_mapping_works() {
    // Test that AniError maps to correct ErrorCode
    let error = AniError::NoPlayableSources {
        anime: Some("Test Anime".to_string()),
        episode: Some("1".to_string()),
        mode: Some("sub".to_string()),
    };
    assert_eq!(error.code(), ErrorCode::NoPlayableSources);

    let error = AniError::EmptySearchQuery;
    assert_eq!(error.code(), ErrorCode::EmptySearchQuery);

    let error = AniError::NoDownloadTool;
    assert_eq!(error.code(), ErrorCode::NoDownloadTool);
}

#[test]
fn error_report_normal_mode_includes_code_and_title() {
    let error = AniError::NoPlayableSources {
        anime: Some("Test Anime".to_string()),
        episode: Some("1".to_string()),
        mode: Some("sub".to_string()),
    };
    let report = ErrorReport::from_error(&error);
    let output = report.render(ErrorVerbosity::Normal);

    assert!(output.contains("ACL-3001"));
    assert!(output.contains("No playable sources found"));
    // In verbose mode, context would be shown
    let verbose_output = report.render(ErrorVerbosity::Verbose);
    assert!(verbose_output.contains("Test Anime"));
    assert!(verbose_output.contains("Episode: 1"));
    assert!(verbose_output.contains("Mode: sub"));
}

#[test]
fn error_report_verbose_mode_includes_context() {
    let error = AniError::NoPlayableSources {
        anime: Some("Test Anime".to_string()),
        episode: Some("1".to_string()),
        mode: Some("sub".to_string()),
    };
    let report = ErrorReport::from_error(&error);
    let output = report.render(ErrorVerbosity::Verbose);

    assert!(output.contains("ACL-3001"));
    assert!(output.contains("Anime: Test Anime"));
    assert!(output.contains("Episode: 1"));
    assert!(output.contains("Mode: sub"));
}

#[test]
fn error_report_debug_mode_includes_diagnostics() {
    let error = AniError::NoPlayableSources {
        anime: Some("Test Anime".to_string()),
        episode: Some("1".to_string()),
        mode: Some("sub".to_string()),
    };
    let report = ErrorReport::from_error(&error);
    let output = report.render(ErrorVerbosity::Debug);

    assert!(output.contains("ACL-3001"));
    assert!(output.contains("Debug information"));
    assert!(output.contains("error_code"));
    assert!(output.contains("error_slug"));
}

#[test]
fn error_report_includes_help_text() {
    let error = AniError::NoDownloadTool;
    let report = ErrorReport::from_error(&error);
    let output = report.render(ErrorVerbosity::Normal);

    assert!(output.contains("help:"));
    assert!(output.contains("yt-dlp"));
    assert!(output.contains("FFmpeg"));
}

#[test]
fn error_report_includes_documentation_link() {
    let error = AniError::NoPlayableSources {
        anime: None,
        episode: None,
        mode: None,
    };
    let report = ErrorReport::from_error(&error);
    let output = report.render(ErrorVerbosity::Normal);

    assert!(output.contains("docs:"));
    assert!(output.contains("vorlie.github.io"));
    // The slug is part of the docs URL
    assert!(output.contains("errors"));
}
