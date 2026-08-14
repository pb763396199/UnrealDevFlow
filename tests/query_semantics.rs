use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    config_dir: PathBuf,
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
        fs::create_dir_all(project.join("Source")).expect("source dir");
        fs::write(project.join("Source/TestEditor.Target.cs"), "// fixture")
            .expect("target fixture");
        let build_bat = engine.join("Engine/Build/BatchFiles/Build.bat");
        fs::create_dir_all(build_bat.parent().unwrap()).expect("build dir");
        fs::write(&build_bat, "@echo off").expect("build fixture");
        let ubt = engine.join("Engine/Binaries/DotNET/UnrealBuildTool/UnrealBuildTool.dll");
        fs::create_dir_all(ubt.parent().unwrap()).expect("ubt dir");
        fs::write(ubt, "fixture").expect("ubt fixture");

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
fn build_check_returns_readiness_without_plan_or_execution_fields() {
    let fixture = Fixture::new();
    let checked = fixture.run_json(&["build", "check", "--workspace", "test"]);

    assert_eq!(checked["command"], "build check");
    assert_eq!(checked["data"]["domain"], "build");
    assert_eq!(checked["data"]["action"], "project");
    assert_eq!(checked["data"]["readiness"], "ready");
    assert!(checked["data"]["checks"].as_array().unwrap().len() >= 3);
    assert!(
        checked["data"]["nextCommand"]
            .as_str()
            .unwrap()
            .contains("udf build project")
    );
    assert!(checked["data"].get("planDigest").is_none());
    assert!(checked["data"].get("steps").is_none());
    assert!(checked["data"].get("executionId").is_none());
    assert!(!fixture.config_dir.join("executions/build/latest").exists());
}

#[test]
fn build_and_package_check_share_readiness_contract() {
    let fixture = Fixture::new();
    let build = fixture.run_json(&["build", "check", "--workspace", "test"]);
    let package = fixture.run_json(&["package", "check", "project", "--workspace", "test"]);

    for report in [&build["data"], &package["data"]] {
        assert!(report["readiness"].is_string());
        assert!(report["checks"].is_array());
        assert!(report["diagnostics"].is_array());
        assert!(report["nextCommand"].is_string());
        assert!(report.get("executionId").is_none());
    }
}

#[test]
fn build_plan_returns_steps_outputs_and_digest_without_readiness_or_execution_fields() {
    let fixture = Fixture::new();
    let planned = fixture.run_json(&["build", "plan", "--workspace", "test"]);

    assert_eq!(planned["command"], "build plan");
    assert_eq!(planned["data"]["domain"], "build");
    assert_eq!(planned["data"]["action"], "project");
    assert!(
        planned["data"]["planDigest"]
            .as_str()
            .unwrap()
            .starts_with("md5:")
    );
    assert!(!planned["data"]["steps"].as_array().unwrap().is_empty());
    assert!(!planned["data"]["outputs"].as_array().unwrap().is_empty());
    assert!(planned["data"].get("readiness").is_none());
    assert!(planned["data"].get("checks").is_none());
    assert!(planned["data"].get("executionId").is_none());
    assert!(!fixture.config_dir.join("executions/build/latest").exists());
}
