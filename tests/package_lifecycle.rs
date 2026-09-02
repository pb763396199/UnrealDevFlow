use assert_cmd::cargo::CommandCargoExt;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    config_dir: PathBuf,
    project: PathBuf,
    plugins: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let config_dir = temp.path().join("config");
        let project = temp.path().join("Project");
        let hosts = temp.path().join("Hosts");
        let engine = temp.path().join("Engine");
        let plugins = temp.path().join("Plugins");
        for path in [&config_dir, &project, &hosts, &engine, &plugins] {
            fs::create_dir_all(path).expect("fixture directory");
        }
        fs::write(project.join("Test.uproject"), "{}").expect("uproject");

        let config = format!(
            "hosts_root = {hosts:?}\n\
             default_project = {project:?}\n\
             engine_path = {engine:?}\n\
             plugins_root = {plugins:?}\n\
             last_used_workspace = \"test\"\n\n\
             [workspaces.test]\n\
             hosts_root = {hosts:?}\n\
             default_project = {project:?}\n\
             engine_path = {engine:?}\n\
             plugins_root = {plugins:?}\n",
            hosts = toml_path(&hosts),
            project = toml_path(&project),
            engine = toml_path(&engine),
            plugins = toml_path(&plugins),
        );
        fs::write(config_dir.join("config.toml"), config).expect("config");

        Self {
            _temp: temp,
            config_dir,
            project,
            plugins,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("udf").expect("udf binary");
        command.env("UNREALDEVFLOW_CONFIG_DIR", &self.config_dir);
        command
    }

    fn run_json(&self, args: &[&str]) -> Value {
        let output = self
            .command()
            .args(["--format", "json"])
            .args(args)
            .output()
            .expect("run udf");
        assert!(
            output.status.success(),
            "command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("json output")
    }

    fn run_failure_json(&self, args: &[&str]) -> Value {
        let output = self
            .command()
            .args(["--format", "json"])
            .args(args)
            .output()
            .expect("run udf");
        assert!(
            !output.status.success(),
            "command unexpectedly succeeded: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        serde_json::from_slice(&output.stdout).expect("failure json")
    }
}

#[test]
fn plugin_collection_expands_nested_uplugins_without_an_execution() {
    let fixture = Fixture::new();
    for relative in [
        "UnrealMCP/Core/Core.uplugin",
        "UnrealMCP/Tools/Tools.uplugin",
    ] {
        let descriptor = fixture.plugins.join(relative);
        fs::create_dir_all(descriptor.parent().unwrap()).unwrap();
        fs::write(descriptor, r#"{"FileVersion":3,"Plugins":[]}"#).unwrap();
    }

    let planned = fixture.run_json(&[
        "package",
        "advanced",
        "plugin",
        "--plan",
        "--workspace",
        "test",
        "--plugin",
        "UnrealMCP",
    ]);

    assert_eq!(planned["data"]["action"], "plugin");
    assert_eq!(planned["data"]["steps"].as_array().unwrap().len(), 6);
    let serialized = serde_json::to_string(&planned).unwrap();
    assert!(serialized.contains("UnrealMCP\\\\Core\\\\Core.uplugin"));
    assert!(serialized.contains("UnrealMCP\\\\Tools\\\\Tools.uplugin"));
    assert!(planned["data"].get("executionId").is_none());
}

#[test]
fn exact_plugin_package_ignores_unrelated_duplicate_named_descriptors() {
    let fixture = Fixture::new();
    for relative in [
        "AesWorld/AesWorld.uplugin",
        "UnrelatedBundle/AesWorld/AesWorld.uplugin",
    ] {
        let descriptor = fixture.plugins.join(relative);
        fs::create_dir_all(descriptor.parent().unwrap()).unwrap();
        fs::write(descriptor, r#"{"FileVersion":3,"Plugins":[]}"#).unwrap();
    }

    let planned = fixture.run_json(&[
        "package",
        "advanced",
        "plugin",
        "--plan",
        "--workspace",
        "test",
        "--plugin",
        "AesWorld",
    ]);

    assert_eq!(planned["data"]["action"], "plugin");
    let serialized = serde_json::to_string(&planned).unwrap();
    assert!(serialized.contains("AesWorld\\\\AesWorld.uplugin"));
    assert!(!serialized.contains("UnrelatedBundle\\\\AesWorld\\\\AesWorld.uplugin"));
}

#[test]
fn plugin_collection_rejects_duplicate_names_inside_collection_subtree() {
    let fixture = Fixture::new();
    for relative in [
        "UnrealMCP/Core/Core.uplugin",
        "UnrealMCP/Alternate/Core/Core.uplugin",
    ] {
        let descriptor = fixture.plugins.join(relative);
        fs::create_dir_all(descriptor.parent().unwrap()).unwrap();
        fs::write(descriptor, r#"{"FileVersion":3,"Plugins":[]}"#).unwrap();
    }

    let failed = fixture.run_failure_json(&[
        "package",
        "advanced",
        "plugin",
        "--plan",
        "--workspace",
        "test",
        "--plugin",
        "UnrealMCP",
    ]);

    assert_eq!(failed["ok"], false);
    assert!(failed["error"].as_str().unwrap().contains("Core"));
}

#[test]
fn plugin_package_rejects_dependency_names_with_multiple_project_candidates() {
    let fixture = Fixture::new();
    let plugin = fixture.plugins.join("AesWorld/AesWorld.uplugin");
    fs::create_dir_all(plugin.parent().unwrap()).unwrap();
    fs::write(
        plugin,
        r#"{"FileVersion":3,"Plugins":[{"Name":"SharedTool","Enabled":true}]}"#,
    )
    .unwrap();
    for relative in [
        "VendorA/SharedTool/SharedTool.uplugin",
        "VendorB/SharedTool/SharedTool.uplugin",
    ] {
        let descriptor = fixture.plugins.join(relative);
        fs::create_dir_all(descriptor.parent().unwrap()).unwrap();
        fs::write(descriptor, r#"{"FileVersion":3,"Plugins":[]}"#).unwrap();
    }

    let failed = fixture.run_failure_json(&[
        "package",
        "advanced",
        "plugin",
        "--plan",
        "--workspace",
        "test",
        "--plugin",
        "AesWorld",
    ]);

    assert_eq!(failed["ok"], false);
    assert!(failed["error"].as_str().unwrap().contains("SharedTool"));
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn plans_do_not_replace_latest_execution() {
    let fixture = Fixture::new();
    let planned = fixture.run_json(&["package", "plan", "project", "--workspace", "test"]);
    assert_eq!(planned["command"], "package plan");
    assert_eq!(planned["ok"], true);
    assert_eq!(planned["data"]["domain"], "package");
    assert_eq!(planned["data"]["action"], "project");
    assert!(planned["data"]["planDigest"].as_str().is_some());
    assert!(!planned["data"]["steps"].as_array().unwrap().is_empty());
    assert!(!planned["data"]["outputs"].as_array().unwrap().is_empty());
    assert!(planned["data"].get("executionId").is_none());
    assert!(planned["data"].get("state").is_none());
    assert!(
        !fixture
            .config_dir
            .join("executions/package/latest")
            .exists()
    );
}

#[test]
fn package_check_returns_readiness_without_creating_an_execution() {
    let fixture = Fixture::new();
    let checked = fixture.run_json(&["package", "check", "project", "--workspace", "test"]);
    assert_eq!(checked["command"], "package check");
    assert_eq!(checked["data"]["domain"], "package");
    assert_eq!(checked["data"]["action"], "project");
    assert_eq!(checked["data"]["readiness"], "blocked");
    assert!(checked["data"]["checks"].as_array().is_some());
    assert!(checked["data"]["nextCommand"].as_str().is_some());
    assert!(checked["data"].get("executionId").is_none());
    assert!(checked["data"].get("commands").is_none());
    assert!(
        !fixture
            .config_dir
            .join("executions/package/latest")
            .exists()
    );
}

#[test]
fn legacy_plugin_entrypoint_only_returns_migration_guidance() {
    let fixture = Fixture::new();
    let failed = fixture.run_failure_json(&["package", "plugin", "AesWorld"]);
    assert_eq!(failed["ok"], false);
    assert!(
        failed["error"]
            .as_str()
            .unwrap()
            .contains("package advanced plugin")
    );
    assert!(!fixture.config_dir.join("executions/package").exists());
}

#[test]
fn clean_without_scope_is_an_inventory_only_operation() {
    let fixture = Fixture::new();
    let report = fixture.run_json(&["package", "clean"]);
    assert_eq!(report["data"]["dryRun"], true);
    assert!(
        report["data"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str().unwrap().contains("未指定 execution ID"))
    );
}

#[test]
fn status_reads_only_real_executions() {
    let fixture = Fixture::new();
    let root = fixture.config_dir.join("executions/package");
    fs::create_dir_all(&root).unwrap();
    write_record(&root, "package-project-20260101T000000Z", "failed");
    write_record(&root, "package-project-20260102T000000Z", "planned");
    fs::write(root.join("latest"), "package-project-20260102T000000Z").unwrap();

    let latest = fixture.run_json(&["package", "status"]);
    assert_eq!(
        latest["data"]["executionId"],
        "package-project-20260101T000000Z"
    );
    assert_eq!(latest["data"]["state"], "failed");

    let legacy = fixture.run_json(&["package", "status", "package-project-20260102T000000Z"]);
    assert_eq!(legacy["data"]["state"], "planned");
}

fn write_record(root: &Path, execution_id: &str, state: &str) {
    fs::write(
        root.join(format!("{execution_id}.json")),
        serde_json::to_vec_pretty(&json!({
            "executionId": execution_id,
            "action": "project",
            "workspace": "test",
            "source": "workspace",
            "state": state,
            "commands": [],
            "outputs": [],
            "logs": [],
            "manifests": [],
            "cleanupTargets": [],
            "diagnostics": []
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn package_clean_refuses_a_recorded_path_outside_managed_artifacts() {
    let fixture = Fixture::new();
    let unsafe_output = fixture.project.join("DoNotDelete");
    fs::create_dir_all(&unsafe_output).expect("unsafe output fixture");
    fs::write(unsafe_output.join("marker.txt"), "keep").expect("marker");

    let execution_root = fixture.config_dir.join("executions/package");
    fs::create_dir_all(&execution_root).expect("execution root");
    let execution_id = "package-project-unsafe-test";
    fs::write(
        execution_root.join(format!("{execution_id}.json")),
        serde_json::to_vec_pretty(&json!({
            "executionId": execution_id,
            "action": "project",
            "workspace": "test",
            "source": "workspace",
            "state": "succeeded",
            "commands": [],
            "outputs": [unsafe_output],
            "logs": [],
            "manifests": [],
            "cleanupTargets": [unsafe_output]
        }))
        .expect("record json"),
    )
    .expect("record");

    let output = fixture
        .command()
        .args(["--format", "json", "package", "clean", execution_id])
        .output()
        .expect("run clean");
    assert!(!output.status.success(), "unsafe clean must fail");
    let response: Value = serde_json::from_slice(&output.stdout).expect("failure json");
    assert_eq!(response["command"], "package clean");
    assert_eq!(response["ok"], false);
    assert!(unsafe_output.join("marker.txt").is_file());
}

#[test]
fn failed_package_persists_execution_id_and_logs_for_status() {
    let fixture = Fixture::new();
    let output = fixture
        .command()
        .args([
            "--format",
            "json",
            "package",
            "project",
            "--workspace",
            "test",
        ])
        .output()
        .expect("run package");
    assert!(!output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).expect("failure json");
    let message = response["error"].as_str().expect("error message");
    assert!(message.contains("execution ID:"));

    let latest = fs::read_to_string(fixture.config_dir.join("executions/package/latest"))
        .expect("latest execution");
    let status = fixture.run_json(&["package", "status", latest.trim()]);
    assert_eq!(status["data"]["state"], "failed");
    assert_eq!(status["data"]["action"], "project");
    assert!(!status["data"]["logs"].as_array().unwrap().is_empty());
}

#[test]
fn recover_refuses_without_a_verified_delivery_journal() {
    let fixture = Fixture::new();
    let output = fixture
        .command()
        .args([
            "--format",
            "json",
            "package",
            "recover",
            "package-project-missing",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(response["error"].as_str().unwrap().contains("执行记录"));
}

#[test]
fn package_check_blocks_when_unreal_tools_are_missing() {
    let fixture = Fixture::new();
    let checked = fixture.run_json(&["package", "check", "project", "--workspace", "test"]);
    assert_eq!(checked["command"], "package check");
    assert_eq!(checked["data"]["readiness"], "blocked");
    assert!(
        checked["data"]["diagnostics"][0]
            .as_str()
            .unwrap()
            .contains("RunUAT.bat")
    );
}

#[test]
fn plugin_package_check_requires_the_ubt_dll_not_only_dotnet() {
    let fixture = Fixture::new();
    let dotnet = fixture
        ._temp
        .path()
        .join("Engine/Binaries/ThirdParty/DotNet/8.0.300/win-x64/dotnet.exe");
    fs::create_dir_all(dotnet.parent().unwrap()).unwrap();
    fs::write(&dotnet, "").unwrap();
    let plugin = fixture._temp.path().join("Plugins/AesWorld");
    fs::create_dir_all(&plugin).unwrap();
    fs::write(
        plugin.join("AesWorld.uplugin"),
        r#"{"FileVersion":3,"Plugins":[]}"#,
    )
    .unwrap();

    let checked = fixture.run_json(&[
        "package",
        "advanced",
        "plugin",
        "--check",
        "--workspace",
        "test",
        "--plugin",
        "AesWorld",
    ]);
    assert_eq!(checked["data"]["readiness"], "blocked");
    assert!(
        checked["data"]["diagnostics"][0]
            .as_str()
            .unwrap()
            .contains("UnrealBuildTool.dll")
    );
}

#[test]
fn build_plan_for_workspace_does_not_emit_engine_source_bootstrap() {
    let fixture = Fixture::new();
    let planned = fixture.run_json(&["build", "plan", "--workspace", "test"]);
    assert_eq!(planned["command"], "build plan");
    let serialized = serde_json::to_string(&planned).unwrap();
    assert!(!serialized.contains("GitDependencies"));
    assert!(!serialized.contains("GenerateProjectFiles"));
    assert!(serialized.contains("Test.uproject"));
}
