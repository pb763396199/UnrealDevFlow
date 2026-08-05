use assert_cmd::Command;
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

fn git_succeeds(dir: &Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run git")
        .status
        .success()
}

fn toml_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn write_test_config(root: &Path, config_dir: &Path, hosts_root: &Path, plugins_root: &Path) {
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
        toml_path(hosts_root),
        toml_path(&root.join("Project")),
        toml_path(&root.join("Engine")),
        toml_path(plugins_root),
        toml_path(hosts_root),
        toml_path(&root.join("Project")),
        toml_path(&root.join("Engine")),
        toml_path(plugins_root),
    );
    fs::write(config_dir.join("config.toml"), config).expect("config");
}

#[derive(Clone, Copy)]
struct TestContext<'a> {
    root: &'a Path,
    hosts_root: &'a Path,
    plugins_root: &'a Path,
}

fn write_test_meta(
    host_dir: &Path,
    context: TestContext<'_>,
    source_repo: &Path,
    task_id: &str,
    branch: &str,
    based_on: &str,
) {
    let meta = serde_json::json!({
        "schema_version": 3,
        "id": task_id,
        "name": task_id,
        "branch": branch,
        "created": "2026-06-15T00:00:00Z",
        "based_on": based_on,
        "status": "created",
        "primary_plugins": [{
            "name": "AesWorld",
            "source_repo": source_repo,
            "worktree": "Plugins/AesWorld",
            "branch": branch,
            "based_on": based_on,
        }],
        "dependency_plugins": [],
        "workspace": "test",
        "task_uid": format!("test/{task_id}"),
        "context": {
            "workspace": "test",
            "hosts_root": context.hosts_root,
            "default_project": context.root.join("Project"),
            "engine_path": context.root.join("Engine"),
            "plugins_root": context.plugins_root,
            "plugin_overrides": {}
        }
    });
    fs::write(
        host_dir.join(".udf-meta.json"),
        serde_json::to_string_pretty(&meta).expect("meta json"),
    )
    .expect("meta");
}

fn setup_repo_with_base(source_repo: &Path) -> String {
    fs::create_dir_all(source_repo).expect("source repo dir");
    git(source_repo, &["init"]);
    git(source_repo, &["config", "user.name", "UnrealDevFlow Test"]);
    git(
        source_repo,
        &["config", "user.email", "unrealdevflow-test@example.com"],
    );
    fs::write(source_repo.join("base.txt"), "base\n").expect("base file");
    git(source_repo, &["add", "base.txt"]);
    git(source_repo, &["commit", "-m", "base"]);
    git(source_repo, &["checkout", "-B", "dev"]);
    git_stdout(source_repo, &["rev-parse", "HEAD"])
}

#[test]
fn merge_yes_skips_confirmation_without_force() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-merge-yes_Host");
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&source_repo).expect("source repo dir");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(host_dir.join("Plugins")).expect("host plugin dir");

    git(&source_repo, &["init"]);
    git(&source_repo, &["config", "user.name", "UnrealDevFlow Test"]);
    git(
        &source_repo,
        &["config", "user.email", "unrealdevflow-test@example.com"],
    );
    fs::write(source_repo.join("tracked.txt"), "base\n").expect("base file");
    git(&source_repo, &["add", "tracked.txt"]);
    git(&source_repo, &["commit", "-m", "base"]);
    git(&source_repo, &["checkout", "-B", "dev"]);
    let based_on = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    git(
        &source_repo,
        &[
            "worktree",
            "add",
            "-b",
            "task-merge-yes",
            worktree.to_str().expect("worktree path"),
            "HEAD",
        ],
    );
    fs::write(worktree.join("tracked.txt"), "base\ntask\n").expect("task file");
    git(&worktree, &["add", "tracked.txt"]);
    git(&worktree, &["commit", "-m", "task change"]);

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
        toml_path(&hosts_root),
        toml_path(&root.join("Project")),
        toml_path(&root.join("Engine")),
        toml_path(&plugins_root),
        toml_path(&hosts_root),
        toml_path(&root.join("Project")),
        toml_path(&root.join("Engine")),
        toml_path(&plugins_root),
    );
    fs::write(config_dir.join("config.toml"), config).expect("config");

    let meta = serde_json::json!({
        "schema_version": 3,
        "id": "merge-yes",
        "name": "merge yes",
        "branch": "task-merge-yes",
        "created": "2026-06-15T00:00:00Z",
        "based_on": based_on,
        "status": "created",
        "primary_plugins": [{
            "name": "AesWorld",
            "source_repo": source_repo,
            "worktree": "Plugins/AesWorld",
            "branch": "task-merge-yes",
            "based_on": based_on,
        }],
        "dependency_plugins": [],
        "workspace": "test",
        "task_uid": "test/merge-yes",
        "context": {
            "workspace": "test",
            "hosts_root": hosts_root,
            "default_project": root.join("Project"),
            "engine_path": root.join("Engine"),
            "plugins_root": plugins_root,
            "plugin_overrides": {}
        }
    });
    fs::write(
        host_dir.join(".udf-meta.json"),
        serde_json::to_string_pretty(&meta).expect("meta json"),
    )
    .expect("meta");

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/merge-yes",
            "--plugin",
            "AesWorld",
            "--strategy",
            "ff-only",
            "-y",
        ])
        .assert()
        .success();

    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "dev"]),
        git_stdout(&worktree, &["rev-parse", "HEAD"])
    );
}

#[test]
fn cleanup_yes_skips_confirmation_without_force() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-cleanup-yes_Host");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&host_dir).expect("host dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(&source_repo, &["branch", "task-cleanup-yes"]);
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "cleanup-yes",
        "task-cleanup-yes",
        &based_on,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "cleanup", "test/cleanup-yes", "-y"])
        .assert()
        .success();

    assert!(!host_dir.exists());
    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "task-cleanup-yes"]),
        ""
    );
}

#[test]
fn cleanup_force_skips_confirmation_without_yes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-cleanup-force_Host");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&host_dir).expect("host dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(&source_repo, &["branch", "task-cleanup-force"]);
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "cleanup-force",
        "task-cleanup-force",
        &based_on,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "cleanup", "test/cleanup-force", "--force"])
        .assert()
        .success();

    assert!(!host_dir.exists());
    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "task-cleanup-force"]),
        ""
    );
}

#[test]
fn cleanup_removes_worktree_before_deleting_branch_and_host() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-cleanup-worktree_Host");
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(host_dir.join("Plugins")).expect("host plugin dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(
        &source_repo,
        &[
            "worktree",
            "add",
            "-b",
            "task-cleanup-worktree",
            worktree.to_str().expect("worktree path"),
            "HEAD",
        ],
    );
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "cleanup-worktree",
        "task-cleanup-worktree",
        &based_on,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "cleanup", "test/cleanup-worktree", "--force"])
        .assert()
        .success();

    assert!(!host_dir.exists());
    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "task-cleanup-worktree"]),
        ""
    );
    assert!(
        !git_stdout(&source_repo, &["worktree", "list", "--porcelain"])
            .contains("T-cleanup-worktree_Host")
    );
}

#[test]
fn cleanup_missing_host_deletes_residual_local_task_branch() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");

    setup_repo_with_base(&source_repo);
    git(&source_repo, &["branch", "task-orphan-cleanup"]);
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "cleanup", "test/orphan-cleanup", "--force"])
        .assert()
        .success();

    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "task-orphan-cleanup"]),
        ""
    );
}

#[test]
fn delete_force_skips_double_confirmation_for_unmerged_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-delete-force_Host");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&host_dir).expect("host dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(&source_repo, &["checkout", "-b", "task-delete-force"]);
    fs::write(source_repo.join("task.txt"), "task\n").expect("task file");
    git(&source_repo, &["add", "task.txt"]);
    git(&source_repo, &["commit", "-m", "unmerged task"]);
    git(&source_repo, &["checkout", "dev"]);
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "delete-force",
        "task-delete-force",
        &based_on,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "delete", "test/delete-force", "--force"])
        .assert()
        .success();

    assert!(!host_dir.exists());
    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "task-delete-force"]),
        ""
    );
}

#[test]
fn delete_removes_broken_non_git_worktree_residue() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-broken-worktree_Host");
    let broken_worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(broken_worktree.join("Source")).expect("broken worktree source");

    let based_on = setup_repo_with_base(&source_repo);
    fs::write(
        broken_worktree.join("Source").join("leftover.cpp"),
        "// residue\n",
    )
    .expect("leftover file");
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "broken-worktree",
        "task-broken-worktree",
        &based_on,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "delete", "test/broken-worktree", "--yes", "--force"])
        .assert()
        .success();

    assert!(!host_dir.exists());
    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "task-broken-worktree"]),
        ""
    );
}

#[test]
fn delete_removes_worktree_before_deleting_branch_and_host() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-delete-worktree_Host");
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(host_dir.join("Plugins")).expect("host plugin dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(
        &source_repo,
        &[
            "worktree",
            "add",
            "-b",
            "task-delete-worktree",
            worktree.to_str().expect("worktree path"),
            "HEAD",
        ],
    );
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "delete-worktree",
        "task-delete-worktree",
        &based_on,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "delete", "test/delete-worktree", "--force"])
        .assert()
        .success();

    assert!(!host_dir.exists());
    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "task-delete-worktree"]),
        ""
    );
    assert!(
        !git_stdout(&source_repo, &["worktree", "list", "--porcelain"])
            .contains("T-delete-worktree_Host")
    );
}

#[test]
fn rebase_merge_preserves_existing_target_commits() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-rebase-preserve_Host");
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&source_repo).expect("source repo dir");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(host_dir.join("Plugins")).expect("host plugin dir");

    git(&source_repo, &["init"]);
    git(&source_repo, &["config", "user.name", "UnrealDevFlow Test"]);
    git(
        &source_repo,
        &["config", "user.email", "unrealdevflow-test@example.com"],
    );
    fs::write(source_repo.join("base.txt"), "base\n").expect("base file");
    git(&source_repo, &["add", "base.txt"]);
    git(&source_repo, &["commit", "-m", "base"]);
    git(&source_repo, &["checkout", "-B", "dev"]);
    let based_on = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    git(
        &source_repo,
        &[
            "worktree",
            "add",
            "-b",
            "task-rebase-preserve",
            worktree.to_str().expect("worktree path"),
            "HEAD",
        ],
    );

    fs::write(source_repo.join("dev.txt"), "dev\n").expect("dev file");
    git(&source_repo, &["add", "dev.txt"]);
    git(&source_repo, &["commit", "-m", "dev commit must keep id"]);
    let dev_commit = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    fs::write(worktree.join("task.txt"), "task\n").expect("task file");
    git(&worktree, &["add", "task.txt"]);
    git(&worktree, &["commit", "-m", "task commit"]);
    let original_task_tip = git_stdout(&worktree, &["rev-parse", "HEAD"]);

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
        toml_path(&hosts_root),
        toml_path(&root.join("Project")),
        toml_path(&root.join("Engine")),
        toml_path(&plugins_root),
        toml_path(&hosts_root),
        toml_path(&root.join("Project")),
        toml_path(&root.join("Engine")),
        toml_path(&plugins_root),
    );
    fs::write(config_dir.join("config.toml"), config).expect("config");

    let meta = serde_json::json!({
        "schema_version": 3,
        "id": "rebase-preserve",
        "name": "rebase preserve",
        "branch": "task-rebase-preserve",
        "created": "2026-06-15T00:00:00Z",
        "based_on": based_on,
        "status": "created",
        "primary_plugins": [{
            "name": "AesWorld",
            "source_repo": source_repo,
            "worktree": "Plugins/AesWorld",
            "branch": "task-rebase-preserve",
            "based_on": based_on,
        }],
        "dependency_plugins": [],
        "workspace": "test",
        "task_uid": "test/rebase-preserve",
        "context": {
            "workspace": "test",
            "hosts_root": hosts_root,
            "default_project": root.join("Project"),
            "engine_path": root.join("Engine"),
            "plugins_root": plugins_root,
            "plugin_overrides": {}
        }
    });
    fs::write(
        host_dir.join(".udf-meta.json"),
        serde_json::to_string_pretty(&meta).expect("meta json"),
    )
    .expect("meta");

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/rebase-preserve",
            "--plugin",
            "AesWorld",
            "--strategy",
            "rebase",
            "-y",
        ])
        .assert()
        .success();

    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "dev~1"]),
        dev_commit
    );
    assert_ne!(
        git_stdout(&source_repo, &["rev-parse", "dev"]),
        original_task_tip
    );
    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "task-rebase-preserve"]),
        original_task_tip
    );
    assert_eq!(
        git_stdout(&worktree, &["rev-parse", "HEAD"]),
        original_task_tip
    );
}

#[test]
fn rebase_merge_uses_actual_merge_base_when_metadata_base_is_stale_in_target() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-stale-base_Host");
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(host_dir.join("Plugins")).expect("host plugin dir");

    let stale_metadata_base = setup_repo_with_base(&source_repo);

    fs::write(source_repo.join("shared.txt"), "shared one\n").expect("shared file");
    git(&source_repo, &["add", "shared.txt"]);
    git(&source_repo, &["commit", "-m", "shared one"]);
    let actual_task_base = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    git(
        &source_repo,
        &[
            "worktree",
            "add",
            "-b",
            "task-stale-base",
            worktree.to_str().expect("worktree path"),
            &actual_task_base,
        ],
    );

    fs::write(worktree.join("task.txt"), "task\n").expect("task file");
    git(&worktree, &["add", "task.txt"]);
    git(&worktree, &["commit", "-m", "task only"]);
    let original_task_tip = git_stdout(&worktree, &["rev-parse", "HEAD"]);

    fs::write(source_repo.join("shared.txt"), "shared two\n").expect("shared file");
    git(&source_repo, &["add", "shared.txt"]);
    git(&source_repo, &["commit", "-m", "shared two"]);
    let target_tip = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "stale-base",
        "task-stale-base",
        &stale_metadata_base,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/stale-base",
            "--plugin",
            "AesWorld",
            "--strategy",
            "rebase",
            "-y",
        ])
        .assert()
        .success();

    assert_eq!(
        git_stdout(
            &source_repo,
            &["log", "--reverse", "--format=%s", "dev~1..dev"]
        ),
        "task only"
    );
    assert_eq!(
        fs::read_to_string(source_repo.join("shared.txt")).expect("shared file"),
        "shared two\n"
    );
    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "dev~1"]),
        target_tip
    );
    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "task-stale-base"]),
        original_task_tip
    );
    assert_eq!(
        git_stdout(&worktree, &["rev-parse", "HEAD"]),
        original_task_tip
    );
}

#[test]
fn rebase_merge_replays_from_actual_merge_base_when_based_on_is_not_in_target() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-feature-rebase_Host");
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(host_dir.join("Plugins")).expect("host plugin dir");

    setup_repo_with_base(&source_repo);
    git(&source_repo, &["checkout", "-b", "feature/sublevels"]);

    fs::write(source_repo.join("feature-l1.txt"), "feature 1\n").expect("feature file");
    git(&source_repo, &["add", "feature-l1.txt"]);
    git(&source_repo, &["commit", "-m", "feature level 1"]);

    fs::write(source_repo.join("feature-l2.txt"), "feature 2\n").expect("feature file");
    git(&source_repo, &["add", "feature-l2.txt"]);
    git(&source_repo, &["commit", "-m", "feature level 2"]);
    let feature_tip = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    git(
        &source_repo,
        &[
            "worktree",
            "add",
            "-b",
            "task-feature-rebase",
            worktree.to_str().expect("worktree path"),
            &feature_tip,
        ],
    );

    fs::write(worktree.join("task-1.txt"), "task 1\n").expect("task file");
    git(&worktree, &["add", "task-1.txt"]);
    git(&worktree, &["commit", "-m", "task commit 1"]);

    fs::write(worktree.join("task-2.txt"), "task 2\n").expect("task file");
    git(&worktree, &["add", "task-2.txt"]);
    git(&worktree, &["commit", "-m", "task commit 2"]);
    let original_task_tip = git_stdout(&worktree, &["rev-parse", "HEAD"]);

    git(&source_repo, &["checkout", "dev"]);
    fs::write(source_repo.join("dev.txt"), "dev\n").expect("dev file");
    git(&source_repo, &["add", "dev.txt"]);
    git(&source_repo, &["commit", "-m", "dev commit must keep id"]);
    let dev_commit = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "feature-rebase",
        "task-feature-rebase",
        &feature_tip,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/feature-rebase",
            "--plugin",
            "AesWorld",
            "--strategy",
            "rebase",
            "-y",
        ])
        .assert()
        .success();

    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "dev~4"]),
        dev_commit
    );
    assert_eq!(
        git_stdout(
            &source_repo,
            &["log", "--reverse", "--format=%s", "dev~4..dev"]
        )
        .lines()
        .collect::<Vec<_>>(),
        vec![
            "feature level 1",
            "feature level 2",
            "task commit 1",
            "task commit 2",
        ]
    );
    assert!(source_repo.join("feature-l1.txt").exists());
    assert!(source_repo.join("feature-l2.txt").exists());
    assert!(source_repo.join("task-1.txt").exists());
    assert!(source_repo.join("task-2.txt").exists());
    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "feature/sublevels"]),
        feature_tip
    );
    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "task-feature-rebase"]),
        original_task_tip
    );
    assert_eq!(
        git_stdout(&worktree, &["rev-parse", "HEAD"]),
        original_task_tip
    );
    assert_ne!(
        git_stdout(&source_repo, &["rev-parse", "dev"]),
        original_task_tip
    );
}

#[test]
fn rebase_merge_restores_target_tip_after_replay_conflict() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-conflict-rebase_Host");
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(host_dir.join("Plugins")).expect("host plugin dir");

    setup_repo_with_base(&source_repo);
    git(&source_repo, &["checkout", "-b", "feature/sublevels"]);
    fs::write(source_repo.join("base.txt"), "feature\n").expect("feature file");
    git(&source_repo, &["add", "base.txt"]);
    git(
        &source_repo,
        &["commit", "-m", "feature conflicting change"],
    );
    let feature_tip = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    git(
        &source_repo,
        &[
            "worktree",
            "add",
            "-b",
            "task-conflict-rebase",
            worktree.to_str().expect("worktree path"),
            &feature_tip,
        ],
    );
    fs::write(worktree.join("task.txt"), "task\n").expect("task file");
    git(&worktree, &["add", "task.txt"]);
    git(&worktree, &["commit", "-m", "task commit"]);
    let original_task_tip = git_stdout(&worktree, &["rev-parse", "HEAD"]);

    git(&source_repo, &["checkout", "dev"]);
    fs::write(source_repo.join("base.txt"), "dev\n").expect("dev file");
    git(&source_repo, &["add", "base.txt"]);
    git(&source_repo, &["commit", "-m", "dev conflicting change"]);
    let dev_commit = git_stdout(&source_repo, &["rev-parse", "HEAD"]);

    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "conflict-rebase",
        "task-conflict-rebase",
        &feature_tip,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/conflict-rebase",
            "--plugin",
            "AesWorld",
            "--strategy",
            "rebase",
            "-y",
        ])
        .assert()
        .failure();

    assert_eq!(git_stdout(&source_repo, &["rev-parse", "dev"]), dev_commit);
    assert_eq!(git_stdout(&source_repo, &["status", "--porcelain"]), "");
    assert!(!git_succeeds(
        &source_repo,
        &["rev-parse", "--verify", "CHERRY_PICK_HEAD"]
    ));
    assert_eq!(
        git_stdout(&source_repo, &["rev-parse", "task-conflict-rebase"]),
        original_task_tip
    );
    assert_eq!(
        git_stdout(&worktree, &["rev-parse", "HEAD"]),
        original_task_tip
    );
}

#[test]
fn cleanup_finds_an_orphan_branch_whose_name_says_nothing_about_the_task() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");

    setup_repo_with_base(&source_repo);
    // 名字里没有 task-id，也不符合任何命名规则；只有台账知道它属于谁
    git(&source_repo, &["branch", "release/unrelated-name"]);
    git(
        &source_repo,
        &[
            "config",
            "branch.release/unrelated-name.udftask",
            "test/ledger-orphan",
        ],
    );
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);

    // Host 从来没建过，等于已经丢失
    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "cleanup", "test/ledger-orphan", "--force"])
        .assert()
        .success();

    assert_eq!(
        git_stdout(
            &source_repo,
            &["branch", "--list", "release/unrelated-name"]
        ),
        "",
        "台账指名的分支应该被清掉"
    );
}

#[test]
fn merge_accepts_a_branch_the_ledger_vouches_for() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-ledger-merge_Host");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&host_dir).expect("host dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(
        &source_repo,
        &["checkout", "-b", "hotfix/nothing-in-common"],
    );
    fs::write(source_repo.join("task.txt"), "task\n").expect("task file");
    git(&source_repo, &["add", "task.txt"]);
    git(&source_repo, &["commit", "-m", "task work"]);
    git(&source_repo, &["checkout", "dev"]);
    git(
        &source_repo,
        &[
            "config",
            "branch.hotfix/nothing-in-common.udftask",
            "test/ledger-merge",
        ],
    );
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "ledger-merge",
        "hotfix/nothing-in-common",
        &based_on,
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/ledger-merge",
            "--strategy",
            "rebase",
            "--yes",
        ])
        .assert()
        .success();
}

#[test]
fn merge_still_refuses_a_branch_nothing_vouches_for() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-ledger-refuse_Host");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&host_dir).expect("host dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(&source_repo, &["branch", "hotfix/someone-elses-work"]);
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "ledger-refuse",
        "hotfix/ledger-refuse",
        &based_on,
    );

    // 让插件记的分支跟任务记的分开：名字不含 task-id、不是 task/ 形态、台账里也没有。
    // 四条判定全不成立，merge 必须停下来。
    let meta_path = host_dir.join(".udf-meta.json");
    let mut meta: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&meta_path).expect("read meta")).expect("meta");
    meta["primary_plugins"][0]["branch"] = serde_json::json!("hotfix/someone-elses-work");
    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).expect("meta json"),
    )
    .expect("write meta");

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/ledger-refuse",
            "--strategy",
            "rebase",
            "--yes",
        ])
        .assert()
        .failure();
}

#[test]
fn cleanup_reaches_the_ledger_even_when_the_task_is_named_without_its_workspace() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");

    setup_repo_with_base(&source_repo);
    git(&source_repo, &["branch", "release/bare-custom"]);
    git(
        &source_repo,
        &["config", "branch.release/bare-custom.udftask", "test/bare"],
    );
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);

    // 裸 task-id，不写 workspace/ 前缀
    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "cleanup", "bare", "--force"])
        .assert()
        .success();

    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "release/bare-custom"]),
        "",
        "裸引用也该查得到台账"
    );
}

#[test]
fn merge_refuses_a_branch_the_ledger_assigns_to_another_workspace() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-shared-id_Host");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&host_dir).expect("host dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(&source_repo, &["branch", "feature/other-workspace-work"]);
    // 台账说这个分支属于【别的 workspace】里同名的任务
    git(
        &source_repo,
        &[
            "config",
            "branch.feature/other-workspace-work.udftask",
            "otherws/shared-id",
        ],
    );
    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "shared-id",
        "feature/shared-id",
        &based_on,
    );

    let meta_path = host_dir.join(".udf-meta.json");
    let mut meta: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&meta_path).expect("read meta")).expect("meta");
    meta["primary_plugins"][0]["branch"] = serde_json::json!("feature/other-workspace-work");
    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta).expect("json"),
    )
    .expect("write meta");

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/shared-id",
            "--strategy",
            "rebase",
            "--yes",
        ])
        .assert()
        .failure();
}

#[test]
fn merge_strategy_leaves_the_source_repo_index_matching_head() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join("T-index-probe_Host");
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");
    fs::create_dir_all(&host_dir).expect("host dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(
        &source_repo,
        &[
            "worktree",
            "add",
            &worktree.to_string_lossy(),
            &based_on,
            "-b",
            "feature/index-probe",
        ],
    );
    fs::write(worktree.join("feature.txt"), "new work\n").expect("feature file");
    git(&worktree, &["add", "-A"]);
    git(&worktree, &["commit", "-m", "feat: 加一个文件"]);

    write_test_config(root, &config_dir, &hosts_root, &plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root: &hosts_root,
            plugins_root: &plugins_root,
        },
        &source_repo,
        "index-probe",
        "feature/index-probe",
        &based_on,
    );

    assert_eq!(
        git_stdout(&source_repo, &["status", "--porcelain"]),
        "",
        "前提：主仓库一开始是干净的"
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/index-probe",
            "--strategy",
            "merge",
            "--yes",
        ])
        .assert()
        .success();

    // 合并把 feature.txt 带进了 HEAD。索引和工作区必须跟着走，否则用户会看到
    // 一堆自己没动过的暂存改动，而且下一次 task create 会被干净工作区检查拦下。
    assert_eq!(
        git_stdout(&source_repo, &["status", "--porcelain"]),
        "",
        "合并之后主仓库的暂存区不该有东西"
    );
    assert!(
        source_repo.join("feature.txt").is_file(),
        "合并带进来的文件必须真的出现在工作区里"
    );
}

/// 主仓库跑完一条命令之后，暂存区和工作区必须跟跑之前一样。
///
/// 唯一允许的差异是 HEAD 前进（合并本来就该产生提交）和分支增减。留下未提交的
/// 改动就是污染——用户会看到自己没做过的暂存内容，下一次 task create 也会被
/// 干净工作区检查拦下。
fn assert_source_repo_undisturbed(source_repo: &Path, what: &str) {
    let status = git_stdout(source_repo, &["status", "--porcelain"]);
    assert_eq!(status, "", "{} 之后主仓库不该有未提交的改动", what);
    let unmerged = git_stdout(source_repo, &["ls-files", "--unmerged"]);
    assert_eq!(unmerged, "", "{} 之后主仓库不该有未合并的条目", what);
    assert!(
        !source_repo.join(".git").join("MERGE_HEAD").exists(),
        "{} 之后主仓库不该停在 MERGING 状态",
        what
    );
    assert!(
        !source_repo.join(".git").join("CHERRY_PICK_HEAD").exists(),
        "{} 之后主仓库不该停在 cherry-pick 中途",
        what
    );
}

fn task_with_one_commit(
    root: &Path,
    config_dir: &Path,
    hosts_root: &Path,
    plugins_root: &Path,
    task_id: &str,
    branch: &str,
    body: &str,
) -> PathBuf {
    let source_repo = plugins_root.join("AesWorld");
    let host_dir = hosts_root.join("W-test").join(format!("T-{task_id}_Host"));
    let worktree = host_dir.join("Plugins").join("AesWorld");
    fs::create_dir_all(&host_dir).expect("host dir");

    let based_on = setup_repo_with_base(&source_repo);
    git(
        &source_repo,
        &[
            "worktree",
            "add",
            &worktree.to_string_lossy(),
            &based_on,
            "-b",
            branch,
        ],
    );
    fs::write(worktree.join("probe.txt"), body).expect("probe file");
    git(&worktree, &["add", "-A"]);
    git(&worktree, &["commit", "-m", "feat: probe"]);

    write_test_config(root, config_dir, hosts_root, plugins_root);
    write_test_meta(
        &host_dir,
        TestContext {
            root,
            hosts_root,
            plugins_root,
        },
        &source_repo,
        task_id,
        branch,
        &based_on,
    );
    source_repo
}

fn merge_with_strategy(config_dir: &Path, task_ref: &str, strategy: &str) {
    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", config_dir)
        .args(["task", "merge", task_ref, "--strategy", strategy, "--yes"])
        .assert()
        .success();
}

#[test]
fn every_merge_strategy_leaves_the_source_repo_undisturbed() {
    for strategy in ["merge", "squash", "rebase", "ff-only"] {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let config_dir = root.join("config");
        let hosts_root = root.join("Hosts");
        let plugins_root = root.join("Plugins");
        fs::create_dir_all(&config_dir).expect("config dir");

        let task_id = format!("probe-{}", strategy.replace('-', ""));
        let source_repo = task_with_one_commit(
            root,
            &config_dir,
            &hosts_root,
            &plugins_root,
            &task_id,
            &format!("feature/{task_id}"),
            "probe\n",
        );

        assert_source_repo_undisturbed(&source_repo, "建任务");
        merge_with_strategy(&config_dir, &format!("test/{task_id}"), strategy);
        assert_source_repo_undisturbed(&source_repo, &format!("--strategy {strategy}"));
    }
}

#[test]
fn a_conflicting_squash_does_not_strand_the_source_repo() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    fs::create_dir_all(&config_dir).expect("config dir");

    let source_repo = task_with_one_commit(
        root,
        &config_dir,
        &hosts_root,
        &plugins_root,
        "clash",
        "feature/clash",
        "from task\n",
    );

    // 主分支上把同一个文件改成别的内容，squash 必然冲突
    fs::write(source_repo.join("probe.txt"), "from dev\n").expect("dev file");
    git(&source_repo, &["add", "-A"]);
    git(&source_repo, &["commit", "-m", "dev writes probe"]);

    // 冲突时命令本身失败是对的，重点是失败之后仓库不能留残骸
    let _ = Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "merge",
            "test/clash",
            "--strategy",
            "squash",
            "--yes",
        ])
        .assert();

    assert_source_repo_undisturbed(&source_repo, "冲突的 squash");
}

#[test]
fn creating_and_deleting_a_task_leaves_the_source_repo_undisturbed() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let hosts_root = root.join("Hosts");
    let plugins_root = root.join("Plugins");
    let source_repo = plugins_root.join("AesWorld");
    fs::create_dir_all(&config_dir).expect("config dir");

    setup_repo_with_base(&source_repo);
    fs::create_dir_all(source_repo.join("Source").join("AesWorld")).expect("source dir");
    fs::write(
        source_repo.join("AesWorld.uplugin"),
        "{ \"FriendlyName\": \"AesWorld\", \"Modules\": [{\"Name\": \"AesWorld\"}], \"Plugins\": [] }",
    )
    .expect("uplugin");
    git(&source_repo, &["add", "-A"]);
    git(&source_repo, &["commit", "-m", "add uplugin"]);

    let project = root.join("UGA").join("DEV");
    fs::create_dir_all(project.join("Plugins")).expect("project");
    fs::write(
        project.join("UGA.uproject"),
        "{ \"FileVersion\": 3, \"EngineAssociation\": \"5.5\", \"Plugins\": [] }",
    )
    .expect("uproject");
    let engine = root.join("Engine");
    fs::create_dir_all(engine.join("Engine").join("Build").join("BatchFiles")).expect("engine");
    fs::write(
        engine
            .join("Engine")
            .join("Build")
            .join("BatchFiles")
            .join("Build.bat"),
        "",
    )
    .expect("build.bat");

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "workspace",
            "add",
            "test",
            "--project",
            &project.to_string_lossy(),
            "--hosts-root",
            &hosts_root.to_string_lossy(),
            "--plugins-root",
            &plugins_root.to_string_lossy(),
            "--engine-path",
            &engine.to_string_lossy(),
            "-y",
        ])
        .assert()
        .success();

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "probe",
            "--workspace",
            "test",
            "--id",
            "create-probe",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success();
    assert_source_repo_undisturbed(&source_repo, "task create");

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "delete", "test/create-probe", "--yes", "--force"])
        .assert()
        .success();
    assert_source_repo_undisturbed(&source_repo, "task delete");
}
