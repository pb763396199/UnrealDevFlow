#[path = "support/run_fixture.rs"]
mod run_fixture;

use run_fixture::Fixture;
use std::fs;

fn editor_profile(path: &std::path::Path) {
    fs::write(
        path,
        r#"{
          "schemaVersion": 1,
          "description": "fixture editor",
          "useWhen": "fixture plan",
          "project": "main",
          "map": "/Game/Maps/Smoke",
          "backend": "editor",
          "editor": {
            "mode": "game",
            "rhi": "default",
            "window": {"mode": "windowed", "width": 800, "height": 600},
            "nativeArgs": []
          }
        }"#,
    )
    .expect("profile");
}

#[test]
fn configure_list_and_plan_reuse_the_same_profile_across_processes() {
    let fixture = Fixture::new();
    let candidate = fixture.temp.path().join("editor.json");
    editor_profile(&candidate);
    let candidate_text = candidate.to_string_lossy().to_string();

    let (configured, value) = fixture.run_json(&[
        "run",
        "configure",
        "fixture-game",
        "--workspace",
        "fixture",
        "--file",
        &candidate_text,
    ]);
    assert!(
        configured.status.success(),
        "{}",
        String::from_utf8_lossy(&configured.stderr)
    );
    assert_eq!(value["data"]["revision"], 1);
    assert_eq!(value["data"]["validationState"], "pending");

    let (listed, list) = fixture.run_json(&["run", "list", "--workspace", "fixture"]);
    assert!(listed.status.success());
    assert_eq!(list["data"]["configurations"][0]["name"], "fixture-game");
    assert_eq!(list["data"]["configurations"][0]["source"], "workspace");

    let (planned, plan) =
        fixture.run_json(&["run", "plan", "fixture-game", "--workspace", "fixture"]);
    assert!(planned.status.success());
    let expected_project = fixture
        .project
        .join("Fixture.uproject")
        .to_string_lossy()
        .replace('\\', "/");
    let actual_project = plan["data"]["resolvedTarget"]["project"]
        .as_str()
        .unwrap()
        .replace('\\', "/");
    let actual_argv_project = plan["data"]["nativeArgv"][0]
        .as_str()
        .unwrap()
        .replace('\\', "/");
    assert_eq!(actual_project, expected_project);
    assert_eq!(actual_argv_project, expected_project);
    assert_eq!(plan["data"]["nativeArgv"][1], "/Game/Maps/Smoke");
    assert!(plan["data"]["displayCommand"].is_string());
    assert!(!fixture.config_dir.join("executions/run").exists());
}

#[test]
fn check_reports_missing_map_without_starting_or_writing_execution() {
    let fixture = Fixture::new();
    let candidate = fixture.temp.path().join("missing-map.json");
    editor_profile(&candidate);
    let mut text = fs::read_to_string(&candidate).unwrap();
    text = text.replace("/Game/Maps/Smoke", "/Game/Maps/DoesNotExist");
    fs::write(&candidate, text).unwrap();
    let candidate_text = candidate.to_string_lossy().to_string();
    let _ = fixture.run_json(&[
        "run",
        "configure",
        "missing-map",
        "--workspace",
        "fixture",
        "--file",
        &candidate_text,
    ]);
    let (checked, value) =
        fixture.run_json(&["run", "check", "missing-map", "--workspace", "fixture"]);
    assert!(checked.status.success());
    assert_eq!(value["data"]["readiness"], "needsUserInput");
    assert!(
        value["data"]["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["code"] == "map_not_resolved")
    );
    assert!(!fixture.config_dir.join("executions/run").exists());
}

#[test]
fn scope_is_required_instead_of_guessing_a_workspace() {
    let fixture = Fixture::new();
    let output = fixture
        .command()
        .args(["--format", "json", "run", "list"])
        .output()
        .expect("run udf");
    assert_eq!(output.status.code(), Some(2));
}
