#![allow(dead_code)]

use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub struct Fixture {
    pub temp: TempDir,
    pub config_dir: PathBuf,
    pub project: PathBuf,
    pub engine: PathBuf,
    pub workspace: String,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let config_dir = temp.path().join("config");
        let project = temp.path().join("Project");
        let engine = temp.path().join("Engine");
        let hosts = temp.path().join("Hosts");
        let plugins = temp.path().join("Plugins");
        for path in [&config_dir, &project, &engine, &hosts, &plugins] {
            fs::create_dir_all(path).expect("fixture directory");
        }
        fs::create_dir_all(project.join("Content/Maps")).expect("content");
        fs::write(project.join("Fixture.uproject"), "{}").expect("uproject");
        fs::write(project.join("Content/Maps/Smoke.umap"), "fixture").expect("map");
        for path in [
            engine.join("Engine/Binaries/Win64/UnrealEditor.exe"),
            engine.join("Engine/Binaries/Win64/UnrealEditor-Cmd.exe"),
            engine.join("Engine/Build/BatchFiles/RunUAT.bat"),
        ] {
            fs::create_dir_all(path.parent().unwrap()).expect("engine binary directory");
            fs::write(path, "fixture").expect("engine binary");
        }
        let workspace = "fixture".to_string();
        let config = format!(
            "hosts_root = {hosts:?}\n\
             default_project = {project:?}\n\
             engine_path = {engine:?}\n\
             plugins_root = {plugins:?}\n\
             [workspaces.fixture]\n\
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
            temp,
            config_dir,
            project,
            engine,
            workspace,
        }
    }

    pub fn command(&self) -> Command {
        let mut command = Command::cargo_bin("udf").expect("udf binary");
        command.env("UNREALDEVFLOW_CONFIG_DIR", &self.config_dir);
        command
    }

    pub fn run_json(&self, args: &[&str]) -> (std::process::Output, Value) {
        let output = self
            .command()
            .args(["--format", "json"])
            .args(args)
            .output()
            .expect("run udf");
        let value = serde_json::from_slice(&output.stdout).expect("json output");
        (output, value)
    }
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
