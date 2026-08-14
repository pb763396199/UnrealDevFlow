//! Controlled build commands: `build check`, `build gate`, `build project`.
//!
//! These sit in front of the Unreal build so that nobody hand-assembles a
//! `Build.bat` line. `build check` reports whether a build may start now,
//! `build gate` refuses commands that bypass this path, and `build project`
//! builds the workspace's main project rather than a task Host.

use std::path::PathBuf;

use md5::{Digest, Md5};
use serde::Serialize;

use crate::build_policy::{
    self, BuildExecution, BuildGateReport, BuildPolicyReport, BuildPolicyRequest, BuildPolicyStatus,
};
use crate::build_profile::BuildProfile;
use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::host;
use crate::output;

/// What a build policy report is about: one task Host, or a whole workspace.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildSubject {
    /// `task` or `workspace`.
    kind: &'static str,
    name: String,
    project_path: PathBuf,
    engine_root: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildCheckOutput {
    domain: &'static str,
    action: &'static str,
    source: BuildSubject,
    readiness: BuildPolicyStatus,
    checks: Vec<String>,
    diagnostics: Vec<String>,
    next_command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildPlanOutput {
    domain: &'static str,
    action: &'static str,
    source: BuildSubject,
    plan_digest: String,
    steps: Vec<crate::execution::ExecutionStep>,
    outputs: Vec<PathBuf>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildProjectOutput {
    subject: BuildSubject,
    execution: BuildExecution,
}

/// Report whether a controlled build may start right now. Never starts one.
///
/// The command itself succeeds even when the verdict is `blocked`: the verdict
/// is the answer, not a failure to answer.
pub fn check(
    task_ref: Option<String>,
    workspace: Option<String>,
    profile: BuildProfile,
    target: Option<String>,
    build_command: Option<String>,
) -> Result<()> {
    let mut request = BuildPolicyRequest {
        build_profile: Some(profile.label().to_string()),
        build_target: target,
        build_command,
        ..BuildPolicyRequest::default()
    };
    let subject = resolve_subject(task_ref, workspace, &mut request)?;
    let report = build_policy::resolve_build_policy(&request);
    let out = BuildCheckOutput {
        domain: "build",
        action: action_for(&subject),
        next_command: next_command_for(&subject),
        source: subject,
        readiness: report.status,
        checks: readiness_checks(&report),
        diagnostics: report.diagnostics,
    };
    output::emit("build check", out, format_check);
    Ok(())
}

/// Resolve the exact controlled task/workspace build command without starting it.
pub fn plan(
    task_ref: Option<String>,
    workspace: Option<String>,
    profile: BuildProfile,
) -> Result<()> {
    let mut request = BuildPolicyRequest {
        build_profile: Some(profile.label().to_string()),
        ..BuildPolicyRequest::default()
    };
    let subject = resolve_subject(task_ref, workspace, &mut request)?;
    let report = build_policy::resolve_build_policy(&request);
    let steps = build_policy::build_execution_step(&report)
        .into_iter()
        .collect::<Vec<_>>();
    let outputs = plan_outputs(&report);
    let normalized = serde_json::to_vec(&(&steps, &outputs, &report.diagnostics))?;
    let out = BuildPlanOutput {
        domain: "build",
        action: action_for(&subject),
        source: subject,
        plan_digest: format!("md5:{:x}", Md5::digest(normalized)),
        steps,
        outputs,
        diagnostics: report.diagnostics,
    };
    output::emit("build plan", out, format_plan);
    Ok(())
}

/// Refuse a proposed command when it bypasses the controlled build path.
///
/// Exits non-zero when the command is not allowed, so a hook or wrapper can
/// stop without parsing the JSON.
pub fn gate(command: String) -> Result<()> {
    let report = build_policy::inspect_provider_command(Some(&command));
    let allowed = report.allowed;
    let reason = report.reason.clone();
    output::emit("build gate", report, format_gate);
    if allowed {
        Ok(())
    } else {
        Err(UdfError::Other(format!(
            "命令未通过受控构建门禁：{}",
            reason
        )))
    }
}

/// Build the workspace's main UE project through the controlled path.
pub fn project(
    workspace: Option<String>,
    profile: BuildProfile,
    target: Option<String>,
) -> Result<()> {
    let mut request = BuildPolicyRequest {
        build_profile: Some(profile.label().to_string()),
        build_target: target,
        ..BuildPolicyRequest::default()
    };
    let subject = resolve_subject(None, workspace, &mut request)?;
    let execution = build_policy::execute_project_build(&request).map_err(UdfError::Other)?;
    let succeeded = execution.succeeded();
    let exit_code = execution.exit_code;
    let out = BuildProjectOutput { subject, execution };
    output::emit("build project", out, format_project);
    if succeeded {
        Ok(())
    } else {
        Err(UdfError::Other(format!(
            "受控构建失败，退出码 {}",
            exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )))
    }
}

/// Point the request at a task Host when a task ref is given, otherwise at the
/// workspace's main project.
fn resolve_subject(
    task_ref: Option<String>,
    workspace: Option<String>,
    request: &mut BuildPolicyRequest,
) -> Result<BuildSubject> {
    let config = Config::load()?;
    match task_ref {
        Some(task_ref) => {
            let (host_dir, meta, context) = host::resolve_task(&config, &task_ref)?;
            let uproject = host::resolve_host_uproject(&host_dir, &meta.id)?;
            request.main_project = Some(uproject.to_string_lossy().to_string());
            request.engine_root = Some(context.engine_path.to_string_lossy().to_string());
            Ok(BuildSubject {
                kind: "task",
                name: task_ref,
                project_path: uproject,
                engine_root: context.engine_path,
            })
        }
        None => {
            let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
            // Resolve the .uproject here rather than handing over the directory.
            // The policy resolver walks *up* when a directory holds no project,
            // which would silently retarget the build at a neighbouring project.
            let uproject = crate::commands::workspace::find_uproject(
                &workspace_config.default_project,
            )
            .ok_or_else(|| {
                UdfError::Other(format!(
                    "workspace '{}' 的主项目目录里没有 .uproject：{}。请用 `udf workspace doctor {}` 检查配置。",
                    name,
                    workspace_config.default_project.display(),
                    name
                ))
            })?;
            request.main_project = Some(uproject.to_string_lossy().to_string());
            request.engine_root = Some(workspace_config.engine_path.to_string_lossy().to_string());
            Ok(BuildSubject {
                kind: "workspace",
                name,
                project_path: uproject,
                engine_root: workspace_config.engine_path,
            })
        }
    }
}

fn format_check(out: &BuildCheckOutput) -> String {
    let mut lines = vec![
        format!(
            "Build readiness: {} ({})",
            out.readiness.as_str(),
            verdict_hint(out.readiness)
        ),
        format!("Subject: {} {}", out.source.kind, out.source.name),
        format!("Project: {}", out.source.project_path.display()),
        format!("Engine: {}", out.source.engine_root.display()),
    ];
    for check in &out.checks {
        lines.push(format!("Check: {check}"));
    }
    for diagnostic in &out.diagnostics {
        lines.push(format!("Diagnostic: {diagnostic}"));
    }
    lines.push(format!("Next: {}", out.next_command));
    lines.join("\n")
}

fn format_plan(out: &BuildPlanOutput) -> String {
    let mut lines = vec![
        format!("Build plan: {}", out.plan_digest),
        format!("Subject: {} {}", out.source.kind, out.source.name),
    ];
    for step in &out.steps {
        lines.push(format!("Step: {} {}", step.id, step.executable));
    }
    for output in &out.outputs {
        lines.push(format!("Output: {}", output.display()));
    }
    lines.join("\n")
}

fn action_for(subject: &BuildSubject) -> &'static str {
    if subject.kind == "task" {
        "task"
    } else {
        "project"
    }
}

fn next_command_for(subject: &BuildSubject) -> String {
    if subject.kind == "task" {
        format!("udf build task {}", subject.name)
    } else {
        format!("udf build project --workspace {}", subject.name)
    }
}

fn readiness_checks(report: &BuildPolicyReport) -> Vec<String> {
    vec![
        format!(
            "project={}",
            report
                .uproject_path
                .as_ref()
                .map_or_else(|| "missing".into(), |path| path.display().to_string())
        ),
        format!(
            "engine={}",
            report
                .engine_root
                .as_ref()
                .map_or_else(|| "missing".into(), |path| path.display().to_string())
        ),
        format!(
            "buildBat={}",
            report
                .build_bat
                .as_ref()
                .map_or_else(|| "missing".into(), |path| path.display().to_string())
        ),
        format!(
            "ubt={}",
            report
                .ubt_dll
                .as_ref()
                .map_or_else(|| "missing".into(), |path| path.display().to_string())
        ),
        format!("mutex={}", report.mutex_status),
    ]
}

fn plan_outputs(report: &BuildPolicyReport) -> Vec<PathBuf> {
    report
        .uproject_path
        .as_ref()
        .and_then(|project| project.parent())
        .map(|project_dir| {
            vec![
                project_dir.join("Binaries"),
                project_dir.join("Intermediate"),
            ]
        })
        .unwrap_or_default()
}

fn format_gate(report: &BuildGateReport) -> String {
    let mut lines = vec![
        format!("门禁：{}", if report.allowed { "放行" } else { "拦截" }),
        format!("原因：{}", report.reason),
    ];
    if let Some(trigger) = &report.matched_trigger {
        lines.push(format!("命中：{}", trigger));
    }
    lines.push(format!("建议：{}", report.recommendation));
    lines.join("\n")
}

fn format_project(out: &BuildProjectOutput) -> String {
    let execution = &out.execution;
    let mut lines = vec![
        format!("主项目：{}", out.subject.project_path.display()),
        format!(
            "退出码：{}",
            execution
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ),
    ];
    if let Some(path) = &execution.log_path {
        lines.push(format!("UBT 日志：{}", path.display()));
    }
    lines.join("\n")
}

fn verdict_hint(status: BuildPolicyStatus) -> &'static str {
    match status {
        BuildPolicyStatus::Ready => "可以开始编译",
        BuildPolicyStatus::NeedsUserInput => "缺少信息，需要用户补充",
        BuildPolicyStatus::Blocked => "被策略拒绝",
        BuildPolicyStatus::Deferred => "另一个 UBT 正在跑，稍后再来",
    }
}
