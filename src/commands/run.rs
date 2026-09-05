//! Controlled entry points for UE's native Editor, Commandlet and Gauntlet.
//!
//! The command deliberately owns only the boundary around native tools:
//! scope resolution, argv construction, a small execution record, and result
//! association.  Test discovery, process supervision and domain assertions
//! remain in Unreal's own tools.

use crate::config::{Config, sanitize_workspace_name};
use crate::error::{Result, UdfError};
use crate::host;
use crate::output;
use crate::run_profile::{
    self, EditorMode, ProfileIssue, ProfileScope, ProfileValidation, Rhi, RunBackend, RunProfile,
    RuntimeProject, ScopeKind, StoredRunProfile,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use sysinfo::System;

const RUN_SCHEMA_VERSION: u32 = 1;
static EXECUTION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const GAUNTLET_SOURCES: &[(&str, &str)] = &[
    (
        "Udf.Automation.csproj",
        include_str!("../../resources/gauntlet/Udf.Automation.csproj"),
    ),
    (
        "StrictExitRules.cs",
        include_str!("../../resources/gauntlet/StrictExitRules.cs"),
    ),
    (
        "EditorExitRulesTests.cs",
        include_str!("../../resources/gauntlet/EditorExitRulesTests.cs"),
    ),
    (
        "EditorExit.cs",
        include_str!("../../resources/gauntlet/EditorExit.cs"),
    ),
];

#[derive(Debug, Clone)]
struct ScopeResolution {
    scope: ProfileScope,
    profile_root: PathBuf,
    workspace_root: PathBuf,
    task_root: Option<PathBuf>,
    workspace_name: String,
    task_ref: Option<String>,
    engine: PathBuf,
    main_project: PathBuf,
    host_project: Option<PathBuf>,
    host_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRunTarget {
    pub source: String,
    pub workspace: String,
    pub task: Option<String>,
    pub project: PathBuf,
    pub engine: PathBuf,
    pub host: Option<PathBuf>,
    pub map: Option<String>,
    pub map_path: Option<PathBuf>,
    pub mode: Option<String>,
    pub rhi: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReason {
    pub code: String,
    pub message: String,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunCheckOutput {
    name: String,
    readiness: crate::execution::CheckState,
    reasons: Vec<RunReason>,
    resolved_target: ResolvedRunTarget,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigurationSummary {
    name: String,
    source: String,
    path: PathBuf,
    description: String,
    use_when: String,
    backend: RunBackend,
    required_inputs: Vec<ProfileIssue>,
    last_validation: Option<ValidationSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidationSummary {
    state: String,
    execution_id: String,
    revision: u64,
    digest: String,
    engine: PathBuf,
    project: PathBuf,
    matches_target: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TemplateSummary {
    name: String,
    backend: String,
    use_when: String,
    required_inputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunListOutput {
    configurations: Vec<ConfigurationSummary>,
    templates: Vec<TemplateSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigureOutput {
    name: String,
    scope: ProfileScope,
    path: PathBuf,
    revision: u64,
    digest: String,
    validation_state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExistingEditorCandidate {
    pid: u32,
    command_line: String,
    target_match: bool,
    start_time: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExistingEditorInfo {
    candidates: Vec<ExistingEditorCandidate>,
    selected_pid: Option<u32>,
    target_match: bool,
    current_map: Option<String>,
    actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunPlanOutput {
    name: String,
    source: String,
    resolved_target: ResolvedRunTarget,
    native_executable: PathBuf,
    native_argv: Vec<String>,
    display_command: String,
    result_path_template: PathBuf,
    existing_editor: Option<ExistingEditorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunArtifact {
    pub kind: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunExecutionRecord {
    pub schema_version: u32,
    pub execution_id: String,
    pub name: String,
    pub profile_revision: u64,
    pub profile_digest: String,
    pub scope: ProfileScope,
    pub resolved_target: ResolvedRunTarget,
    pub native_executable: PathBuf,
    pub native_argv: Vec<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub state: String,
    pub test_result: Option<String>,
    pub exit_result: Option<String>,
    pub business_result: Option<String>,
    pub tool_exit_code: Option<i32>,
    pub ue_exit_code: Option<i32>,
    pub pid: Option<u32>,
    pub artifacts: Vec<RunArtifact>,
    pub diagnostics: Vec<RunReason>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunCompareOutput {
    before_id: String,
    after_id: String,
    differences: Vec<String>,
    expectation: Option<String>,
    expectation_met: Option<bool>,
    before: RunExecutionRecord,
    after: RunExecutionRecord,
}

#[derive(Debug, Clone)]
struct PreparedRun {
    stored: StoredRunProfile,
    validation_root: PathBuf,
    resolution: ScopeResolution,
    target: ResolvedRunTarget,
    executable: PathBuf,
    argv: Vec<String>,
    readiness: crate::execution::CheckState,
    reasons: Vec<RunReason>,
}

fn invalid(message: impl Into<String>) -> UdfError {
    UdfError::InvalidConfig(message.into())
}

fn require_scope(
    workspace: Option<String>,
    task: Option<String>,
) -> Result<(Option<String>, Option<String>)> {
    match (workspace, task) {
        (Some(_), Some(_)) => Err(invalid("运行命令只能指定 --workspace 或 --task 其中一个")),
        (None, None) => Err(invalid("运行命令必须明确指定 --workspace 或 --task")),
        values => Ok(values),
    }
}

fn workspace_profile_root(config: &Config, name: &str) -> Result<PathBuf> {
    let (resolved, _) = config.resolve_workspace(Some(name))?;
    Ok(Config::config_dir()?
        .join("workspaces")
        .join(sanitize_workspace_name(&resolved))
        .join("run"))
}

fn resolve_scope(
    config: &Config,
    workspace: Option<String>,
    task: Option<String>,
) -> Result<ScopeResolution> {
    let (workspace, task) = require_scope(workspace, task)?;
    if let Some(task_ref) = task {
        let (host_dir, meta, context) = host::resolve_task(config, &task_ref)?;
        let workspace_name = context.workspace.clone();
        let workspace_root = Config::config_dir()?
            .join("workspaces")
            .join(sanitize_workspace_name(&workspace_name))
            .join("run");
        let host_project = host::resolve_host_uproject(&host_dir, &meta.id)?;
        let main_project = crate::commands::workspace::find_uproject(&context.default_project)
            .unwrap_or_else(|| context.default_project.join("<missing>.uproject"));
        let canonical_task = format!("{}/{}", workspace_name, meta.id);
        let task_root = host_dir.join(".udf").join("run");
        Ok(ScopeResolution {
            scope: ProfileScope {
                kind: ScopeKind::Task,
                value: canonical_task.clone(),
            },
            profile_root: task_root.clone(),
            workspace_root,
            task_root: Some(task_root),
            workspace_name,
            task_ref: Some(canonical_task),
            engine: context.engine_path,
            main_project,
            host_project: Some(host_project),
            host_dir: Some(host_dir),
        })
    } else {
        let requested = workspace.expect("require_scope checked workspace");
        let (workspace_name, workspace_config) = config.resolve_workspace(Some(&requested))?;
        let main_project =
            crate::commands::workspace::find_uproject(&workspace_config.default_project)
                .unwrap_or_else(|| workspace_config.default_project.join("<missing>.uproject"));
        let root = workspace_profile_root(config, &workspace_name)?;
        Ok(ScopeResolution {
            scope: ProfileScope {
                kind: ScopeKind::Workspace,
                value: workspace_name.clone(),
            },
            profile_root: root.clone(),
            workspace_root: root,
            task_root: None,
            workspace_name,
            task_ref: None,
            engine: workspace_config.engine_path,
            main_project,
            host_project: None,
            host_dir: None,
        })
    }
}

fn resolve_target(resolution: &ScopeResolution, profile: &RunProfile) -> ResolvedRunTarget {
    let project = match profile.project {
        RuntimeProject::Main => resolution.main_project.clone(),
        RuntimeProject::Host => resolution
            .host_project
            .clone()
            .unwrap_or_else(|| resolution.main_project.clone()),
    };
    let (mode, rhi) = profile
        .editor
        .as_ref()
        .map(|options| {
            (
                Some(match options.mode {
                    EditorMode::Editor => "editor".to_string(),
                    EditorMode::Game => "game".to_string(),
                }),
                Some(
                    match options.rhi {
                        Rhi::Default => "default",
                        Rhi::Null => "null",
                        Rhi::Dx11 => "dx11",
                        Rhi::Dx12 => "dx12",
                        Rhi::Vulkan => "vulkan",
                    }
                    .to_string(),
                ),
            )
        })
        .unwrap_or((None, None));
    ResolvedRunTarget {
        source: match resolution.scope.kind {
            ScopeKind::Task => "task".into(),
            ScopeKind::Workspace => "workspace".into(),
        },
        workspace: resolution.workspace_name.clone(),
        task: resolution.task_ref.clone(),
        project,
        engine: resolution.engine.clone(),
        host: resolution.host_dir.clone(),
        map: profile.map.clone(),
        map_path: None,
        mode,
        rhi,
    }
}

fn find_umap_by_stem(
    content: &Path,
    stem: &str,
    matches: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if !content.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(content)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(value) => value,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            find_umap_by_stem(&path, stem, matches)?;
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("umap"))
            && path
                .file_stem()
                .is_some_and(|value| value.eq_ignore_ascii_case(stem))
        {
            matches.push(path);
        }
    }
    Ok(())
}

fn resolve_map(project: &Path, map: Option<&str>) -> (Option<PathBuf>, Option<String>) {
    let Some(map) = map else { return (None, None) };
    if map.starts_with('/') && map.get(1..).is_some_and(|tail| tail.starts_with("Game/")) {
        let relative = map.trim_start_matches("/Game/").replace('/', "\\");
        let content = project.parent().unwrap_or(project).join("Content");
        let path = content.join(format!("{relative}.umap"));
        return if path.is_file() {
            (Some(path), None)
        } else {
            (
                None,
                Some(format!("地图不存在：{} ({})", map, path.display())),
            )
        };
    }
    let candidate = PathBuf::from(map);
    if candidate.is_absolute() {
        return if candidate.is_file() {
            (Some(candidate), None)
        } else {
            (
                None,
                Some(format!("地图文件不存在：{}", candidate.display())),
            )
        };
    }
    let content = project.parent().unwrap_or(project).join("Content");
    let mut matches = Vec::new();
    if let Err(error) = find_umap_by_stem(&content, map, &mut matches) {
        return (None, Some(format!("枚举地图失败：{error}")));
    }
    match matches.as_slice() {
        [one] => (Some(one.clone()), None),
        [] => (None, Some(format!("找不到地图短名：{map}"))),
        many => (
            None,
            Some(format!(
                "地图短名有多个候选：{}",
                many.iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        ),
    }
}

fn executable_for(engine: &Path, backend: RunBackend) -> PathBuf {
    match backend {
        RunBackend::Editor => engine
            .join("Engine")
            .join("Binaries")
            .join("Win64")
            .join("UnrealEditor.exe"),
        RunBackend::Commandlet => engine
            .join("Engine")
            .join("Binaries")
            .join("Win64")
            .join("UnrealEditor-Cmd.exe"),
        RunBackend::Gauntlet => engine
            .join("Engine")
            .join("Build")
            .join("BatchFiles")
            .join("RunUAT.bat"),
    }
}

fn add_editor_rhi(args: &mut Vec<String>, rhi: Rhi) {
    match rhi {
        Rhi::Default => {}
        Rhi::Null => args.push("-nullrhi".into()),
        Rhi::Dx11 => args.push("-dx11".into()),
        Rhi::Dx12 => args.push("-dx12".into()),
        Rhi::Vulkan => args.push("-vulkan".into()),
    }
}

fn build_argv(
    profile: &RunProfile,
    target: &ResolvedRunTarget,
    execution_id: Option<&str>,
    execution_root: Option<&Path>,
) -> Vec<String> {
    let mut args = Vec::new();
    match profile.backend {
        RunBackend::Editor => {
            args.push(target.project.to_string_lossy().to_string());
            if let Some(map) = &profile.map {
                args.push(map.clone());
            }
            if let Some(options) = &profile.editor {
                if options.mode == EditorMode::Game {
                    args.push("-game".into());
                    // Game smoke runs need a per-execution Engine log so the
                    // caller can prove the configured map loaded.  Keep the
                    // logging switches in argv; never scrape a global Saved/
                    // Logs directory.
                    args.push("-log".into());
                    args.push("-stdout".into());
                    args.push("-FullStdOutLogOutput".into());
                }
                add_editor_rhi(&mut args, options.rhi);
                if let Some(window) = &options.window {
                    match window.mode {
                        crate::run_profile::WindowMode::Windowed => args.push("-windowed".into()),
                        crate::run_profile::WindowMode::Fullscreen => {
                            args.push("-fullscreen".into())
                        }
                        crate::run_profile::WindowMode::Borderless => {
                            args.push("-borderless".into())
                        }
                    }
                    if let Some(width) = window.width {
                        args.push(format!("-resx={width}"));
                    }
                    if let Some(height) = window.height {
                        args.push(format!("-resy={height}"));
                    }
                }
                args.extend(options.native_args.iter().cloned());
            }
        }
        RunBackend::Commandlet => {
            args.push(target.project.to_string_lossy().to_string());
            if let Some(map) = &profile.map {
                args.push(format!("-map={map}"));
            }
            if let Some(options) = &profile.commandlet {
                args.push(format!("-run={}", options.name));
                args.extend(options.native_args.iter().cloned());
            }
        }
        RunBackend::Gauntlet => {
            if let Some(options) = &profile.gauntlet {
                args.push("RunUnreal".into());
                args.push(format!("-project={}", target.project.display()));
                args.push(format!("-build={}", options.build));
                args.push(format!("-test={}", options.test));
                args.push(format!("-platform={}", options.platform));
                args.push(format!("-configuration={}", options.configuration));
                if options.null_rhi {
                    args.push("-NullRHI".into());
                }
                if options.unattended {
                    args.push("-Unattended".into());
                }
                if let Some(max) = options.max_duration_seconds {
                    args.push(format!("-MaxDuration={max}"));
                }
                if let Some(filter) = &options.run_test {
                    args.push(format!("-RunTest={filter}"));
                }
                if !options.exec_cmds.is_empty() {
                    let key = if options.test.eq_ignore_ascii_case("Udf.EditorExit") {
                        "-UdfSyncCmds"
                    } else {
                        "-ExecCmds"
                    };
                    args.push(format!("{key}={}", options.exec_cmds.join(",")));
                }
                if options.test.eq_ignore_ascii_case("Udf.EditorExit") {
                    let root = execution_root
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| PathBuf::from("<executionRoot>"));
                    args.push(format!("-ScriptDir={}", root.join("gauntlet").display()));
                }
                if let Some(root) = execution_root {
                    args.push(format!("-TempDir={}", root.join("temp").display()));
                    args.push(format!("-LogDir={}", root.join("logs").display()));
                } else {
                    args.push("-TempDir=<executionRoot>\\temp".into());
                    args.push("-LogDir=<executionRoot>\\logs".into());
                }
                args.extend(options.native_args.iter().cloned());
            }
        }
    }
    if let Some(id) = execution_id
        && !args
            .iter()
            .any(|arg| arg.to_ascii_lowercase().starts_with("-abslog="))
    {
        let path = execution_root
            .map(|root| root.join("native.log"))
            .unwrap_or_else(|| PathBuf::from(format!("<executionRoot>\\{id}.log")));
        args.push(format!("-abslog={}", path.display()));
    }
    args
}

fn native_display(executable: &Path, argv: &[String]) -> String {
    let quote = |arg: String| {
        if arg.is_empty() || arg.chars().any(char::is_whitespace) || arg.contains('"') {
            format!("\"{}\"", arg.replace('"', "\\\""))
        } else {
            arg
        }
    };
    std::iter::once(quote(executable.to_string_lossy().to_string()))
        .chain(argv.iter().cloned().map(quote))
        .collect::<Vec<_>>()
        .join(" ")
}

fn prepared_run(
    config: &Config,
    name: &str,
    workspace: Option<String>,
    task: Option<String>,
) -> Result<PreparedRun> {
    run_profile::validate_name(name)?;
    let resolution = resolve_scope(config, workspace, task)?;
    let selected = run_profile::select_profile(
        resolution.task_root.as_deref(),
        &resolution.workspace_root,
        name,
    )?
    .ok_or_else(|| {
        invalid(format!(
            "找不到运行配置 '{name}'；先执行 `udf run configure`"
        ))
    })?;
    let profile = &selected.stored.profile;
    let mut target = resolve_target(&resolution, profile);
    let (map_path, map_error) = resolve_map(&target.project, profile.map.as_deref());
    target.map_path = map_path;
    let executable = executable_for(&resolution.engine, profile.backend);
    let mut reasons = Vec::new();
    for issue in profile.required_inputs() {
        reasons.push(RunReason {
            code: issue.code,
            message: issue.message,
            next_action: "补齐配置后重新 check".into(),
        });
    }
    if map_error.is_some() {
        reasons.push(RunReason {
            code: "map_not_resolved".into(),
            message: map_error.unwrap_or_default(),
            next_action: "使用 /Game/... 精确地图路径或消除同名候选".into(),
        });
    }
    if !target.project.is_file() {
        reasons.push(RunReason {
            code: "project_missing".into(),
            message: format!("项目 .uproject 不存在：{}", target.project.display()),
            next_action: "检查 workspace/task 的冻结项目路径".into(),
        });
    }
    if !resolution.engine.is_dir() {
        reasons.push(RunReason {
            code: "engine_missing".into(),
            message: format!("引擎目录不存在：{}", resolution.engine.display()),
            next_action: "运行 `udf workspace doctor` 修正引擎路径".into(),
        });
    }
    if !executable.is_file() {
        reasons.push(RunReason {
            code: "native_tool_missing".into(),
            message: format!("原生入口不存在：{}", executable.display()),
            next_action: "确认 Engine 路径和对应安装组件".into(),
        });
    }
    let readiness = if reasons.is_empty() {
        crate::execution::CheckState::Ready
    } else if reasons
        .iter()
        .any(|reason| reason.code == "native_tool_missing")
    {
        crate::execution::CheckState::Blocked
    } else {
        crate::execution::CheckState::NeedsUserInput
    };
    let argv = build_argv(profile, &target, None, None);
    Ok(PreparedRun {
        stored: selected.stored,
        validation_root: selected
            .path
            .parent()
            .unwrap_or(&resolution.profile_root)
            .to_path_buf(),
        resolution,
        target,
        executable,
        argv,
        readiness,
        reasons,
    })
}

fn templates() -> Vec<TemplateSummary> {
    vec![
        TemplateSummary {
            name: "editor-boot".into(),
            backend: "gauntlet".into(),
            use_when: "确认 UE Editor 能启动并自然结束".into(),
            required_inputs: vec!["task 或 workspace".into()],
        },
        TemplateSummary {
            name: "editor-exit".into(),
            backend: "gauntlet".into(),
            use_when: "验证原生 Editor 退出与 UE 原始退出码".into(),
            required_inputs: vec!["task".into(), "Udf.EditorExit 资源".into()],
        },
        TemplateSummary {
            name: "automation-smoke".into(),
            backend: "gauntlet".into(),
            use_when: "调用 UE.EditorAutomation 的精确测试过滤器".into(),
            required_inputs: vec!["runTest".into()],
        },
        TemplateSummary {
            name: "game".into(),
            backend: "editor".into(),
            use_when: "用指定地图启动 Game".into(),
            required_inputs: vec!["map".into()],
        },
        TemplateSummary {
            name: "commandlet".into(),
            backend: "commandlet".into(),
            use_when: "运行 UE 原生 Commandlet".into(),
            required_inputs: vec!["commandlet.name".into()],
        },
    ]
}

fn load_summaries(resolution: &ScopeResolution) -> Result<Vec<ConfigurationSummary>> {
    let mut roots = Vec::new();
    if let Some(task_root) = &resolution.task_root {
        roots.push(("task".to_string(), task_root.clone()));
    }
    roots.push(("workspace".to_string(), resolution.workspace_root.clone()));
    let mut names = BTreeSet::new();
    for (_, root) in &roots {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                    && let Some(stem) = entry.path().file_stem().and_then(|value| value.to_str())
                {
                    names.insert(stem.to_string());
                }
            }
        }
    }
    let mut output = Vec::new();
    for name in names {
        if let Some(selected) = run_profile::select_profile(
            resolution.task_root.as_deref(),
            &resolution.workspace_root,
            &name,
        )? {
            let validation =
                selected
                    .stored
                    .last_validation
                    .as_ref()
                    .map(|value| ValidationSummary {
                        state: value.state.clone(),
                        execution_id: value.execution_id.clone(),
                        revision: value.revision,
                        digest: value.digest.clone(),
                        engine: value.engine.clone(),
                        project: value.project.clone(),
                        matches_target: value.engine == resolution.engine
                            && value.project
                                == resolve_target(resolution, &selected.stored.profile).project,
                    });
            output.push(ConfigurationSummary {
                name: selected.stored.name.clone(),
                source: if selected.inherited {
                    "workspace".into()
                } else {
                    match resolution.scope.kind {
                        ScopeKind::Task => "task".into(),
                        ScopeKind::Workspace => "workspace".into(),
                    }
                },
                path: selected.path,
                description: selected.stored.profile.description.clone(),
                use_when: selected.stored.profile.use_when.clone(),
                backend: selected.stored.profile.backend,
                required_inputs: selected.stored.profile.required_inputs(),
                last_validation: validation,
            });
        }
    }
    Ok(output)
}

pub fn list(workspace: Option<String>, task: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let resolution = resolve_scope(&config, workspace, task)?;
    let out = RunListOutput {
        configurations: load_summaries(&resolution)?,
        templates: templates(),
    };
    output::emit("run list", out, |data| {
        let mut lines = Vec::new();
        if data.configurations.is_empty() {
            lines.push("没有已登记配置；可从 templates 选择用途".into());
        } else {
            for item in &data.configurations {
                lines.push(format!("{} [{}] {}", item.name, item.source, item.use_when));
            }
        }
        lines.push(format!(
            "内置模板：{}",
            data.templates
                .iter()
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        lines.join("\n")
    });
    Ok(())
}

pub fn configure(
    name: String,
    workspace: Option<String>,
    task: Option<String>,
    file: PathBuf,
    reason: Option<String>,
) -> Result<()> {
    let config = Config::load()?;
    let resolution = resolve_scope(&config, workspace, task)?;
    let text = fs::read_to_string(&file)?;
    let candidate = run_profile::parse_candidate(&text)?;
    let stored = run_profile::save_profile(
        &resolution.profile_root,
        &name,
        &resolution.scope,
        &candidate,
        reason.as_deref(),
        None,
    )?;
    let out = ConfigureOutput {
        name: stored.name.clone(),
        scope: stored.scope.clone(),
        path: resolution.profile_root.join(format!("{name}.json")),
        revision: stored.revision,
        digest: stored.digest.clone(),
        validation_state: stored.validation_state().into(),
    };
    output::emit("run configure", out, |data| {
        format!(
            "已登记 {} revision={} digest={}\n路径：{}",
            data.name,
            data.revision,
            data.digest,
            data.path.display()
        )
    });
    Ok(())
}

pub fn check(name: String, workspace: Option<String>, task: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let prepared = prepared_run(&config, &name, workspace, task)?;
    let out = RunCheckOutput {
        name,
        readiness: prepared.readiness,
        reasons: prepared.reasons,
        resolved_target: prepared.target,
    };
    output::emit("run check", out, |data| {
        format!(
            "运行检查：{:?}\n项目：{}",
            data.readiness,
            data.resolved_target.project.display()
        )
    });
    Ok(())
}

fn editor_candidates(project: &Path) -> Vec<ExistingEditorCandidate> {
    let mut system = System::new_all();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let needle = project.to_string_lossy().to_ascii_lowercase();
    let mut result = system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let name = process.name().to_string_lossy();
            if !name.to_ascii_lowercase().contains("unrealeditor") {
                return None;
            }
            let command_line = process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let target_match = command_line.to_ascii_lowercase().contains(&needle);
            Some(ExistingEditorCandidate {
                pid: pid.as_u32(),
                command_line,
                target_match,
                start_time: process.start_time(),
            })
        })
        .collect::<Vec<_>>();
    result.sort_by_key(|item| item.pid);
    result
}

pub fn plan(
    name: String,
    workspace: Option<String>,
    task: Option<String>,
    existing_editor: bool,
    pid: Option<u32>,
) -> Result<()> {
    let config = Config::load()?;
    let prepared = prepared_run(&config, &name, workspace, task)?;
    let mut editor_info = None;
    if existing_editor {
        let candidates = editor_candidates(&prepared.target.project);
        let selected_pid = pid.or_else(|| {
            let matching = candidates
                .iter()
                .filter(|item| item.target_match)
                .collect::<Vec<_>>();
            if matching.len() == 1 {
                Some(matching[0].pid)
            } else {
                None
            }
        });
        if let Some(requested) = pid
            && !candidates.iter().any(|item| item.pid == requested)
        {
            return Err(invalid(format!("指定的 Editor PID 不存在：{requested}")));
        }
        editor_info = Some(ExistingEditorInfo {
            target_match: selected_pid.is_some_and(|selected| {
                candidates
                    .iter()
                    .any(|item| item.pid == selected && item.target_match)
            }),
            candidates,
            selected_pid,
            current_map: None,
            actions: vec![
                "使用 Session Frontend 连接已选 Editor".into(),
                "无法读取当前地图时不要把启动参数当作当前状态".into(),
                "未连接控制接口时只复制原生控制台命令，不声称已执行".into(),
            ],
        });
    }
    let argv = build_argv(&prepared.stored.profile, &prepared.target, None, None);
    let template_root = Config::config_dir()?
        .join("executions")
        .join("run")
        .join("{executionId}");
    let out = RunPlanOutput {
        name,
        source: if prepared.stored.scope.kind == ScopeKind::Task {
            "task".into()
        } else {
            "workspace".into()
        },
        resolved_target: prepared.target.clone(),
        native_executable: prepared.executable.clone(),
        native_argv: argv.clone(),
        display_command: native_display(&prepared.executable, &argv),
        result_path_template: template_root,
        existing_editor: editor_info,
    };
    output::emit("run plan", out, |data| {
        format!(
            "原生入口：{}\n{}",
            data.native_executable.display(),
            data.display_command
        )
    });
    Ok(())
}

fn execution_root(execution_id: &str) -> Result<PathBuf> {
    if execution_id.is_empty()
        || !execution_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || value == b'-' || value == b'_')
    {
        return Err(invalid(format!("非法 execution ID：{execution_id}")));
    }
    Ok(Config::config_dir()?
        .join("executions")
        .join("run")
        .join(execution_id))
}

fn write_execution(record: &RunExecutionRecord) -> Result<PathBuf> {
    let root = execution_root(&record.execution_id)?;
    fs::create_dir_all(&root)?;
    let path = root.join("record.json");
    let temp = root.join(format!(".record.{}.tmp", std::process::id()));
    let mut bytes = serde_json::to_vec_pretty(record)?;
    bytes.push(b'\n');
    fs::write(&temp, bytes)?;
    fs::rename(&temp, &path)?;
    Ok(path)
}

fn load_execution(execution_id: &str) -> Result<RunExecutionRecord> {
    let root = execution_root(execution_id)?;
    let path = root.join("record.json");
    let bytes = fs::read(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            UdfError::Other(format!("找不到 execution：{execution_id}"))
        } else {
            error.into()
        }
    })?;
    let record: RunExecutionRecord = serde_json::from_slice(&bytes)?;
    if record.schema_version != RUN_SCHEMA_VERSION || record.execution_id != execution_id {
        return Err(invalid(format!(
            "execution 记录身份或版本无效：{}",
            path.display()
        )));
    }
    Ok(record)
}

fn new_execution_id(name: &str) -> String {
    // Keep the ID a portable path component; the milliseconds format has no
    // punctuation that would be rejected by execution_root validation.
    let stamp = Utc::now().format("%Y%m%dT%H%M%S%3fZ");
    let sequence = EXECUTION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("run-{name}-{}-{}-{}", stamp, std::process::id(), sequence)
}

fn prepare_gauntlet_resources(root: &Path) -> Result<PathBuf> {
    let gauntlet = root.join("gauntlet");
    fs::create_dir_all(&gauntlet)?;
    for (name, content) in GAUNTLET_SOURCES {
        fs::write(gauntlet.join(name), content)?;
    }
    Ok(gauntlet)
}

fn spawn_native(executable: &Path, argv: &[String], log: &Path) -> Result<Child> {
    let parent = log
        .parent()
        .ok_or_else(|| invalid("原生日志路径缺少父目录"))?;
    fs::create_dir_all(parent)?;
    let stdout = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log)?;
    let stderr = stdout.try_clone()?;
    let mut command = if executable.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd")
    }) {
        let mut command = Command::new("cmd.exe");
        command.arg("/d").arg("/c").arg("call").arg(executable);
        command
    } else {
        Command::new(executable)
    };
    command
        .args(argv)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    command.spawn().map_err(|error| {
        UdfError::Other(format!(
            "启动原生入口失败 {}：{error}",
            executable.display()
        ))
    })
}

fn log_text(path: &Path) -> String {
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let mut content = String::new();
    let _ = file.read_to_string(&mut content);
    if content.len() > 4 * 1024 * 1024 {
        content.split_off(content.len() - 4 * 1024 * 1024)
    } else {
        content
    }
}

fn find_report(root: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_report(&path) {
                return Some(found);
            }
        } else if path
            .file_name()
            .is_some_and(|name| name == "udf-editor-exit.json")
        {
            return Some(path);
        }
    }
    None
}

type ChildEvaluation = (
    String,
    Option<String>,
    Option<String>,
    Option<i32>,
    Vec<RunArtifact>,
    Vec<RunReason>,
);

fn report_exit_code(report: &serde_json::Value) -> Option<i32> {
    report
        .get("roles")
        .and_then(|roles| roles.as_array())
        .and_then(|roles| {
            roles
                .iter()
                .find_map(|role| role.get("rawExitCode").and_then(|value| value.as_i64()))
        })
        .and_then(|value| i32::try_from(value).ok())
}

fn evaluate_child(
    profile: &RunProfile,
    code: i32,
    log_path: &Path,
    execution_root: &Path,
) -> ChildEvaluation {
    let mut artifacts = vec![RunArtifact {
        kind: "nativeLog".into(),
        path: log_path.to_path_buf(),
    }];
    let mut diagnostics = Vec::new();
    let text = log_text(log_path);
    let report_path = find_report(execution_root);
    let mut report_value = None;
    let mut ue_exit_code = None;
    if let Some(path) = &report_path {
        artifacts.push(RunArtifact {
            kind: "nativeReport".into(),
            path: path.clone(),
        });
        if let Ok(bytes) = fs::read(path)
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
        {
            ue_exit_code = report_exit_code(&value);
            report_value = value
                .get("result")
                .and_then(|result| result.as_str())
                .map(str::to_string);
        }
    }
    match profile.backend {
        RunBackend::Commandlet => {
            let result = if code == 0 { "passed" } else { "failed" };
            (
                result.into(),
                Some(result.into()),
                Some(result.into()),
                None,
                artifacts,
                diagnostics,
            )
        }
        RunBackend::Gauntlet => {
            let passed_marker = report_value.as_deref() == Some("Passed")
                || text.contains("result=Passed")
                || text.contains("Test Udf.EditorExit Passed")
                || text.contains("Test UE.EditorAutomation Passed")
                || text.contains("Test UE.EditorBootTest Passed");
            let failed_marker = report_value.as_deref() == Some("Failed")
                || text.contains("result=Failed")
                || text.contains("Test Udf.EditorExit Failed")
                || text.contains("Test UE.EditorAutomation Failed")
                || text.contains("Test UE.EditorBootTest Failed");
            let result = if code != 0 || failed_marker {
                "failed"
            } else if passed_marker {
                "passed"
            } else {
                diagnostics.push(RunReason {
                    code: "native_result_missing".into(),
                    message: "原生工具返回成功但没有最终测试结果".into(),
                    next_action: "检查 Gauntlet 报告和测试节点是否完成终止".into(),
                });
                "unknown"
            };
            let exit_result = if profile
                .gauntlet
                .as_ref()
                .is_some_and(|options| options.test.eq_ignore_ascii_case("Udf.EditorExit"))
            {
                Some(match ue_exit_code {
                    Some(0) if passed_marker => "passed".into(),
                    Some(_) => "failed".into(),
                    None => "unknown".into(),
                })
            } else {
                None
            };
            (
                result.into(),
                Some(result.into()),
                exit_result,
                ue_exit_code,
                artifacts,
                diagnostics,
            )
        }
        RunBackend::Editor => {
            diagnostics.push(RunReason {
                code: "editor_detached".into(),
                message: "Editor 已启动；UDF 不把进程创建当作测试通过".into(),
                next_action: "使用原生 Editor 或 Session Frontend 继续操作".into(),
            });
            ("started".into(), None, None, None, artifacts, diagnostics)
        }
    }
}

fn record_validation_safely(
    prepared: &PreparedRun,
    execution_id: &str,
    state: &str,
) -> Option<String> {
    let validation = ProfileValidation {
        revision: prepared.stored.revision,
        digest: prepared.stored.digest.clone(),
        execution_id: execution_id.into(),
        state: state.into(),
        engine: prepared.resolution.engine.clone(),
        project: prepared.target.project.clone(),
        validated_at: Utc::now().to_rfc3339(),
    };
    run_profile::record_validation(&prepared.validation_root, &prepared.stored.name, validation)
        .err()
        .map(|error| error.to_string())
}

pub fn start(name: String, workspace: Option<String>, task: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let prepared = prepared_run(&config, &name, workspace, task)?;
    if prepared.readiness != crate::execution::CheckState::Ready {
        let out = RunCheckOutput {
            name,
            readiness: prepared.readiness,
            reasons: prepared.reasons.clone(),
            resolved_target: prepared.target,
        };
        output::emit_failure_with_data("run start", out, "运行前检查未通过；未创建 execution 记录");
        return Err(UdfError::Other("运行前检查未通过".into()));
    }

    if prepared.stored.profile.backend == RunBackend::Editor {
        let candidates = editor_candidates(&prepared.target.project);
        if candidates.iter().any(|candidate| candidate.target_match) {
            let out = RunCheckOutput {
                name,
                readiness: crate::execution::CheckState::Blocked,
                reasons: vec![RunReason {
                    code: "editor_already_running".into(),
                    message: "已有匹配项目的 Editor 实例；为避免重复启动，UDF 只提供已有实例路线"
                        .into(),
                    next_action: "运行 `udf run plan ... --existing-editor` 查看 PID 和原生操作"
                        .into(),
                }],
                resolved_target: prepared.target,
            };
            output::emit_failure_with_data("run start", out, "已有匹配 Editor，未重复启动");
            return Err(UdfError::Other("已有匹配 Editor，未重复启动".into()));
        }
    }

    let execution_id = new_execution_id(&prepared.stored.name);
    let root = execution_root(&execution_id)?;
    fs::create_dir_all(&root)?;
    if prepared.stored.profile.backend == RunBackend::Gauntlet
        && prepared
            .stored
            .profile
            .gauntlet
            .as_ref()
            .is_some_and(|options| options.test.eq_ignore_ascii_case("Udf.EditorExit"))
    {
        prepare_gauntlet_resources(&root)?;
    }
    let argv = build_argv(
        &prepared.stored.profile,
        &prepared.target,
        Some(&execution_id),
        Some(&root),
    );
    let log_path = root.join("native.log");
    let mut record = RunExecutionRecord {
        schema_version: RUN_SCHEMA_VERSION,
        execution_id: execution_id.clone(),
        name: prepared.stored.name.clone(),
        profile_revision: prepared.stored.revision,
        profile_digest: prepared.stored.digest.clone(),
        scope: prepared.stored.scope.clone(),
        resolved_target: prepared.target.clone(),
        native_executable: prepared.executable.clone(),
        native_argv: argv.clone(),
        started_at: Utc::now().to_rfc3339(),
        finished_at: None,
        state: "running".into(),
        test_result: None,
        exit_result: None,
        business_result: None,
        tool_exit_code: None,
        ue_exit_code: None,
        pid: None,
        artifacts: vec![RunArtifact {
            kind: "nativeLog".into(),
            path: log_path.clone(),
        }],
        diagnostics: Vec::new(),
    };
    write_execution(&record)?;
    let mut child = match spawn_native(&prepared.executable, &argv, &log_path) {
        Ok(child) => child,
        Err(error) => {
            record.state = "failed".into();
            record.finished_at = Some(Utc::now().to_rfc3339());
            record.diagnostics.push(RunReason {
                code: "spawn_failed".into(),
                message: error.to_string(),
                next_action: "检查原生入口路径和权限".into(),
            });
            write_execution(&record)?;
            output::emit_failure_with_data("run start", record, &error.to_string());
            return Err(error);
        }
    };
    record.pid = Some(child.id());
    if prepared.stored.profile.backend == RunBackend::Editor {
        record.state = "started".into();
        record.finished_at = None;
        write_execution(&record)?;
        output::emit("run start", record, |data| {
            format!(
                "已启动 {}，PID={}\n日志：{}",
                data.name,
                data.pid.unwrap_or_default(),
                data.artifacts
                    .first()
                    .map(|item| item.path.display().to_string())
                    .unwrap_or_default()
            )
        });
        return Ok(());
    }

    let status = child.wait()?;
    let code = status.code().unwrap_or(-1);
    let (state, test_result, exit_result, ue_exit_code, artifacts, mut diagnostics) =
        evaluate_child(&prepared.stored.profile, code, &log_path, &root);
    record.state = state.clone();
    record.finished_at = Some(Utc::now().to_rfc3339());
    record.test_result = test_result;
    record.exit_result = exit_result;
    record.tool_exit_code = Some(code);
    record.ue_exit_code = ue_exit_code;
    record.pid = None;
    record.artifacts = artifacts;
    record.diagnostics.append(&mut diagnostics);
    if let Some(error) = record_validation_safely(&prepared, &execution_id, &state) {
        record.diagnostics.push(RunReason {
            code: "validation_record_failed".into(),
            message: error,
            next_action: "检查配置目录权限后运行 status".into(),
        });
    }
    write_execution(&record)?;
    if state == "passed" {
        output::emit("run start", record, |data| {
            format!(
                "原生测试通过：{}\nexecution：{}",
                data.name, data.execution_id
            )
        });
        Ok(())
    } else {
        let message = format!("原生执行结果：{}；execution={execution_id}", state);
        output::emit_failure_with_data("run start", record, &message);
        Err(UdfError::Other(message))
    }
}

fn all_execution_ids() -> Result<Vec<String>> {
    let root = Config::config_dir()?.join("executions").join("run");
    let mut ids = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir()
                && let Some(id) = entry.file_name().to_str()
                && entry.path().join("record.json").is_file()
            {
                ids.push(id.to_string());
            }
        }
    }
    Ok(ids)
}

pub fn status(
    execution_id: Option<String>,
    workspace: Option<String>,
    task: Option<String>,
) -> Result<()> {
    let id = if let Some(id) = execution_id {
        id
    } else {
        let config = Config::load()?;
        let resolution = resolve_scope(&config, workspace, task)?;
        let mut records = all_execution_ids()?
            .into_iter()
            .filter_map(|id| load_execution(&id).ok())
            .filter(|record| {
                record.scope == resolution.scope
                    || (resolution.scope.kind == ScopeKind::Task
                        && record.resolved_target.task == Some(resolution.scope.value.clone()))
            })
            .collect::<Vec<_>>();
        records.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        records
            .pop()
            .map(|record| record.execution_id)
            .ok_or_else(|| invalid("该作用域还没有 execution 记录"))?
    };
    let mut record = load_execution(&id)?;
    if record.state == "running" {
        let alive = record.pid.is_some_and(|pid| {
            let mut system = System::new();
            system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            system.process(sysinfo::Pid::from_u32(pid)).is_some()
        });
        if !alive {
            record.state = "unknown".into();
            record.diagnostics.push(RunReason {
                code: "missing_final_observation".into(),
                message: "记录显示原生进程已不在，但没有最终报告；未推断为通过".into(),
                next_action: "检查 nativeLog/nativeReport，必要时重新 start".into(),
            });
        }
    }
    output::emit("run status", record, |data| {
        format!(
            "{}: {}\n日志：{}",
            data.execution_id,
            data.state,
            data.artifacts
                .first()
                .map(|item| item.path.display().to_string())
                .unwrap_or_default()
        )
    });
    Ok(())
}

pub fn compare(before_id: String, after_id: String, expect: Option<String>) -> Result<()> {
    let before = load_execution(&before_id)?;
    let after = load_execution(&after_id)?;
    let mut differences = Vec::new();
    if before.resolved_target.project != after.resolved_target.project {
        differences.push(format!(
            "project: {} -> {}",
            before.resolved_target.project.display(),
            after.resolved_target.project.display()
        ));
    }
    if before.resolved_target.engine != after.resolved_target.engine {
        differences.push(format!(
            "engine: {} -> {}",
            before.resolved_target.engine.display(),
            after.resolved_target.engine.display()
        ));
    }
    if before.resolved_target.map != after.resolved_target.map {
        differences.push(format!(
            "map: {:?} -> {:?}",
            before.resolved_target.map, after.resolved_target.map
        ));
    }
    if before.profile_digest != after.profile_digest {
        differences.push("profileDigest 不同".into());
    }
    if before.state != after.state {
        differences.push(format!("state: {} -> {}", before.state, after.state));
    }
    let expectation_met = expect.as_deref().map(|value| {
        value == "pass-after-fail"
            && before.state == "failed"
            && after.state == "passed"
            && before.profile_digest == after.profile_digest
            && before.resolved_target.project == after.resolved_target.project
            && before.resolved_target.engine == after.resolved_target.engine
            && before.resolved_target.map == after.resolved_target.map
            && before.resolved_target.mode == after.resolved_target.mode
            && before.resolved_target.rhi == after.resolved_target.rhi
    });
    let out = RunCompareOutput {
        before_id: before_id.clone(),
        after_id: after_id.clone(),
        differences,
        expectation: expect.clone(),
        expectation_met,
        before,
        after,
    };
    if expectation_met == Some(false) {
        let message = "compare 期望 pass-after-fail 未满足";
        output::emit_failure_with_data("run compare", out, message);
        return Err(UdfError::Other(message.into()));
    }
    output::emit("run compare", out, |data| {
        let mut lines = vec![format!("{} -> {}", data.before_id, data.after_id)];
        lines.extend(data.differences.iter().cloned());
        if let Some(value) = data.expectation_met {
            lines.push(format!("期望满足：{value}"));
        }
        lines.join("\n")
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_profile::{EditorOptions, Rhi};

    fn editor_profile() -> RunProfile {
        RunProfile {
            schema_version: 1,
            description: "打开编辑器".into(),
            use_when: "smoke".into(),
            project: RuntimeProject::Main,
            backend: RunBackend::Editor,
            map: Some("/Game/Maps/Test".into()),
            editor: Some(EditorOptions {
                mode: EditorMode::Game,
                rhi: Rhi::Null,
                window: None,
                native_args: vec![],
            }),
            commandlet: None,
            gauntlet: None,
        }
    }

    #[test]
    fn build_editor_argv_keeps_project_and_map_as_separate_arguments() {
        let profile = editor_profile();
        let target = ResolvedRunTarget {
            source: "workspace".into(),
            workspace: "demo".into(),
            task: None,
            project: PathBuf::from("C:\\UE Projects\\Demo.uproject"),
            engine: PathBuf::from("C:\\UE"),
            host: None,
            map: profile.map.clone(),
            map_path: None,
            mode: Some("game".into()),
            rhi: Some("null".into()),
        };
        let argv = build_argv(&profile, &target, None, None);
        assert_eq!(argv[0], "C:\\UE Projects\\Demo.uproject");
        assert_eq!(argv[1], "/Game/Maps/Test");
        assert!(argv.contains(&"-game".into()));
        assert!(argv.contains(&"-nullrhi".into()));
    }

    #[test]
    fn display_command_quotes_spaces_without_becoming_execution_input() {
        let text = native_display(
            Path::new("C:\\UE Projects\\RunUAT.bat"),
            &[
                "-test=UE.EditorBootTest".into(),
                "-project=C:\\UE Projects\\Demo.uproject".into(),
            ],
        );
        assert!(text.contains("\"C:\\UE Projects\\RunUAT.bat\""));
        assert!(text.contains("\"-project=C:\\UE Projects\\Demo.uproject\""));
    }
}
