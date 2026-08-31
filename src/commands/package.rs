use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::output;
use crate::package_profile;
use crate::ue_commands::{
    Configuration, EngineSourceBuildOptions, InstalledBuildOptions, PackageContainer,
    ProjectPackageOptions, UbtMutexMode, UeCommand, UePlatform, engine_source_build_commands,
    installed_build_commands, project_package_commands,
};
use chrono::Utc;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn plugin_stage_root(execution_id: &str) -> PathBuf {
    let mut hasher = Md5::new();
    hasher.update(execution_id.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    std::env::temp_dir().join("UDF").join(&digest[..12])
}

fn is_managed_cleanup_target(path: &Path) -> bool {
    if path
        .components()
        .any(|part| part.as_os_str() == "UnrealDevFlow")
    {
        return true;
    }

    let temp_stage_root = std::env::temp_dir().join("UDF");
    path.starts_with(temp_stage_root) && path != std::env::temp_dir().join("UDF")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageMode {
    Run,
    Plan,
    Check,
}

impl PackageMode {
    fn executes(self) -> bool {
        self == Self::Run
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageCheckReport {
    domain: &'static str,
    action: String,
    source: String,
    readiness: String,
    checks: Vec<String>,
    diagnostics: Vec<String>,
    next_command: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackagePlanStep {
    name: String,
    executable: String,
    argv: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackagePlanReport {
    domain: &'static str,
    action: String,
    source: String,
    plan_digest: String,
    steps: Vec<PackagePlanStep>,
    outputs: Vec<PathBuf>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageResult {
    execution_id: String,
    action: String,
    workspace: String,
    source: String,
    state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    commands: Vec<Vec<String>>,
    outputs: Vec<PathBuf>,
    #[serde(default)]
    artifacts: Vec<PathBuf>,
    logs: Vec<PathBuf>,
    #[serde(default)]
    manifests: Vec<PathBuf>,
    #[serde(default)]
    cleanup_targets: Vec<PathBuf>,
    #[serde(default)]
    diagnostics: Vec<String>,
}

fn check_package_commands(
    commands: &[UeCommand],
    mutex_project: Option<(&Path, &Path)>,
) -> (String, Vec<String>) {
    let mut missing = commands
        .iter()
        .map(|command| PathBuf::from(&command.executable))
        .filter(|path| !path.is_file())
        .collect::<Vec<_>>();
    missing.extend(
        commands
            .iter()
            .flat_map(|command| &command.arguments)
            .filter(|argument| {
                argument
                    .to_ascii_lowercase()
                    .ends_with("unrealbuildtool.dll")
            })
            .map(PathBuf::from)
            .filter(|path| !path.is_file()),
    );
    missing.sort();
    missing.dedup();
    if !missing.is_empty() {
        return (
            "blocked".to_string(),
            missing
                .iter()
                .map(|path| format!("缺少可执行文件：{}", path.display()))
                .collect(),
        );
    }
    if let Some((project, engine_root)) = mutex_project {
        let report =
            crate::build_policy::resolve_build_policy(&crate::build_policy::BuildPolicyRequest {
                main_project: Some(project.to_string_lossy().to_string()),
                engine_root: Some(engine_root.to_string_lossy().to_string()),
                ..crate::build_policy::BuildPolicyRequest::default()
            });
        return (
            report.status.as_str().to_string(),
            std::iter::once(report.reason)
                .chain(report.diagnostics)
                .collect(),
        );
    }
    ("ready".to_string(), Vec::new())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    path: PathBuf,
    bytes: u64,
}

fn collect_manifest_entries(
    root: &Path,
    current: &Path,
    entries: &mut Vec<ManifestEntry>,
) -> Result<()> {
    if !current.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_manifest_entries(root, &path, entries)?;
        } else if entry.file_type()?.is_file() {
            entries.push(ManifestEntry {
                path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                bytes: entry.metadata()?.len(),
            });
        }
    }
    Ok(())
}

fn write_manifest(root: &Path) -> Result<PathBuf> {
    let mut files = Vec::new();
    collect_manifest_entries(root, root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let path = root.join(".udf-manifest.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": 1,
            "generatedAt": Utc::now().to_rfc3339(),
            "root": root,
            "files": files,
        }))?,
    )?;
    Ok(path)
}

fn execution_id(action: &str) -> String {
    format!("package-{action}-{}", Utc::now().format("%Y%m%dT%H%M%SZ"))
}

fn project_file(project_dir: &Path) -> Result<PathBuf> {
    let mut projects = fs::read_dir(project_dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "uproject"))
        .collect::<Vec<_>>();
    projects.sort();
    match projects.as_slice() {
        [project] => Ok(project.clone()),
        [] => Err(UdfError::Other(format!(
            "项目目录中找不到 .uproject：{}",
            project_dir.display()
        ))),
        _ => Err(UdfError::Other(format!(
            "项目目录中有多个 .uproject，请先修正 workspace：{}",
            project_dir.display()
        ))),
    }
}

fn commands_as_argv(commands: &[UeCommand]) -> Vec<Vec<String>> {
    commands.iter().map(UeCommand::argv).collect()
}

fn emit_check(
    action: &str,
    source: &str,
    commands: &[UeCommand],
    mutex_project: Option<(&Path, &Path)>,
    next_command: String,
) -> Result<()> {
    let (readiness, diagnostics) = check_package_commands(commands, mutex_project);
    let checks = if diagnostics.is_empty() {
        vec!["toolchainAvailable".to_string()]
    } else {
        diagnostics
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .split('：')
                    .next()
                    .unwrap_or(diagnostic)
                    .to_string()
            })
            .collect()
    };
    let report = PackageCheckReport {
        domain: "package",
        action: action.to_string(),
        source: source.to_string(),
        readiness,
        checks,
        diagnostics,
        next_command,
    };
    output::emit("package check", report, |report| {
        format!("package {}: {}", report.action, report.readiness)
    });
    Ok(())
}

fn emit_plan(
    action: &str,
    source: &str,
    commands: &[UeCommand],
    outputs: Vec<PathBuf>,
    diagnostics: Vec<String>,
) -> Result<()> {
    let steps = commands
        .iter()
        .enumerate()
        .map(|(index, command)| PackagePlanStep {
            name: format!("step-{:02}", index + 1),
            executable: command.executable.clone(),
            argv: command.argv(),
        })
        .collect::<Vec<_>>();
    let normalized = serde_json::to_vec(&(&steps, &outputs, &diagnostics))?;
    let plan_digest = format!("md5:{:x}", Md5::digest(normalized));
    let report = PackagePlanReport {
        domain: "package",
        action: action.to_string(),
        source: source.to_string(),
        plan_digest,
        steps,
        outputs,
        diagnostics,
    };
    output::emit("package plan", report, |report| {
        format!("package {} plan: {}", report.action, report.plan_digest)
    });
    Ok(())
}

fn render(result: &PackageResult) -> String {
    let mut lines = vec![format!(
        "package {}: {} ({})",
        result.action, result.state, result.execution_id
    )];
    for command in &result.commands {
        lines.push(format!("  {}", command.join(" ")));
    }
    for output in &result.outputs {
        lines.push(format!("  输出：{}", output.display()));
    }
    lines.join("\n")
}

fn execution_root() -> Result<PathBuf> {
    Ok(Config::config_dir()?.join("executions").join("package"))
}

fn save_result(result: &PackageResult) -> Result<()> {
    let root = execution_root()?;
    fs::create_dir_all(&root)?;
    fs::write(
        root.join(format!("{}.json", result.execution_id)),
        serde_json::to_vec_pretty(result)?,
    )?;
    fs::write(root.join("latest"), result.execution_id.as_bytes())?;
    Ok(())
}

fn finish_execution(command: &str, result: PackageResult) -> Result<()> {
    save_result(&result)?;
    output::emit(command, result, render);
    Ok(())
}

fn run_commands(commands: &[UeCommand], log_dir: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(log_dir)?;
    let mut logs = Vec::new();
    for (index, command) in commands.iter().enumerate() {
        let log = log_dir.join(format!("step-{:02}.log", index + 1));
        let stdout = fs::File::create(&log)?;
        let stderr = stdout.try_clone()?;
        let status = Command::new(&command.executable)
            .args(&command.arguments)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .status()
            .map_err(|error| {
                UdfError::Other(format!("无法启动 {}：{}", command.executable, error))
            })?;
        logs.push(log.clone());
        if !status.success() {
            return Err(UdfError::Other(format!(
                "package 第 {} 步失败，退出码 {:?}。日志：{}",
                index + 1,
                status.code(),
                log.display()
            )));
        }
    }
    Ok(logs)
}

#[allow(clippy::too_many_arguments)]
fn run_package_commands(
    commands: &[UeCommand],
    log_dir: &Path,
    execution_id: &str,
    action: &str,
    workspace: &str,
    source: &str,
    outputs: Vec<PathBuf>,
    cleanup_targets: Vec<PathBuf>,
) -> Result<Vec<PathBuf>> {
    match run_commands(commands, log_dir) {
        Ok(logs) => Ok(logs),
        Err(error) => {
            let mut logs = fs::read_dir(log_dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "log"))
                .collect::<Vec<_>>();
            logs.sort();
            save_result(&PackageResult {
                execution_id: execution_id.to_string(),
                action: action.to_string(),
                workspace: workspace.to_string(),
                source: source.to_string(),
                state: "failed".to_string(),
                exit_code: None,
                commands: commands_as_argv(commands),
                artifacts: outputs.clone(),
                outputs,
                logs,
                manifests: Vec::new(),
                cleanup_targets,
                diagnostics: vec![error.to_string()],
            })?;
            Err(UdfError::Other(format!(
                "{}（execution ID: {}）",
                error, execution_id
            )))
        }
    }
}

fn collect_plugin_descriptors(
    root: &Path,
    relative: &Path,
    descriptors: &mut BTreeMap<String, Vec<PathBuf>>,
) -> Result<()> {
    let directory = root.join(relative);
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_plugin_descriptors(root, &relative.join(entry.file_name()), descriptors)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "uplugin")
        {
            let name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| UdfError::Other(format!("无效插件描述文件：{}", path.display())))?
                .to_string();
            let plugin_dir = path
                .parent()
                .and_then(|parent| parent.strip_prefix(root).ok())
                .ok_or_else(|| {
                    UdfError::Other(format!("插件不在插件根目录内：{}", path.display()))
                })?
                .to_path_buf();
            descriptors.entry(name).or_default().push(plugin_dir);
        }
    }
    Ok(())
}

fn plugin_index(plugins_root: &Path) -> Result<BTreeMap<String, Vec<PathBuf>>> {
    let mut descriptors = BTreeMap::new();
    collect_plugin_descriptors(plugins_root, Path::new(""), &mut descriptors)?;
    Ok(descriptors)
}

fn resolve_plugin_seeds(
    plugins_root: &Path,
    requested: &[String],
    index: &BTreeMap<String, Vec<PathBuf>>,
) -> Result<(Vec<String>, BTreeMap<String, PathBuf>)> {
    let mut resolved = BTreeSet::new();
    let mut selected = BTreeMap::new();
    for request in requested {
        let direct = plugins_root
            .join(request)
            .join(format!("{request}.uplugin"));
        if direct.is_file() {
            resolved.insert(request.clone());
            selected.insert(request.clone(), PathBuf::from(request));
            continue;
        }
        let collection = plugins_root.join(request);
        let prefix = Path::new(request);
        let mut members = Vec::new();
        for (name, candidates) in index {
            let matches = candidates
                .iter()
                .filter(|relative| relative.starts_with(prefix))
                .collect::<Vec<_>>();
            if matches.len() > 1 {
                return Err(ambiguous_plugin_error(name, &matches));
            }
            if let Some(relative) = matches.first() {
                members.push(name.clone());
                selected.insert(name.clone(), (*relative).clone());
            }
        }
        if !collection.is_dir() || members.is_empty() {
            return Err(UdfError::Other(format!(
                "找不到插件或插件集合：{}",
                collection.display()
            )));
        }
        resolved.extend(members);
    }
    Ok((resolved.into_iter().collect(), selected))
}

fn ambiguous_plugin_error(name: &str, candidates: &[&PathBuf]) -> UdfError {
    let locations = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" 与 ");
    UdfError::Other(format!("插件集合中存在重名插件 '{}'：{}", name, locations))
}

fn plugin_closure(
    plugins_root: &Path,
    seeds: &[String],
    index: &BTreeMap<String, Vec<PathBuf>>,
    selected: &mut BTreeMap<String, PathBuf>,
) -> Result<Vec<String>> {
    let mut closure = BTreeSet::new();
    let mut pending = seeds.to_vec();
    while let Some(name) = pending.pop() {
        if !closure.insert(name.clone()) {
            continue;
        }
        let Some(relative_dir) = selected.get(&name) else {
            // Dependencies supplied by the engine are not staged.
            continue;
        };
        let plugin_dir = plugins_root.join(relative_dir);
        for dependency in crate::plugin::uplugin::read_dependencies(&plugin_dir)? {
            if let Some(candidates) = index.get(&dependency) {
                if candidates.len() > 1 {
                    return Err(ambiguous_plugin_error(
                        &dependency,
                        &candidates.iter().collect::<Vec<_>>(),
                    ));
                }
                if let Some(relative) = candidates.first() {
                    selected.insert(dependency.clone(), relative.clone());
                    pending.push(dependency);
                }
            }
        }
    }
    Ok(closure.into_iter().collect())
}

fn excluded_entry(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git" | "workflow" | "intermediate" | "saved" | "deriveddatacache" | "nul"
    )
}

fn copy_tree(source: &Path, destination: &Path, include_source: bool) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if excluded_entry(&name_text)
            || (!include_source && name_text.eq_ignore_ascii_case("Source"))
        {
            continue;
        }
        let source_path = entry.path();
        let destination_path = destination.join(&name);
        if crate::junction::exists(&source_path).unwrap_or(false) {
            let target = crate::junction::get_target(&source_path)?;
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            crate::junction::create(&target, &destination_path)?;
            continue;
        }
        if entry.file_type()?.is_dir() {
            copy_tree(&source_path, &destination_path, include_source)?;
        } else if entry.file_type()?.is_file() {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn prepare_project_stage(
    project: &Path,
    stage_root: &Path,
    disabled_plugins: &[String],
    plugin_overlays: &[(String, PathBuf)],
) -> Result<PathBuf> {
    if stage_root.exists() {
        fs::remove_dir_all(stage_root)?;
    }
    let source_root = project
        .parent()
        .ok_or_else(|| UdfError::Other(format!("项目路径没有父目录：{}", project.display())))?;
    copy_tree(source_root, stage_root, true)?;
    let staged_project = stage_root.join(
        project
            .file_name()
            .ok_or_else(|| UdfError::Other("项目文件名为空".into()))?,
    );
    let mut descriptor: serde_json::Value = serde_json::from_slice(&fs::read(&staged_project)?)?;
    if let Some(plugins) = descriptor
        .get_mut("Plugins")
        .and_then(serde_json::Value::as_array_mut)
    {
        for plugin in plugins {
            let Some(name) = plugin.get("Name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if disabled_plugins
                .iter()
                .any(|disabled| disabled.eq_ignore_ascii_case(name))
            {
                plugin["Enabled"] = serde_json::Value::Bool(false);
            }
        }
    }
    fs::write(&staged_project, serde_json::to_vec_pretty(&descriptor)?)?;
    for (name, source) in plugin_overlays {
        let destination = stage_root.join("Plugins").join(name);
        if crate::junction::exists(&destination).unwrap_or(false) {
            crate::junction::delete(&destination)?;
        } else if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        copy_tree(source, &destination, true)?;
    }
    Ok(staged_project)
}

fn prepare_plugin_stage(
    plugins_root: &Path,
    stage_root: &Path,
    closure: &[String],
    index: &BTreeMap<String, PathBuf>,
) -> Result<()> {
    if stage_root.exists() {
        fs::remove_dir_all(stage_root)?;
    }
    fs::create_dir_all(stage_root.join("Plugins"))?;
    for plugin in closure {
        if let Some(relative_dir) = index.get(plugin) {
            let source = plugins_root.join(relative_dir);
            copy_tree(
                &source,
                &stage_root.join("Plugins").join(relative_dir),
                true,
            )?;
        }
    }
    let enabled = closure
        .iter()
        .map(|name| serde_json::json!({"Name": name, "Enabled": true}))
        .collect::<Vec<_>>();
    fs::write(
        stage_root.join("HostProject.uproject"),
        serde_json::to_vec_pretty(&serde_json::json!({"FileVersion": 3, "Plugins": enabled}))?,
    )?;
    Ok(())
}

fn direct_plugin_commands(
    engine_root: &Path,
    stage_root: &Path,
    seeds: &[String],
    index: &BTreeMap<String, PathBuf>,
) -> Vec<UeCommand> {
    let dotnet = engine_root.join("Engine/Binaries/ThirdParty/DotNet/8.0.300/win-x64/dotnet.exe");
    let ubt = engine_root.join("Engine/Binaries/DotNET/UnrealBuildTool/UnrealBuildTool.dll");
    let project = stage_root.join("HostProject.uproject");
    let mut commands = Vec::new();
    for seed in seeds {
        let relative_dir = index.get(seed).expect("resolved plugin seed");
        let plugin = stage_root
            .join("Plugins")
            .join(relative_dir)
            .join(format!("{seed}.uplugin"));
        for (target, configuration) in [
            ("UnrealEditor", "Development"),
            ("UnrealGame", "Development"),
            ("UnrealGame", "Shipping"),
        ] {
            commands.push(UeCommand::new(
                dotnet.to_string_lossy(),
                vec![
                    ubt.to_string_lossy().to_string(),
                    target.to_string(),
                    "Win64".to_string(),
                    configuration.to_string(),
                    format!("-Project={}", project.display()),
                    format!("-plugin={}", plugin.display()),
                    "-nohotreload".to_string(),
                    // Plugin packaging compiles from a private staging project whose
                    // project/plugin intermediates are isolated per execution. UEB's
                    // dependency-aware path therefore does not take the engine-global
                    // UBT mutex and must not block normal Host builds.
                    "-NoMutex".to_string(),
                    format!(
                        "-log={}",
                        stage_root
                            .join("Saved/Logs")
                            .join(format!("UBT-{seed}-{target}-{configuration}.log"))
                            .display()
                    ),
                ],
            ));
        }
    }
    commands
}

pub fn project(workspace: Option<String>, task: Option<String>, mode: PackageMode) -> Result<()> {
    let config = Config::load()?;
    let requested_task = task.clone();
    let (name, project_root, engine_root, source) = if let Some(task_ref) = task {
        let (host_dir, _, context) = crate::host::resolve_task(&config, &task_ref)?;
        (context.workspace, host_dir, context.engine_path, "task")
    } else {
        let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
        (
            name,
            workspace_config.default_project,
            workspace_config.engine_path,
            "workspace",
        )
    };
    let saved_profile = package_profile::load_for_project(
        &config,
        workspace.as_deref(),
        requested_task.as_deref(),
    )?;
    let project = project_file(
        saved_profile
            .as_ref()
            .map(|p| &p.project)
            .unwrap_or(&project_root),
    )?;
    let archive_dir = saved_profile
        .as_ref()
        .map(|p| p.output.clone())
        .unwrap_or_else(|| project_root.join("Saved/UnrealDevFlow/Packages/Win64"));
    let engine_root = saved_profile
        .as_ref()
        .map(|p| p.engine.clone())
        .unwrap_or(engine_root);
    let configuration = saved_profile
        .as_ref()
        .map(|p| Configuration::parse(&p.configuration))
        .unwrap_or(Configuration::Development);
    let container = saved_profile
        .as_ref()
        .map(|p| match p.container {
            package_profile::Container::Loose => PackageContainer::Loose,
            package_profile::Container::Pak => PackageContainer::Pak,
            package_profile::Container::Iostore => PackageContainer::Iostore,
        })
        .unwrap_or(PackageContainer::Pak);
    let execution_id = execution_id("project");
    let staged_project = if mode.executes() {
        if let (Some(profile), Some(task_ref)) = (saved_profile.as_ref(), requested_task.as_deref())
        {
            let (host_dir, meta, _) = crate::host::resolve_task(&config, task_ref)?;
            let overlays = meta
                .primary_plugins
                .iter()
                .map(|plugin| (plugin.name.clone(), host_dir.join(&plugin.worktree)))
                .collect::<Vec<_>>();
            Some(prepare_project_stage(
                &project,
                &std::env::temp_dir().join("UDF").join(&execution_id),
                &profile.disabled_plugins,
                &overlays,
            )?)
        } else {
            None
        }
    } else {
        None
    };
    let project = staged_project.unwrap_or(project);
    let commands = project_package_commands(&ProjectPackageOptions {
        engine_root: engine_root.clone(),
        project: project.clone(),
        archive_dir: archive_dir.clone(),
        platform: UePlatform::Windows,
        configuration,
        mutex: UbtMutexMode::Wait,
        package_args: None,
        container,
        clean: false,
    });
    if mode == PackageMode::Check {
        let next = requested_task
            .map(|task| format!("udf package project --task {task}"))
            .unwrap_or_else(|| format!("udf package project --workspace {name}"));
        return emit_check(
            "project",
            source,
            &commands,
            Some((&project, &engine_root)),
            next,
        );
    }
    if mode == PackageMode::Plan {
        let (_, diagnostics) = check_package_commands(&commands, None);
        return emit_plan("project", source, &commands, vec![archive_dir], diagnostics);
    }
    let id = execution_id;
    let log_dir = project_root.join("Saved").join("UnrealDevFlow").join(&id);
    let logs = if mode.executes() {
        run_package_commands(
            &commands,
            &log_dir,
            &id,
            "project",
            &name,
            source,
            vec![archive_dir.clone()],
            vec![log_dir.clone()],
        )?
    } else {
        Vec::new()
    };
    let manifests = if mode.executes() {
        vec![write_manifest(&archive_dir)?]
    } else {
        Vec::new()
    };
    finish_execution(
        "package project",
        PackageResult {
            execution_id: id,
            action: "project".to_string(),
            workspace: name,
            source: source.to_string(),
            state: "succeeded".to_string(),
            exit_code: Some(0),
            commands: commands_as_argv(&commands),
            artifacts: vec![archive_dir.clone()],
            outputs: vec![archive_dir.clone()],
            logs,
            manifests,
            cleanup_targets: vec![archive_dir, log_dir],
            diagnostics: Vec::new(),
        },
    )
}

pub fn plugin(
    plugins: Vec<String>,
    task: Option<String>,
    workspace: Option<String>,
    mode: PackageMode,
) -> Result<()> {
    if plugins.is_empty() {
        return Err(UdfError::Other("至少指定一个插件名".to_string()));
    }
    let config = Config::load()?;
    let requested_task = task.clone();
    let (name, plugins_root, engine_root, output_dir, source) = if let Some(task_ref) = task {
        let (host_dir, _, context) = crate::host::resolve_task(&config, &task_ref)?;
        (
            context.workspace,
            host_dir.join("Plugins"),
            context.engine_path,
            host_dir.join("Artifacts/UnrealDevFlow/Plugins"),
            "task",
        )
    } else {
        let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
        let plugins_root = workspace_config
            .effective_plugins_root()
            .ok_or_else(|| UdfError::Other(format!("workspace '{}' 没有 plugins_root", name)))?;
        let output_dir = plugins_root
            .parent()
            .unwrap_or(&plugins_root)
            .join("Artifacts/UnrealDevFlow/Plugins");
        (
            name,
            plugins_root,
            workspace_config.engine_path,
            output_dir,
            "workspace",
        )
    };
    let index = plugin_index(&plugins_root)?;
    let (seed_plugins, mut selected_index) = resolve_plugin_seeds(&plugins_root, &plugins, &index)?;
    let id = if mode.executes() {
        execution_id("plugin")
    } else {
        "package-plugin-query".to_string()
    };
    // UBT still rejects action paths longer than 260 characters on Windows.
    // Host/task artifact paths are often already deep, so keep the disposable
    // build stage in the system temp directory and copy only final artifacts
    // back into the managed output directory.
    let stage_root = plugin_stage_root(&id);
    let closure = plugin_closure(&plugins_root, &seed_plugins, &index, &mut selected_index)?;
    let commands =
        direct_plugin_commands(&engine_root, &stage_root, &seed_plugins, &selected_index);
    let log_dir = output_dir.join(".udf-logs").join(&id);
    let package_dirs = plugins
        .iter()
        .map(|plugin| output_dir.join(plugin))
        .collect::<Vec<_>>();
    if mode == PackageMode::Check {
        let selector = requested_task
            .map(|task| format!("--task {task}"))
            .unwrap_or_else(|| format!("--workspace {name}"));
        return emit_check(
            "plugin",
            source,
            &commands,
            None,
            format!("udf package plugin {} {selector}", plugins.join(" ")),
        );
    }
    if mode == PackageMode::Plan {
        let (_, diagnostics) = check_package_commands(&commands, None);
        return emit_plan("plugin", source, &commands, package_dirs, diagnostics);
    }
    let logs = if mode.executes() {
        prepare_plugin_stage(&plugins_root, &stage_root, &closure, &selected_index)?;
        let package_dirs = plugins
            .iter()
            .map(|plugin| output_dir.join(plugin))
            .collect::<Vec<_>>();
        let mut failure_cleanup = vec![stage_root.clone()];
        failure_cleanup.push(log_dir.clone());
        let logs = run_package_commands(
            &commands,
            &log_dir,
            &id,
            "plugin",
            &name,
            source,
            package_dirs,
            failure_cleanup,
        )?;
        for plugin in &plugins {
            let package_dir = output_dir.join(plugin);
            if package_dir.exists() {
                fs::remove_dir_all(&package_dir)?;
            }
            copy_tree(
                &stage_root.join("Plugins").join(plugin),
                &package_dir,
                false,
            )?;
        }
        logs
    } else {
        Vec::new()
    };
    let manifests = if mode.executes() {
        plugins
            .iter()
            .map(|plugin| write_manifest(&output_dir.join(plugin)))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let package_dirs = plugins
        .iter()
        .map(|plugin| output_dir.join(plugin))
        .collect::<Vec<_>>();
    let mut cleanup_targets = package_dirs.clone();
    cleanup_targets.push(stage_root);
    cleanup_targets.push(log_dir);
    if !mode.executes() {
        cleanup_targets.clear();
    }
    finish_execution(
        "package plugin",
        PackageResult {
            execution_id: id,
            action: "plugin".to_string(),
            workspace: name,
            source: source.to_string(),
            state: "succeeded".to_string(),
            exit_code: Some(0),
            commands: commands_as_argv(&commands),
            artifacts: package_dirs.clone(),
            outputs: package_dirs,
            logs,
            manifests,
            cleanup_targets,
            diagnostics: Vec::new(),
        },
    )
}

pub fn engine(workspace: Option<String>, mode: PackageMode) -> Result<()> {
    let config = Config::load()?;
    let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
    let output_dir = workspace_config
        .default_project
        .join("Saved")
        .join("UnrealDevFlow")
        .join("InstalledBuild");
    let commands = installed_build_commands(&InstalledBuildOptions {
        engine_root: workspace_config.engine_path,
        output_dir: output_dir.clone(),
        platform: UePlatform::Windows,
    });
    if mode == PackageMode::Check {
        return emit_check(
            "engine",
            "workspace",
            &commands,
            None,
            format!("udf package engine --workspace {name}"),
        );
    }
    if mode == PackageMode::Plan {
        let (_, diagnostics) = check_package_commands(&commands, None);
        return emit_plan(
            "engine",
            "workspace",
            &commands,
            vec![output_dir],
            diagnostics,
        );
    }
    let id = execution_id("engine");
    let log_dir = output_dir.join(".udf-logs").join(&id);
    let logs = if mode.executes() {
        run_package_commands(
            &commands,
            &log_dir,
            &id,
            "engine",
            &name,
            "workspace",
            vec![output_dir.clone()],
            vec![log_dir.clone()],
        )?
    } else {
        Vec::new()
    };
    let manifests = if mode.executes() {
        vec![write_manifest(&output_dir)?]
    } else {
        Vec::new()
    };
    finish_execution(
        "package engine",
        PackageResult {
            execution_id: id,
            action: "engine".to_string(),
            workspace: name,
            source: "workspace".to_string(),
            state: "succeeded".to_string(),
            exit_code: Some(0),
            commands: commands_as_argv(&commands),
            artifacts: vec![output_dir.clone()],
            outputs: vec![output_dir.clone()],
            logs,
            manifests,
            cleanup_targets: vec![output_dir],
            diagnostics: Vec::new(),
        },
    )
}

pub fn build_engine(workspace: Option<String>, plan_only: bool) -> Result<()> {
    let config = Config::load()?;
    let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
    let commands = engine_source_build_commands(&EngineSourceBuildOptions {
        engine_root: workspace_config.engine_path.clone(),
        platform: UePlatform::Windows,
        configuration: Configuration::Development,
        mutex: UbtMutexMode::Wait,
        gitdeps_threads: 16,
        gitdeps_cache: None,
        editor_target: "UnrealEditor".to_string(),
        extra_targets: Vec::new(),
    });
    let id = format!("build-engine-{}", Utc::now().format("%Y%m%dT%H%M%SZ"));
    let log_dir = workspace_config
        .default_project
        .join("Saved/UnrealDevFlow")
        .join(&id);
    if !plan_only && !Path::new(&commands[0].executable).is_file() {
        return Err(UdfError::Other(format!(
            "workspace '{}' 使用的引擎不是源码引擎，缺少 {}。可先运行 `udf build plan --workspace {}` 查看完整计划",
            name, commands[0].executable, name
        )));
    }
    let logs = if plan_only {
        Vec::new()
    } else {
        run_commands(&commands, &log_dir)?
    };
    let result = PackageResult {
        execution_id: id,
        action: "engine".to_string(),
        workspace: name,
        source: "workspace".to_string(),
        state: if plan_only { "planned" } else { "succeeded" }.to_string(),
        exit_code: if plan_only { None } else { Some(0) },
        commands: commands_as_argv(&commands),
        artifacts: vec![workspace_config.engine_path.clone()],
        outputs: vec![workspace_config.engine_path],
        logs,
        manifests: Vec::new(),
        cleanup_targets: Vec::new(),
        diagnostics: Vec::new(),
    };
    output::emit(
        if plan_only {
            "build plan"
        } else {
            "build engine"
        },
        result,
        render,
    );
    Ok(())
}

fn read_result(root: &Path, id: &str) -> Result<PackageResult> {
    let path = root.join(format!("{id}.json"));
    let bytes = fs::read(&path).map_err(|error| {
        UdfError::Other(format!(
            "找不到 package 执行记录 {}：{}",
            path.display(),
            error
        ))
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn is_real_execution_state(state: &str) -> bool {
    !matches!(state, "planned" | "ready" | "blocked" | "deferred")
}

fn load_result(execution_id: Option<&str>) -> Result<PackageResult> {
    let root = execution_root()?;
    if let Some(id) = execution_id {
        return read_result(&root, id);
    }

    if let Ok(latest_id) = fs::read_to_string(root.join("latest"))
        && let Ok(result) = read_result(&root, latest_id.trim())
        && is_real_execution_state(&result.state)
    {
        return Ok(result);
    }

    let mut candidates = fs::read_dir(&root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<PackageResult>(&bytes).ok())
        .filter(|result| is_real_execution_state(&result.state))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.execution_id.cmp(&right.execution_id));
    candidates.pop().ok_or_else(|| {
        UdfError::Other("找不到真实的 package 执行记录；check 和 plan 不属于执行状态".to_string())
    })
}

pub fn status(execution_id: Option<String>) -> Result<()> {
    let result = load_result(execution_id.as_deref())?;
    output::emit("package status", result, render);
    Ok(())
}

pub fn clean(execution_id: Option<String>) -> Result<()> {
    let mut result = load_result(execution_id.as_deref())?;
    if result.cleanup_targets.is_empty() {
        return Err(UdfError::Other(format!(
            "执行记录 '{}' 没有声明可清理目标",
            result.execution_id
        )));
    }
    for target in &result.cleanup_targets {
        let resolved = dunce::canonicalize(target).unwrap_or_else(|_| target.clone());
        let allowed = is_managed_cleanup_target(&resolved);
        if !allowed {
            return Err(UdfError::Other(format!(
                "拒绝清理未位于 UnrealDevFlow 制品目录内的路径：{}",
                target.display()
            )));
        }
        if target.is_dir() {
            fs::remove_dir_all(target)?;
        } else if target.is_file() {
            fs::remove_file(target)?;
        }
    }
    result.state = "cleaned".to_string();
    save_result(&result)?;
    output::emit("package clean", result, render);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_plugin_matrix_never_takes_engine_global_mutex() {
        let commands = direct_plugin_commands(
            Path::new("C:/UE"),
            Path::new("C:/stage/package-1"),
            &["AesWorld".to_string()],
            &BTreeMap::from([("AesWorld".to_string(), PathBuf::from("AesWorld"))]),
        );

        assert_eq!(commands.len(), 3);
        for command in commands {
            let argv = command.argv();
            assert!(argv.contains(&"-NoMutex".to_string()));
            assert!(!argv.contains(&"-WaitMutex".to_string()));
        }
    }

    #[test]
    fn plugin_stage_uses_short_managed_temp_root() {
        let stage = plugin_stage_root("package-plugin-1");

        assert_eq!(
            stage.parent(),
            Some(std::env::temp_dir().join("UDF").as_path())
        );
        assert_eq!(stage.file_name().unwrap().to_string_lossy().len(), 12);
        assert!(!stage.to_string_lossy().contains("Artifacts"));
        assert!(is_managed_cleanup_target(&stage));
        assert!(!is_managed_cleanup_target(&std::env::temp_dir()));
    }

    #[test]
    fn project_stage_disables_plugins_without_mutating_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("SourceProject");
        let stage = root.path().join("Stage");
        fs::create_dir_all(source.join("Plugins/AesWorld")).unwrap();
        fs::write(
            source.join("Game.uproject"),
            br#"{"FileVersion":3,"Plugins":[{"Name":"ModelContextProtocol","Enabled":true},{"Name":"AesWorld","Enabled":true}]}"#,
        ).unwrap();
        fs::write(source.join("Plugins/AesWorld/AesWorld.uplugin"), b"{} ").unwrap();
        let original = fs::read(source.join("Game.uproject")).unwrap();
        let staged = prepare_project_stage(
            &source.join("Game.uproject"),
            &stage,
            &["ModelContextProtocol".to_string()],
            &[],
        )
        .unwrap();
        let descriptor: serde_json::Value =
            serde_json::from_slice(&fs::read(staged).unwrap()).unwrap();
        assert_eq!(descriptor["Plugins"][0]["Enabled"], false);
        assert_eq!(descriptor["Plugins"][1]["Enabled"], true);
        assert_eq!(fs::read(source.join("Game.uproject")).unwrap(), original);
    }
}
