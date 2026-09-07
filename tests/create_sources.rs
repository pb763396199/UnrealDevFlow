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

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
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

fn write_named_uplugin(plugin_dir: &Path, name: &str) {
    fs::write(
        plugin_dir.join(format!("{}.uplugin", name)),
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

fn write_uplugin_with_dependency(plugin_dir: &Path, dependency: &str) {
    fs::write(
        plugin_dir.join("AesWorld.uplugin"),
        format!(
            r#"{{
  "FileVersion": 3,
  "VersionName": "test",
  "Plugins": [
    {{ "Name": "{}", "Enabled": true }}
  ]
}}"#,
            dependency
        ),
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

fn write_workspace_config_with_plugin_path(
    config_dir: &Path,
    root: &Path,
    project: &Path,
    plugins_root: &Path,
    plugin_path: &Path,
) {
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
plugin_path = "{}"
default_project = "{}"
engine_path = "{}"
plugins_root = "{}"

[workspaces.bad]
hosts_root = "{}"
plugin_path = "{}"
default_project = "{}"
engine_path = "{}"
plugins_root = "{}"
"#,
        toml_path(&hosts_root),
        toml_path(plugin_path),
        toml_path(project),
        toml_path(&engine),
        toml_path(plugins_root),
        toml_path(&hosts_root),
        toml_path(plugin_path),
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
fn task_create_allows_untracked_main_checkout_changes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let temporary_file = main_repo.join("workflow").join("session.md");
    fs::create_dir_all(temporary_file.parent().expect("temporary parent"))
        .expect("temporary directory");
    fs::write(&temporary_file, "local-only\n").expect("temporary file");
    let source_status_before = git_stdout(&main_repo, &["status", "--porcelain"]);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "untracked source",
            "--workspace",
            "bad",
            "--id",
            "untracked-source",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("不会带入 Host"));

    let worktree = root
        .join("Hosts")
        .join("W-bad")
        .join("T-untracked-source_Host")
        .join("Plugins")
        .join("AesWorld");
    assert!(!worktree.join("workflow").exists());
    assert_eq!(
        git_stdout(&main_repo, &["status", "--porcelain"]),
        source_status_before
    );
    assert!(temporary_file.exists());

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["task", "delete", "bad/untracked-source", "--yes", "--force"])
        .assert()
        .success();
}

#[test]
fn task_create_allows_unstaged_tracked_main_checkout_changes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let descriptor = main_repo.join("AesWorld.uplugin");
    let committed_content = fs::read_to_string(&descriptor).expect("committed descriptor");
    let local_content = committed_content.replace(
        "\"Plugins\": []",
        "\"Plugins\": [{ \"Name\": \"DirtyOnlyMissing\", \"Enabled\": true }]",
    );
    fs::write(&descriptor, &local_content).expect("unstaged tracked change");
    let source_status_before = git_stdout(&main_repo, &["status", "--porcelain"]);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "unstaged tracked source",
            "--workspace",
            "bad",
            "--id",
            "unstaged-tracked-source",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("不会带入 Host"));

    assert_eq!(
        git_stdout(&main_repo, &["status", "--porcelain"]),
        source_status_before
    );
    let task_worktree = root
        .join("Hosts")
        .join("W-bad")
        .join("T-unstaged-tracked-source_Host")
        .join("Plugins")
        .join("AesWorld");
    assert_eq!(
        normalize_newlines(
            &fs::read_to_string(task_worktree.join("AesWorld.uplugin")).expect("task descriptor")
        ),
        normalize_newlines(&committed_content)
    );
    assert_eq!(
        fs::read_to_string(&descriptor).expect("main descriptor"),
        local_content
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "delete",
            "bad/unstaged-tracked-source",
            "--yes",
            "--force",
        ])
        .assert()
        .success();
}

#[test]
fn task_create_allows_staged_tracked_main_checkout_changes() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let descriptor = main_repo.join("AesWorld.uplugin");
    let committed_content = fs::read_to_string(&descriptor).expect("committed descriptor");
    let local_content = committed_content.replace(
        "\"Plugins\": []",
        "\"Plugins\": [{ \"Name\": \"DirtyOnlyMissing\", \"Enabled\": true }]",
    );
    fs::write(&descriptor, &local_content).expect("staged tracked change");
    git(&main_repo, &["add", "AesWorld.uplugin"]);
    let source_status_before = git_stdout(&main_repo, &["status", "--porcelain"]);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "staged tracked source",
            "--workspace",
            "bad",
            "--id",
            "staged-tracked-source",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("不会带入 Host"));

    assert_eq!(
        git_stdout(&main_repo, &["status", "--porcelain"]),
        source_status_before
    );
    let task_worktree = root
        .join("Hosts")
        .join("W-bad")
        .join("T-staged-tracked-source_Host")
        .join("Plugins")
        .join("AesWorld");
    assert_eq!(
        normalize_newlines(
            &fs::read_to_string(task_worktree.join("AesWorld.uplugin")).expect("task descriptor")
        ),
        normalize_newlines(&committed_content)
    );
    assert_eq!(
        fs::read_to_string(&descriptor).expect("main descriptor"),
        local_content
    );

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "delete",
            "bad/staged-tracked-source",
            "--yes",
            "--force",
        ])
        .assert()
        .success();
}

#[test]
fn task_create_rejects_unmerged_conflicts() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let descriptor = main_repo.join("AesWorld.uplugin");
    let committed_content = fs::read_to_string(&descriptor).expect("committed descriptor");

    git(&main_repo, &["checkout", "-b", "conflicting-change"]);
    fs::write(
        &descriptor,
        committed_content.replace("\"test\"", "\"feature\""),
    )
    .expect("feature descriptor");
    git(&main_repo, &["add", "AesWorld.uplugin"]);
    git(&main_repo, &["commit", "-m", "feature descriptor"]);
    git(&main_repo, &["checkout", "dev"]);
    fs::write(
        &descriptor,
        committed_content.replace("\"test\"", "\"dev\""),
    )
    .expect("dev descriptor");
    git(&main_repo, &["add", "AesWorld.uplugin"]);
    git(&main_repo, &["commit", "-m", "dev descriptor"]);

    let merge = std::process::Command::new("git")
        .args(["merge", "conflicting-change"])
        .current_dir(&main_repo)
        .output()
        .expect("run conflicting merge");
    assert!(
        !merge.status.success(),
        "merge should leave an unmerged conflict"
    );
    let source_status_before = git_stdout(&main_repo, &["status", "--porcelain"]);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "conflicted source",
            "--workspace",
            "bad",
            "--id",
            "conflicted-source",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("未合并冲突"))
        .stderr(predicate::str::contains("AesWorld.uplugin"));

    assert_eq!(
        git_stdout(&main_repo, &["status", "--porcelain"]),
        source_status_before
    );
    assert!(
        !root
            .join("Hosts")
            .join("W-bad")
            .join("T-conflicted-source_Host")
            .exists()
    );
    git(&main_repo, &["merge", "--abort"]);
}

#[test]
fn task_create_dirty_checkout_docs_are_consistent() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative_path in [
        "README.md",
        "docs/releases/v0.5.0.md",
        "skills/unrealdevflow/SKILL.md",
    ] {
        let content = fs::read_to_string(repo.join(relative_path)).expect("document");
        let compact = content.split_whitespace().collect::<String>();
        assert!(
            compact.contains("未跟踪、未暂存或已暂存"),
            "{relative_path} should list every allowed ordinary change state"
        );
        assert!(
            compact.contains("未合并冲突或未完成的Git操作"),
            "{relative_path} should name the unsafe states that still block creation"
        );
    }
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

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
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

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
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

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
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

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
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
fn create_uses_dev_commit_and_keeps_dirty_main_checkout_untouched() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let dev_head = git_stdout(&main_repo, &["rev-parse", "HEAD"]);
    fs::create_dir_all(main_repo.join("output")).expect("output");
    fs::write(main_repo.join("output/build.log"), "local output\n").expect("output file");
    fs::write(main_repo.join("next-rerun-only.yml"), "local workflow\n").expect("workflow file");
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "dirty main checkout",
            "--workspace",
            "bad",
            "--id",
            "dirty-main",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success();

    let host = root.join("Hosts").join("W-bad").join("T-dirty-main_Host");
    let task_worktree = host.join("Plugins").join("AesWorld");
    assert_eq!(git_stdout(&task_worktree, &["rev-parse", "HEAD"]), dev_head);
    assert!(main_repo.join("output/build.log").is_file());
    assert!(main_repo.join("next-rerun-only.yml").is_file());
    assert!(git_stdout(&main_repo, &["status", "--porcelain"]).contains("next-rerun-only.yml"));
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

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
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
        "feature/good-task"
    );
}

#[test]
fn workspace_add_does_not_overwrite_existing_legacy_defaults() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).expect("config dir");

    let old_project = root.join("OldProject");
    let old_hosts = root.join("OldHosts");
    let old_engine = root.join("OldEngine");
    let old_plugins = root.join("OldPlugins");
    fs::create_dir_all(&old_project).expect("old project");
    fs::create_dir_all(&old_hosts).expect("old hosts");
    fs::create_dir_all(&old_engine).expect("old engine");
    fs::create_dir_all(&old_plugins).expect("old plugins");
    fs::write(
        config_dir.join("config.toml"),
        format!(
            r#"hosts_root = "{}"
default_project = "{}"
engine_path = "{}"
plugins_root = "{}"
"#,
            toml_path(&old_hosts),
            toml_path(&old_project),
            toml_path(&old_engine),
            toml_path(&old_plugins),
        ),
    )
    .expect("legacy config");

    let new_project = root.join("UGA").join("DEV");
    fs::create_dir_all(&new_project).expect("new project");
    fs::write(
        new_project.join("DEV.uproject"),
        r#"{"EngineAssociation":"5.5"}"#,
    )
    .expect("uproject");
    let new_engine = root.join("UE_5.5");
    let build_bat = new_engine
        .join("Engine")
        .join("Build")
        .join("BatchFiles")
        .join("Build.bat");
    fs::create_dir_all(build_bat.parent().expect("Build.bat parent")).expect("engine dir");
    fs::write(&build_bat, "").expect("Build.bat");
    let new_plugins = root.join("Plugins");
    fs::create_dir_all(&new_plugins).expect("new plugins");

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "workspace",
            "add",
            "new",
            "--project",
            new_project.to_str().expect("new project path"),
            "--hosts-root",
            root.join("Hosts").to_str().expect("hosts path"),
            "--plugins-root",
            new_plugins.to_str().expect("plugins path"),
            "--engine-path",
            new_engine.to_str().expect("engine path"),
            "--yes",
        ])
        .assert()
        .success();

    let saved = fs::read_to_string(config_dir.join("config.toml")).expect("saved config");
    assert!(saved.contains(&format!("default_project = '{}'", old_project.display())));
    assert!(saved.contains("[workspaces.new]"));
    assert!(saved.contains(&format!("default_project = '{}'", new_project.display())));
}

#[test]
fn workspace_doctor_deep_scans_all_plugin_sources_but_default_is_shallow() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let plugins_root = main_repo.parent().expect("plugins root");

    let unrelated_a = plugins_root.join("LegacyPack").join("AesArtAsset");
    let unrelated_b = plugins_root.join("AesArtAsset");
    fs::create_dir_all(&unrelated_a).expect("unrelated a dir");
    fs::create_dir_all(&unrelated_b).expect("unrelated b dir");
    write_named_uplugin(&unrelated_a, "AesArtAsset");
    write_named_uplugin(&unrelated_b, "AesArtAsset");

    let project = root.join("UGA").join("DEV");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["workspace", "doctor", "bad"])
        .assert()
        .success();

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args(["workspace", "doctor", "bad", "--deep"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("重复插件 'AesArtAsset'"));
}

#[test]
fn start_ignores_duplicate_unrelated_plugins_in_plugins_root() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let plugins_root = main_repo.parent().expect("plugins root");

    let unrelated_a = plugins_root.join("LegacyPack").join("AesArtAsset");
    let unrelated_b = plugins_root.join("AesArtAsset");
    fs::create_dir_all(&unrelated_a).expect("unrelated a dir");
    fs::create_dir_all(&unrelated_b).expect("unrelated b dir");
    write_named_uplugin(&unrelated_a, "AesArtAsset");
    write_named_uplugin(&unrelated_b, "AesArtAsset");

    let project = root.join("UGA").join("DEV");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "good task",
            "--workspace",
            "bad",
            "--id",
            "ignore-unrelated",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success();

    assert!(
        root.join("Hosts")
            .join("W-bad")
            .join("T-ignore-unrelated_Host")
            .join(".udf-meta.json")
            .exists()
    );
}

#[test]
fn start_uses_exact_default_plugin_path_to_resolve_duplicate_primary_name() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let plugins_root = main_repo.parent().expect("plugins root");

    let duplicate_same_name = plugins_root.join("AesWorld_AI");
    fs::create_dir_all(&duplicate_same_name).expect("duplicate dir");
    write_named_uplugin(&duplicate_same_name, "AesWorld");

    let project = root.join("UGA").join("DEV");
    write_workspace_config_with_plugin_path(&config_dir, root, &project, plugins_root, &main_repo);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "good task",
            "--workspace",
            "bad",
            "--id",
            "exact-default",
            "--yes",
        ])
        .assert()
        .success();

    let host_dir = root
        .join("Hosts")
        .join("W-bad")
        .join("T-exact-default_Host");
    let meta_text = fs::read_to_string(host_dir.join(".udf-meta.json")).expect("meta");
    let meta: serde_json::Value = serde_json::from_str(&meta_text).expect("meta json");
    assert_eq!(
        meta["primary_plugins"][0]["source_repo"].as_str().unwrap(),
        dunce::canonicalize(&main_repo)
            .unwrap_or(main_repo)
            .to_string_lossy()
    );
}

#[test]
fn explicit_primary_uses_default_plugin_path_to_resolve_duplicate_primary_name() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let plugins_root = main_repo.parent().expect("plugins root");

    let duplicate_same_name = plugins_root.join("AesWorld_AI");
    fs::create_dir_all(&duplicate_same_name).expect("duplicate dir");
    write_named_uplugin(&duplicate_same_name, "AesWorld");

    let project = root.join("UGA").join("DEV");
    write_workspace_config_with_plugin_path(&config_dir, root, &project, plugins_root, &main_repo);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "bad explicit primary",
            "--workspace",
            "bad",
            "--id",
            "explicit-duplicate",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success();

    let host_dir = root
        .join("Hosts")
        .join("W-bad")
        .join("T-explicit-duplicate_Host");
    let meta_text = fs::read_to_string(host_dir.join(".udf-meta.json")).expect("meta");
    let meta: serde_json::Value = serde_json::from_str(&meta_text).expect("meta json");
    assert_eq!(
        meta["primary_plugins"][0]["source_repo"].as_str().unwrap(),
        dunce::canonicalize(&main_repo)
            .unwrap_or(main_repo)
            .to_string_lossy()
    );
}

#[test]
fn override_dep_engine_stays_engine_and_creates_no_junction() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    write_uplugin_with_dependency(&main_repo, "SharedDep");
    git(&main_repo, &["add", "AesWorld.uplugin"]);
    git(&main_repo, &["commit", "-m", "add shared dep"]);

    let plugins_root = main_repo.parent().expect("plugins root");
    let project_dep = plugins_root.join("SharedDep");
    fs::create_dir_all(&project_dep).expect("project dep dir");
    write_named_uplugin(&project_dep, "SharedDep");

    let engine_dep = root
        .join("UE_5.5")
        .join("Engine")
        .join("Plugins")
        .join("Runtime")
        .join("SharedDep");
    fs::create_dir_all(&engine_dep).expect("engine dep dir");
    write_named_uplugin(&engine_dep, "SharedDep");

    let project = root.join("UGA").join("DEV");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "engine dep",
            "--workspace",
            "bad",
            "--id",
            "engine-dep",
            "--primary",
            "AesWorld",
            "--override-dep",
            "SharedDep=engine",
            "--yes",
        ])
        .assert()
        .success();

    let host_dir = root.join("Hosts").join("W-bad").join("T-engine-dep_Host");
    let meta_text = fs::read_to_string(host_dir.join(".udf-meta.json")).expect("meta");
    let meta: serde_json::Value = serde_json::from_str(&meta_text).expect("meta json");
    let dep = &meta["dependency_plugins"][0];
    assert_eq!(dep["name"].as_str().unwrap(), "SharedDep");
    assert_eq!(dep["source"].as_str().unwrap(), "engine");
    assert!(dep.get("junction").is_none());
    assert!(!host_dir.join("Plugins").join("SharedDep").exists());
}

#[test]
fn create_rejects_duplicate_relevant_engine_dependency() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    write_uplugin_with_dependency(&main_repo, "DupEngine");
    git(&main_repo, &["add", "AesWorld.uplugin"]);
    git(&main_repo, &["commit", "-m", "add dup engine dep"]);

    let engine_plugins = root.join("UE_5.5").join("Engine").join("Plugins");
    let engine_a = engine_plugins.join("Runtime").join("DupEngine");
    let engine_b = engine_plugins.join("Experimental").join("DupEngine");
    fs::create_dir_all(&engine_a).expect("engine a dir");
    fs::create_dir_all(&engine_b).expect("engine b dir");
    write_named_uplugin(&engine_a, "DupEngine");
    write_named_uplugin(&engine_b, "DupEngine");

    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "dup engine",
            "--workspace",
            "bad",
            "--id",
            "dup-engine",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "引擎 Plugins 中发现重复插件 'DupEngine'",
        ));
}

#[test]
fn create_rejects_linked_worktree_project_dependency() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    write_uplugin_with_dependency(&main_repo, "LinkedDep");
    git(&main_repo, &["add", "AesWorld.uplugin"]);
    git(&main_repo, &["commit", "-m", "add linked dep"]);

    let dep_main = root.join("DepRepos").join("LinkedDep");
    fs::create_dir_all(&dep_main).expect("dep main dir");
    write_named_uplugin(&dep_main, "LinkedDep");
    git(&dep_main, &["init"]);
    git(&dep_main, &["config", "user.name", "UnrealDevFlow Test"]);
    git(
        &dep_main,
        &["config", "user.email", "unrealdevflow-test@example.com"],
    );
    git(&dep_main, &["add", "."]);
    git(&dep_main, &["commit", "-m", "base"]);
    git(&dep_main, &["checkout", "-B", "dev"]);

    let plugins_root = main_repo.parent().expect("plugins root");
    let linked_dep = plugins_root.join("LinkedDep");
    git(
        &dep_main,
        &[
            "worktree",
            "add",
            "-b",
            "task-active-dep",
            linked_dep.to_str().expect("linked dep path"),
            "dev",
        ],
    );

    let project = root.join("UGA").join("DEV");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "bad dep",
            "--workspace",
            "bad",
            "--id",
            "bad-linked-dep",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("项目依赖插件 'LinkedDep'"))
        .stderr(predicate::str::contains("linked worktree"));
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

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
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

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
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

#[test]
fn default_branch_name_carries_the_change_type_and_not_the_workspace() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "some work",
            "--workspace",
            "bad",
            "--id",
            "save-bug",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success();

    let worktree = root
        .join("Hosts")
        .join("W-bad")
        .join("T-save-bug_Host")
        .join("Plugins")
        .join("AesWorld");
    let branch = git_stdout(&worktree, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(branch, "feature/save-bug");
    assert!(!branch.contains("bad"), "分支名不该带工程名: {}", branch);
}

#[test]
fn change_type_picks_the_branch_prefix() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "fix it",
            "--workspace",
            "bad",
            "--id",
            "save-bug",
            "--type",
            "fix",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success();

    let worktree = root
        .join("Hosts")
        .join("W-bad")
        .join("T-save-bug_Host")
        .join("Plugins")
        .join("AesWorld");
    assert_eq!(
        git_stdout(&worktree, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "fix/save-bug"
    );
}

#[test]
fn an_unknown_change_type_is_refused_before_anything_is_created() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "nope",
            "--workspace",
            "bad",
            "--id",
            "save-bug",
            "--type",
            "nonsense",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("feature").and(predicate::str::contains("hotfix")));

    assert!(
        !root
            .join("Hosts")
            .join("W-bad")
            .join("T-save-bug_Host")
            .exists(),
        "取值非法时不该建出 Host"
    );
}

#[test]
fn the_branch_ledger_records_which_task_owns_the_branch() {
    let temp = TempDir::new().expect("temp dir");
    let root = temp.path();
    let config_dir = root.join("config");
    let main_repo = setup_main_plugin_repo(root);
    let project = root.join("UGA").join("DEV");
    let plugins_root = main_repo.parent().expect("plugins root");
    write_workspace_config(&config_dir, root, &project, plugins_root);

    Command::cargo_bin("udf")
        .expect("binary")
        .env("UNREALDEVFLOW_CONFIG_DIR", &config_dir)
        .args([
            "task",
            "create",
            "ledger",
            "--workspace",
            "bad",
            "--id",
            "save-bug",
            "--branch",
            "release/totally-unrelated",
            "--primary",
            "AesWorld",
            "--yes",
        ])
        .assert()
        .success();

    // 分支名完全不含 task-id，能不能找回来只取决于台账
    let recorded = git_stdout(
        &main_repo,
        &[
            "config",
            "--get",
            "branch.release/totally-unrelated.udftask",
        ],
    );
    assert_eq!(recorded, "bad/save-bug");
}
