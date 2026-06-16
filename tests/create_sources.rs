use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git");
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
        .expect("failed to run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn write_uplugin(plugin_dir: &Path) {
    fs::write(
        plugin_dir.join("AesWorld.uplugin"),
        r#"{
  "FileVersion": 3,
  "VersionName": "test",
  "Plugins": []
}"#,
    )
    .expect("uplugin");
}

fn write_uplugin_with_missing_dependency(plugin_dir: &Path) {
    fs::write(
        plugin_dir.join("AesWorld.uplugin"),
        r#"{
  "FileVersion": 3,
  "VersionName": "test",
  "Plugins": [
    { "Name": "MissingPlugin", "Enabled": true }
  ]
}"#,
    )
    .expect("uplugin");
}

fn write_workspace_config(config_dir: &Path, root: &Path, project: &Path, plugins_root: &Path) {
    fs::create_dir_all(config_dir).expect("config dir");
    let hosts_root = root.join("Hosts");
    let engine = root.join("UE_5.5");
    let build_bat = engine
        .join("Engine")
        .join("Build")
        .join("BatchFiles")
        .join("Build.bat");
    fs::create_dir_all(build_bat.parent().expect("Build.bat parent")).expect("engine dir");
    fs::write(&build_bat, "").expect("Build.bat");
    fs::create_dir_all(project).expect("project dir");
    fs::write(
        project.join("DEV.uproject"),
        r#"{"EngineAssociation":"5.5"}"#,
    )
    .expect("uproject");

    let config = format!(
        r#"hosts_root = "{}"
default_project = "{}"
engine_path = "{}"
plugins_root = "{}"

[workspaces.bad]
hosts_root = "{}"
default_project = "{}"
engine_path = "{}"
plugins_root = "{}"
"#,
        toml_path(&hosts_root),
        toml_path(project),
        toml_path(&engine),
        toml_path(plugins_root),
        toml_path(&hosts_root),
        toml_path(project),
        toml_path(&engine),
        toml_path(plugins_root),
    );
    fs::write(config_dir.join("config.toml"), config).expect("config");
}

fn setup_main_plugin_repo(root: &Path) -> PathBuf {
    let main_repo = root.join("Plugins").join("AesWorld");
    fs::create_dir_all(&main_repo).expect("main repo dir");
    write_uplugin(&main_repo);
    git(&main_repo, &["init"]);
    git(&main_repo, &["config", "user.name", "UnrealDevFlow Test"]);
    git(
        &main_repo,
        &["config", "user.email", "unrealdevflow-test@example.com"],
    );
    git(&main_repo, &["add", "."]);
    git(&main_repo, &["commit", "-m", "base"]);
    git(&main_repo, &["checkout", "-B", "dev"]);
    main_repo
}

#[test]
fn create_rejects_primary_source_that_resolves_to_linked_worktree() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let project = root.join("UGA").join("DEV");
    let active_plugins_root = root.join("ActivePlugins");
    let active_plugin = active_plugins_root.join("AesWorld");

    fs::create_dir_all(&active_plugins_root).expect("active plugins dir");
    let old_base = git_stdout(&main_repo, &["rev-parse", "HEAD"]);

    fs::write(main_repo.join("new-on-dev.txt"), "new\n").expect("new file");
    git(&main_repo, &["add", "new-on-dev.txt"]);
    git(&main_repo, &["commit", "-m", "new dev commit"]);

    git(
        &main_repo,
        &[
            "worktree",
            "add",
            "-b",
            "task-active",
            active_plugin.to_str().expect("active plugin path"),
            &old_base,
        ],
    );
    write_workspace_config(&config_dir, root, &project, &active_plugins_root);

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "create",
            "bad source",
            "--workspace",
            "bad",
            "--id",
            "bad-base",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("linked worktree"));

    assert!(
        !root
            .join("Hosts")
            .join("W-bad")
            .join("T-bad-base_Host")
            .exists()
    );
}

#[test]
fn create_rejects_plugins_root_inside_default_project() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let project = root.join("UGA").join("DEV");
    let project_plugins_root = project.join("Plugins");
    let project_plugin = project_plugins_root.join("AesWorld");
    fs::create_dir_all(&project_plugin).expect("project plugin dir");
    write_uplugin(&project_plugin);
    git(&project_plugin, &["init"]);
    git(
        &project_plugin,
        &["config", "user.name", "UnrealDevFlow Test"],
    );
    git(
        &project_plugin,
        &["config", "user.email", "unrealdevflow-test@example.com"],
    );
    git(&project_plugin, &["add", "."]);
    git(&project_plugin, &["commit", "-m", "base"]);
    git(&project_plugin, &["checkout", "-B", "dev"]);
    write_workspace_config(&config_dir, root, &project, &project_plugins_root);

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "start",
            "bad source",
            "--workspace",
            "bad",
            "--id",
            "bad-project-root",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "plugins_root 不能位于 UE 项目目录内",
        ));
}

#[test]
fn create_rejects_path_like_task_id_before_host_creation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "start",
            "bad id",
            "--workspace",
            "bad",
            "--id",
            "bad/id",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("task id 非法"));

    assert!(
        !root
            .join("Hosts")
            .join("W-bad")
            .join("T-bad")
            .join("id_Host")
            .exists()
    );
}

#[test]
fn create_rejects_primary_repo_when_current_branch_is_not_dev() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    git(&main_repo, &["checkout", "-b", "feature/current"]);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "start",
            "wrong branch",
            "--workspace",
            "bad",
            "--id",
            "wrong-branch",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("当前分支"));
}

#[test]
fn start_from_valid_main_repo_records_canonical_source_and_base() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let dev_head = git_stdout(&main_repo, &["rev-parse", "HEAD"]);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "start",
            "good task",
            "--workspace",
            "bad",
            "--id",
            "good-task",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success();

    let host_dir = root.join("Hosts").join("W-bad").join("T-good-task_Host");
    let meta_text = fs::read_to_string(host_dir.join(".udf-meta.json")).expect("meta");
    let meta: serde_json::Value = serde_json::from_str(&meta_text).expect("meta json");
    let primary = &meta["primary_plugins"][0];
    let expected_source = dunce::canonicalize(&main_repo).unwrap_or(main_repo);
    assert_eq!(
        primary["source_repo"].as_str().unwrap(),
        expected_source.to_string_lossy()
    );
    assert_eq!(primary["based_on"].as_str().unwrap(), dev_head);
    assert_eq!(meta["task_uid"].as_str().unwrap(), "bad/good-task");

    let worktree = host_dir.join("Plugins").join("AesWorld");
    assert_eq!(git_stdout(&worktree, &["rev-parse", "HEAD"]), dev_head);
    assert_eq!(
        git_stdout(&worktree, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "task/bad/good-task"
    );
}

#[test]
fn create_rejects_missing_dependencies_before_host_creation() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    write_uplugin_with_missing_dependency(&main_repo);
    git(&main_repo, &["add", "AesWorld.uplugin"]);
    git(&main_repo, &["commit", "-m", "add missing dependency"]);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "start",
            "missing dep",
            "--workspace",
            "bad",
            "--id",
            "missing-dep",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("依赖无法解析"));

    assert!(
        !root
            .join("Hosts")
            .join("W-bad")
            .join("T-missing-dep_Host")
            .exists()
    );
}

#[test]
fn create_rejects_duplicate_plugin_names_in_plugins_root() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let duplicate_plugin = main_repo
        .parent()
        .expect("plugins root")
        .join("Old")
        .join("AesWorld");
    fs::create_dir_all(&duplicate_plugin).expect("duplicate plugin dir");
    write_uplugin(&duplicate_plugin);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "start",
            "duplicate plugin",
            "--workspace",
            "bad",
            "--id",
            "duplicate-plugin",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("重复插件"));
}
