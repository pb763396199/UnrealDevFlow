use assert_cmd::cargo::CommandCargoExt;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    config_dir: PathBuf,
    project: PathBuf,
    task_host: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config_dir = temp.path().join("config");
        let project = temp.path().join("Project");
        let hosts = temp.path().join("Hosts");
        let task_host = hosts.join("W-test").join("T-shipping_Host");
        let engine = temp.path().join("Engine");
        let plugins = temp.path().join("Plugins");
        for path in [&config_dir, &project, &task_host, &hosts, &engine, &plugins] {
            fs::create_dir_all(path).unwrap();
        }
        fs::write(project.join("Game.uproject"), "{\"FileVersion\":3}").unwrap();
        fs::write(task_host.join("Task.uproject"), "{\"FileVersion\":3}").unwrap();
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
        fs::write(config_dir.join("config.toml"), config).unwrap();
        Self {
            _temp: temp,
            config_dir,
            project,
            task_host,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::cargo_bin("udf").unwrap();
        command.env("UNREALDEVFLOW_CONFIG_DIR", &self.config_dir);
        command
    }
}

fn toml_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[test]
fn configure_creates_a_task_profile_and_preserves_shipping() {
    let fixture = Fixture::new();
    let task_meta_dir = fixture.task_host.join("Plugins");
    fs::create_dir_all(&task_meta_dir).unwrap();
    fs::write(
        fixture.task_host.join(".udf-meta.json"),
        format!(
            r#"{{"schema_version":3,"id":"shipping","name":"shipping","branch":"feature/shipping","created":"2026-08-31T00:00:00Z","based_on":"0123456789abcdef","status":"active","primary_plugins":[],"dependency_plugins":[],"workspace":"test","task_uid":"test/shipping","context":{{"workspace":"test","hosts_root":"{}","default_project":"{}","engine_path":"{}","plugins_root":"{}","plugin_overrides":{{}}}}}}"#,
            fixture.task_host.parent().unwrap().to_string_lossy().replace('\\', "/"),
            fixture.project.to_string_lossy().replace('\\', "/"),
            fixture.config_dir.to_string_lossy().replace('\\', "/"),
            fixture.config_dir.to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();

    let output = fixture
        .command()
        .args([
            "--format",
            "json",
            "package",
            "configure",
            "--task",
            "shipping",
            "--configuration",
            "Shipping",
            "--container",
            "loose",
            "--output",
            "C:/Packages/Shipping",
            "--disable-plugin",
            "ModelContextProtocol",
            "--reason",
            "验证 Shipping 配置",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["data"]["configuration"], "Shipping");
    assert_eq!(response["data"]["container"], "loose");
    assert!(response["data"]["profilePath"].as_str().is_some());

    let plan = fixture
        .command()
        .args([
            "--format", "json", "package", "plan", "project", "--task", "shipping",
        ])
        .output()
        .unwrap();
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stdout)
    );
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    let command = plan["data"]["steps"][0]["argv"].as_array().unwrap();
    assert!(
        command
            .iter()
            .any(|value| value == "-clientconfig=Shipping")
    );
    assert!(!command.iter().any(|value| value == "-pak"));
}

#[test]
fn configure_rejects_unknown_profile_fields() {
    let fixture = Fixture::new();
    let candidate = fixture.config_dir.join("candidate.toml");
    fs::write(
        &candidate,
        "[package]\nconfiguration=\"Shipping\"\nunknown=true\n",
    )
    .unwrap();
    let output = fixture
        .command()
        .args([
            "--format",
            "json",
            "package",
            "configure",
            "--workspace",
            "test",
            "--file",
            candidate.to_str().unwrap(),
            "--reason",
            "测试未知字段",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(response["error"].as_str().unwrap().contains("unknown"));
}

#[test]
fn package_plan_exposes_the_saved_profile_without_a_second_show_command() {
    let fixture = Fixture::new();
    let output = fixture
        .command()
        .args([
            "--format",
            "json",
            "package",
            "plan",
            "project",
            "--workspace",
            "test",
        ])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout={text} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains("Development"), "{text}");
}

#[test]
fn configure_writes_nested_profile_and_rejects_direct_edits() {
    let fixture = Fixture::new();
    let configured = fixture
        .command()
        .args([
            "package",
            "configure",
            "--workspace",
            "test",
            "--configuration",
            "Shipping",
            "--reason",
            "固定 Shipping 配方",
        ])
        .output()
        .unwrap();
    assert!(configured.status.success());
    let profile = fixture
        .config_dir
        .join("package/profiles/workspace_test.toml");
    let text = fs::read_to_string(&profile).unwrap();
    assert!(text.contains("[build]"), "{text}");
    assert!(text.contains("configuration = \"Shipping\""), "{text}");

    fs::write(
        &profile,
        text.replace(
            "configuration = \"Shipping\"",
            "configuration = \"Development\"",
        ),
    )
    .unwrap();
    let plan = fixture
        .command()
        .args(["package", "plan", "project", "--workspace", "test"])
        .output()
        .unwrap();
    assert!(!plan.status.success());
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&plan.stdout),
        String::from_utf8_lossy(&plan.stderr)
    );
    assert!(diagnostics.contains("直接修改"), "{diagnostics}");
}

#[test]
fn configure_inherits_project_packaging_baseline_and_detects_changes() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.project.join("Config")).unwrap();
    fs::write(
        fixture.project.join("Config/DefaultGame.ini"),
        r#"[/Script/UnrealEd.ProjectPackagingSettings]
BuildConfiguration=PPBC_Development
PerPlatformBuildConfig=(("Windows", PPBC_Shipping))
UsePakFile=True
bCompressed=True
IncludePrerequisites=True
"#,
    )
    .unwrap();
    let configured = fixture
        .command()
        .args([
            "--format",
            "json",
            "package",
            "configure",
            "--workspace",
            "test",
            "--reason",
            "读取项目原生配置",
        ])
        .output()
        .unwrap();
    assert!(configured.status.success());
    let response: Value = serde_json::from_slice(&configured.stdout).unwrap();
    assert_eq!(response["data"]["configuration"], "Shipping");
    assert_eq!(response["data"]["cookMode"], "iterate");

    let profile = fixture
        .config_dir
        .join("package/profiles/workspace_test.toml");
    let text = fs::read_to_string(&profile).unwrap();
    assert!(text.contains("project_settings"), "{text}");
    assert!(text.contains("mode = \"iterate\""), "{text}");

    fs::write(
        fixture.project.join("Config/DefaultGame.ini"),
        "[/Script/UnrealEd.ProjectPackagingSettings]\nUsePakFile=False\n",
    )
    .unwrap();
    let plan = fixture
        .command()
        .args([
            "--format",
            "json",
            "package",
            "plan",
            "project",
            "--workspace",
            "test",
        ])
        .output()
        .unwrap();
    assert!(!plan.status.success());
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&plan.stdout),
        String::from_utf8_lossy(&plan.stderr)
    );
    assert!(diagnostics.contains("原生打包设置已变化"), "{diagnostics}");
}

#[test]
fn configure_can_fix_full_mode_and_requires_reason_for_existing_profile() {
    let fixture = Fixture::new();
    let first = fixture
        .command()
        .args([
            "package",
            "configure",
            "--workspace",
            "test",
            "--cook-mode",
            "iterate",
            "--reason",
            "日常开发",
        ])
        .output()
        .unwrap();
    assert!(first.status.success());

    let missing_reason = fixture
        .command()
        .args([
            "package",
            "configure",
            "--workspace",
            "test",
            "--cook-mode",
            "full",
        ])
        .output()
        .unwrap();
    assert!(!missing_reason.status.success());

    let release = fixture
        .command()
        .args([
            "package",
            "configure",
            "--workspace",
            "test",
            "--cook-mode",
            "full",
            "--reason",
            "正式发布",
        ])
        .output()
        .unwrap();
    assert!(release.status.success());
    let profile = fixture
        .config_dir
        .join("package/profiles/workspace_test.toml");
    assert!(
        fs::read_to_string(profile)
            .unwrap()
            .contains("mode = \"full\"")
    );
}
