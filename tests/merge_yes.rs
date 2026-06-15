use assert_cmd::Command;
use std::fs;
use std::path::Path;
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

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
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

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["cleanup", "test/cleanup-yes", "-y"])
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

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["cleanup", "test/cleanup-force", "--force"])
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

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["cleanup", "test/cleanup-worktree", "--force"])
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

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["cleanup", "test/orphan-cleanup", "--force"])
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

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["delete", "test/delete-force", "--force"])
        .assert()
        .success();

    assert!(!host_dir.exists());
    assert_eq!(
        git_stdout(&source_repo, &["branch", "--list", "task-delete-force"]),
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

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["delete", "test/delete-worktree", "--force"])
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

    Command::cargo_bin("unrealdevflow")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
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
