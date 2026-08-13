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
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn package_plan_status_share_one_persisted_lifecycle_and_plan_is_not_cleanable() {
    let fixture = Fixture::new();
    let planned = fixture.run_json(&["package", "plan", "project", "--workspace", "test"]);
    assert_eq!(planned["command"], "package plan");
    assert_eq!(planned["ok"], true);
    assert_eq!(planned["data"]["action"], "project");
    assert_eq!(planned["data"]["state"], "planned");
    assert_eq!(planned["data"]["source"], "workspace");
    let execution_id = planned["data"]["executionId"]
        .as_str()
        .expect("execution id");

    let record = fixture
        .config_dir
        .join("executions/package")
        .join(format!("{execution_id}.json"));
    assert!(record.is_file(), "plan must persist its execution record");

    let status = fixture.run_json(&["package", "status", execution_id]);
    assert_eq!(status["command"], "package status");
    assert_eq!(status["data"]["executionId"], execution_id);
    assert_eq!(status["data"]["state"], "planned");

    let output_dir = fixture.project.join("Saved/UnrealDevFlow/Packages/Win64");
    fs::create_dir_all(&output_dir).expect("planned output fixture");
    fs::write(output_dir.join("marker.txt"), "reproducible").expect("marker");

    let clean = fixture
        .command()
        .args(["--format", "json", "package", "clean", execution_id])
        .output()
        .expect("run clean");
    assert!(!clean.status.success(), "a plan owns no created artifacts");
    assert!(output_dir.join("marker.txt").is_file());
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
fn package_check_blocks_when_unreal_tools_are_missing() {
    let fixture = Fixture::new();
    let checked = fixture.run_json(&["package", "check", "project", "--workspace", "test"]);
    assert_eq!(checked["command"], "package check");
    assert_eq!(checked["data"]["state"], "blocked");
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
        "check",
        "plugin",
        "--workspace",
        "test",
        "--plugin",
        "AesWorld",
    ]);
    assert_eq!(checked["data"]["state"], "blocked");
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
