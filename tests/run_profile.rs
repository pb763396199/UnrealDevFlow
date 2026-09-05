#[path = "support/run_fixture.rs"]
mod run_fixture;

use run_fixture::Fixture;
use std::fs;

#[test]
fn profile_file_is_reused_by_a_fresh_process_without_execution_side_effects() {
    let fixture = Fixture::new();
    let candidate = fixture.temp.path().join("boot.json");
    fs::write(
        &candidate,
        r#"{
          "schemaVersion": 1,
          "description": "boot",
          "useWhen": "boot check",
          "project": "main",
          "backend": "gauntlet",
          "gauntlet": {
            "test": "UE.EditorBootTest",
            "build": "editor",
            "platform": "Win64",
            "configuration": "Development",
            "nullRhi": true,
            "unattended": true,
            "execCmds": [],
            "maxDurationSeconds": 30
          }
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
            "boot",
            "--workspace",
            "fixture",
            "--file",
            &candidate_text,
        ])
        .output()
        .unwrap();
    assert!(configured.status.success());

    let fresh = fixture
        .command()
        .args([
            "--format",
            "json",
            "run",
            "plan",
            "boot",
            "--workspace",
            "fixture",
        ])
        .output()
        .unwrap();
    assert!(
        fresh.status.success(),
        "{}",
        String::from_utf8_lossy(&fresh.stderr)
    );
    let plan: serde_json::Value = serde_json::from_slice(&fresh.stdout).unwrap();
    assert_eq!(plan["data"]["nativeArgv"][0], "RunUnreal");
    assert_eq!(plan["data"]["nativeArgv"][3], "-test=UE.EditorBootTest");
    assert_eq!(plan["data"]["nativeArgv"][6], "-NullRHI");
    assert!(!fixture.config_dir.join("executions/run").exists());
}

#[test]
fn configure_rejects_unknown_fields_before_creating_a_profile() {
    let fixture = Fixture::new();
    let candidate = fixture.temp.path().join("unknown.json");
    fs::write(
        &candidate,
        r#"{
          "schemaVersion": 1,
          "description": "bad",
          "useWhen": "bad",
          "project": "main",
          "backend": "editor",
          "unexpected": true,
          "editor": {"mode": "editor", "rhi": "default", "nativeArgs": []}
        }"#,
    )
    .unwrap();
    let candidate_text = candidate.to_string_lossy().to_string();
    let output = fixture
        .command()
        .args([
            "--format",
            "json",
            "run",
            "configure",
            "bad",
            "--workspace",
            "fixture",
            "--file",
            &candidate_text,
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        !fixture
            .config_dir
            .join("workspaces/fixture/run/bad.json")
            .exists()
    );
}
