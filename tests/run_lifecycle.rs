#[path = "support/fake_native.rs"]
mod fake_native;
#[path = "support/run_fixture.rs"]
mod run_fixture;

use fake_native::write_record;
use run_fixture::Fixture;

#[test]
fn compare_pass_after_fail_is_a_single_json_success_envelope() {
    let fixture = Fixture::new();
    let project = fixture.project.join("Fixture.uproject");
    write_record(
        &fixture.config_dir,
        "before-failed",
        "failed",
        "md5:same",
        &project,
        &fixture.engine,
        Some("/Game/Maps/Smoke"),
    );
    write_record(
        &fixture.config_dir,
        "after-passed",
        "passed",
        "md5:same",
        &project,
        &fixture.engine,
        Some("/Game/Maps/Smoke"),
    );

    let output = fixture
        .command()
        .args([
            "--format",
            "json",
            "run",
            "compare",
            "before-failed",
            "after-passed",
            "--expect",
            "pass-after-fail",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let documents = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(documents.len(), 1, "stdout must contain one JSON document");
    let value = &documents[0];
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["expectationMet"], true);
}

#[test]
fn failed_expectation_keeps_comparison_data_and_returns_nonzero_once() {
    let fixture = Fixture::new();
    let project = fixture.project.join("Fixture.uproject");
    write_record(
        &fixture.config_dir,
        "before-passed",
        "passed",
        "md5:same",
        &project,
        &fixture.engine,
        None,
    );
    write_record(
        &fixture.config_dir,
        "after-failed",
        "failed",
        "md5:same",
        &project,
        &fixture.engine,
        None,
    );

    let output = fixture
        .command()
        .args([
            "--format",
            "json",
            "run",
            "compare",
            "before-passed",
            "after-failed",
            "--expect",
            "pass-after-fail",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let documents = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        documents.len(),
        1,
        "failure must not emit a second JSON document"
    );
    let value = &documents[0];
    assert_eq!(value["ok"], false);
    assert_eq!(value["data"]["expectationMet"], false);
    assert!(value["error"].is_string());
}

#[test]
fn status_does_not_turn_a_missing_final_observation_into_success() {
    let fixture = Fixture::new();
    let project = fixture.project.join("Fixture.uproject");
    write_record(
        &fixture.config_dir,
        "running-gone",
        "running",
        "md5:run",
        &project,
        &fixture.engine,
        None,
    );
    let output = fixture
        .command()
        .args(["--format", "json", "run", "status", "running-gone"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["state"], "unknown");
    assert!(
        value["data"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "missing_final_observation")
    );
    assert!(value["data"]["toolExitCode"].is_null());
}
