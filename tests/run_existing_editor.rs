#[path = "support/run_fixture.rs"]
mod run_fixture;

use run_fixture::Fixture;
use std::fs;

#[test]
fn existing_editor_plan_is_read_only_and_reports_candidates_without_guessing() {
    let fixture = Fixture::new();
    let candidate = fixture.temp.path().join("editor.json");
    fs::write(
        &candidate,
        r#"{
          "schemaVersion": 1,
          "description": "editor",
          "useWhen": "editor",
          "project": "main",
          "backend": "editor",
          "editor": {"mode": "editor", "rhi": "default", "nativeArgs": []}
        }"#,
    )
    .unwrap();
    let candidate_text = candidate.to_string_lossy().to_string();
    let configured = fixture
        .command()
        .args([
            "--format",
            "json",
            "run",
            "configure",
            "editor",
            "--workspace",
            "fixture",
            "--file",
            &candidate_text,
        ])
        .output()
        .unwrap();
    assert!(configured.status.success());

    let planned = fixture
        .command()
        .args([
            "--format",
            "json",
            "run",
            "plan",
            "editor",
            "--workspace",
            "fixture",
            "--existing-editor",
        ])
        .output()
        .unwrap();
    assert!(
        planned.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&planned.stdout),
        String::from_utf8_lossy(&planned.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&planned.stdout).unwrap();
    let existing = &value["data"]["existingEditor"];
    assert!(existing["candidates"].is_array());
    assert!(existing["selectedPid"].is_null());
    assert!(existing["currentMap"].is_null());
    assert!(existing["actions"].as_array().unwrap().len() >= 2);
    assert!(!fixture.config_dir.join("executions/run").exists());
}
