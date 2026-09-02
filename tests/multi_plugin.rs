use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const PLUGINS: [&str; 3] = ["AesWorld", "WdpCamera", "WdpAPI"];

fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_stdout(dir: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {:?} failed", args);
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

struct Fixture {
    _temp: TempDir,
    config_dir: PathBuf,
    hosts: PathBuf,
    project: PathBuf,
    repos: Vec<PathBuf>,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let config_dir = root.join("config");
        let hosts = root.join("Hosts");
        let project = root.join("DEV_2");
        let plugins_root = root.join("Plugins");
        let engine = root.join("UE_5.5");
        let build_bat = engine
            .join("Engine")
            .join("Build")
            .join("BatchFiles")
            .join("Build.bat");
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(project.join("Plugins")).unwrap();
        fs::create_dir_all(build_bat.parent().unwrap()).unwrap();
        fs::write(&build_bat, "").unwrap();
        fs::write(
            project.join("DEV_2.uproject"),
            r#"{"EngineAssociation":"5.5"}"#,
        )
        .unwrap();

        let mut repos = Vec::new();
        for name in PLUGINS {
            let repo = plugins_root.join(name);
            fs::create_dir_all(&repo).unwrap();
            let descriptor = if name == "WdpAPI" {
                let child = repo.join("WdpRuntimeAPI");
                fs::create_dir_all(&child).unwrap();
                child.join("WdpRuntimeAPI.uplugin")
            } else {
                repo.join(format!("{name}.uplugin"))
            };
            fs::write(
                descriptor,
                r#"{"FileVersion":3,"VersionName":"test","Plugins":[]}"#,
            )
            .unwrap();
            git(&repo, &["init"]);
            git(&repo, &["config", "user.name", "UnrealDevFlow Test"]);
            git(
                &repo,
                &["config", "user.email", "unrealdevflow-test@example.com"],
            );
            git(&repo, &["add", "."]);
            git(&repo, &["commit", "-m", "base"]);
            git(&repo, &["checkout", "-B", "dev"]);
            repos.push(repo);
        }

        let config = format!(
            r#"hosts_root = "{}"
default_project = "{}"
engine_path = "{}"
plugins_root = "{}"

[workspaces.test]
hosts_root = "{}"
default_project = "{}"
engine_path = "{}"
plugins_root = "{}"
"#,
            toml_path(&hosts),
            toml_path(&project),
            toml_path(&engine),
            toml_path(&plugins_root),
            toml_path(&hosts),
            toml_path(&project),
            toml_path(&engine),
            toml_path(&plugins_root),
        );
        fs::write(config_dir.join("config.toml"), config).unwrap();
        Self {
            _temp: temp,
            config_dir,
            hosts,
            project,
            repos,
        }
    }

    fn host(&self) -> PathBuf {
        self.hosts.join("W-test").join("T-multi_Host")
    }

    fn create(&self) {
        let wdp_api_override = format!("WdpAPI={}", self.repos[2].display());
        let mut command = Command::cargo_bin("udf").unwrap();
        command
            .env("UNREALDEVFLOW_CONFIG_DIR", &self.config_dir)
            .args([
                "task",
                "create",
                "three primary plugins",
                "--workspace",
                "test",
                "--id",
                "multi",
                "--primary",
                "AesWorld,WdpCamera,WdpAPI",
                "--branch",
                "feature/test",
                "--override-dep",
                &wdp_api_override,
                "--yes",
            ]);
        command.assert().success();
    }

    fn switch(&self) -> assert_cmd::assert::Assert {
        Command::cargo_bin("udf")
            .unwrap()
            .env("UNREALDEVFLOW_CONFIG_DIR", &self.config_dir)
            .args([
                "task",
                "switch",
                "test/multi",
                "--force",
                "--skip-regen-project-files",
            ])
            .assert()
    }
}

#[test]
fn create_three_primaries_uses_three_worktrees_and_explicit_shared_branch() {
    let f = Fixture::new();
    f.create();
    let meta: Value =
        serde_json::from_str(&fs::read_to_string(f.host().join(".udf-meta.json")).unwrap())
            .unwrap();
    let primaries = meta["primary_plugins"].as_array().unwrap();
    assert_eq!(primaries.len(), 3);
    for (index, name) in PLUGINS.iter().enumerate() {
        let worktree = f.host().join("Plugins").join(name);
        assert!(worktree.exists(), "missing worktree for {name}");
        assert_eq!(primaries[index]["name"], *name);
        assert_eq!(primaries[index]["branch"], "feature/test");
        assert_eq!(
            git_stdout(&worktree, &["branch", "--show-current"]),
            "feature/test"
        );
    }
}

#[test]
fn switch_conflict_is_atomic_and_leaves_all_existing_project_plugins_untouched() {
    let f = Fixture::new();
    f.create();
    for name in PLUGINS {
        let dir = f.project.join("Plugins").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("sentinel.txt"), name).unwrap();
    }
    f.switch()
        .failure()
        .stderr(predicate::str::contains("Switch aborted"));
    for name in PLUGINS {
        let dir = f.project.join("Plugins").join(name);
        assert_eq!(fs::read_to_string(dir.join("sentinel.txt")).unwrap(), name);
        assert!(!junction::exists(&dir).unwrap_or(false));
    }
    assert!(!f.config_dir.join("state.json").exists());
}

#[test]
fn switch_creates_three_junctions_and_delete_removes_every_resource_and_state_entry() {
    let f = Fixture::new();
    f.create();
    f.switch().success();
    for name in PLUGINS {
        let path = f.project.join("Plugins").join(name);
        assert!(
            junction::exists(&path).unwrap(),
            "missing junction for {name}"
        );
        assert_eq!(
            junction::get_target(&path).unwrap(),
            f.host().join("Plugins").join(name)
        );
    }
    let state: Value =
        serde_json::from_str(&fs::read_to_string(f.config_dir.join("state.json")).unwrap())
            .unwrap();
    assert_eq!(
        state["projects"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()["junctions"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    Command::cargo_bin("udf")
        .unwrap()
        .env("UNREALDEVFLOW_CONFIG_DIR", &f.config_dir)
        .args(["task", "delete", "test/multi", "--force"])
        .assert()
        .success();
    assert!(!f.host().exists());
    for (name, repo) in PLUGINS.iter().zip(&f.repos) {
        assert!(!f.project.join("Plugins").join(name).exists());
        let branches = git_stdout(repo, &["branch", "--list", "feature/test"]);
        assert!(branches.is_empty(), "branch remains in {name}");
        assert!(!repo.join("..").join("irrelevant").exists());
    }
    let state: Value =
        serde_json::from_str(&fs::read_to_string(f.config_dir.join("state.json")).unwrap())
            .unwrap();
    let project_state = state["projects"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .expect("project state retained after delete");
    assert!(
        project_state["active_task"].is_null(),
        "delete left an active task"
    );
    assert!(
        project_state["junctions"].as_array().unwrap().is_empty(),
        "delete left stale junction state"
    );
}

#[test]
fn cleanup_missing_host_removes_dangling_project_junctions_before_branch_cleanup() {
    let f = Fixture::new();
    f.create();
    f.switch().success();

    // Reproduce the reported failure: the Host/worktrees are removed first,
    // leaving project-side Junctions whose targets no longer exist.
    fs::remove_dir_all(f.host()).expect("remove task host to create dangling links");
    let aesworld = f.project.join("Plugins/AesWorld");
    assert!(
        !aesworld.exists(),
        "Path::exists follows the broken junction"
    );
    assert!(
        fs::symlink_metadata(&aesworld).is_ok(),
        "the broken Junction entry must still exist on disk"
    );

    Command::cargo_bin("udf")
        .unwrap()
        .env("UNREALDEVFLOW_CONFIG_DIR", &f.config_dir)
        .args(["task", "cleanup", "test/multi", "--force"])
        .assert()
        .success();

    for name in PLUGINS {
        let path = f.project.join("Plugins").join(name);
        assert!(
            fs::symlink_metadata(&path).is_err(),
            "cleanup left project Junction for {name}"
        );
    }
    assert!(
        !f.config_dir.join("state.json").exists() || {
            let state: Value =
                serde_json::from_str(&fs::read_to_string(f.config_dir.join("state.json")).unwrap())
                    .unwrap();
            state["projects"]
                .as_object()
                .unwrap()
                .values()
                .all(|project| project["junctions"].as_array().unwrap().is_empty())
        }
    );
}

#[test]
fn delete_removes_dangling_project_junctions_before_host_and_worktree_cleanup() {
    let f = Fixture::new();
    f.create();
    f.switch().success();

    // Only one worktree is gone, while the Host and the other worktrees still
    // exist. This isolates the ordering bug in the normal delete path.
    fs::remove_dir_all(f.host().join("Plugins/AesWorld"))
        .expect("remove one worktree to create a dangling project link");
    let aesworld = f.project.join("Plugins/AesWorld");
    assert!(!aesworld.exists());
    assert!(fs::symlink_metadata(&aesworld).is_ok());

    Command::cargo_bin("udf")
        .unwrap()
        .env("UNREALDEVFLOW_CONFIG_DIR", &f.config_dir)
        .args(["task", "delete", "test/multi", "--force"])
        .assert()
        .success();

    for name in PLUGINS {
        assert!(
            fs::symlink_metadata(f.project.join("Plugins").join(name)).is_err(),
            "delete left project Junction for {name}"
        );
    }
    assert!(!f.host().exists());
}

#[test]
fn merge_requires_selection_for_multi_primary_and_all_dry_run_is_reverse_order() {
    let f = Fixture::new();
    f.create();
    Command::cargo_bin("udf")
        .unwrap()
        .env("UNREALDEVFLOW_CONFIG_DIR", &f.config_dir)
        .args([
            "task",
            "merge",
            "test/multi",
            "--strategy",
            "rebase",
            "--dry-run",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Use --plugin <name> or --all"));

    let output = Command::cargo_bin("udf")
        .unwrap()
        .env("UNREALDEVFLOW_CONFIG_DIR", &f.config_dir)
        .args([
            "task",
            "merge",
            "test/multi",
            "--strategy",
            "rebase",
            "--all",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let api = stdout.find("plugin 'WdpAPI'").unwrap();
    let camera = stdout.find("plugin 'WdpCamera'").unwrap();
    let aes = stdout.find("plugin 'AesWorld'").unwrap();
    assert!(
        api < camera && camera < aes,
        "unexpected merge order:\n{stdout}"
    );
}
