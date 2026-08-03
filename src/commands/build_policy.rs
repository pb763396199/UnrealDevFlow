//! Controlled build commands: `build-check`, `build-gate`, `build-project`.
//!
//! These sit in front of the Unreal build so that nobody hand-assembles a
//! `Build.bat` line. `build-check` reports whether a build may start now,
//! `build-gate` refuses commands that bypass this path, and `build-project`
//! builds the workspace's main project rather than a task Host.

use std::path::PathBuf;

use serde::Serialize;

use crate::build_policy::{
    self, BuildExecution, BuildGateReport, BuildPolicyReport, BuildPolicyRequest, BuildPolicyStatus,
};
use crate::build_profile::BuildProfile;
use crate::cli::OutputFormat;
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
    subject: BuildSubject,
    build_policy: BuildPolicyReport,
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
    format: &OutputFormat,
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
        subject,
        build_policy: report,
    };
    output::print_output(format, &out, format_check);
    Ok(())
}

/// Refuse a proposed command when it bypasses the controlled build path.
///
/// Exits non-zero when the command is not allowed, so a hook or wrapper can
/// stop without parsing the JSON.
pub fn gate(format: &OutputFormat, command: String) -> Result<()> {
    let report = build_policy::inspect_provider_command(Some(&command));
    output::print_output(format, &report, format_gate);
    if report.allowed {
        Ok(())
    } else {
        Err(UdfError::Other(format!(
            "命令未通过受控构建门禁：{}",
            report.reason
        )))
    }
}

/// Build the workspace's main UE project through the controlled path.
pub fn project(
    format: &OutputFormat,
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
    let execution =
        build_policy::execute_project_build(&request).map_err(|error| UdfError::Other(error))?;
    let succeeded = execution.succeeded();
    let exit_code = execution.exit_code;
    let out = BuildProjectOutput { subject, execution };
    output::print_output(format, &out, format_project);
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
            request.main_project = Some(
                workspace_config
                    .default_project
                    .to_string_lossy()
                    .to_string(),
            );
            request.engine_root = Some(workspace_config.engine_path.to_string_lossy().to_string());
            Ok(BuildSubject {
                kind: "workspace",
                name,
                project_path: workspace_config.default_project,
                engine_root: workspace_config.engine_path,
            })
        }
    }
}

fn format_check(out: &BuildCheckOutput) -> String {
    let report = &out.build_policy;
    let mut lines = vec![
        format!(
            "构建策略：{}（{}）",
            report.status.as_str(),
            verdict_hint(report.status)
        ),
        format!("原因：{}", report.reason),
        format!("对象：{} {}", out.subject.kind, out.subject.name),
        format!("项目：{}", out.subject.project_path.display()),
        format!("引擎：{}", out.subject.engine_root.display()),
    ];
    if let Some(target) = &report.target {
        lines.push(format!("目标：{}", target));
    }
    lines.push(format!(
        "UBT 互斥锁：{}（{}）",
        report.mutex_name.as_deref().unwrap_or("未解析"),
        report.mutex_status
    ));
    if let Some(command) = &report.validation_command {
        lines.push(format!("受控命令：{}", command));
    }
    for diagnostic in &report.diagnostics {
        lines.push(format!("提示：{}", diagnostic));
    }
    lines.join("\n")
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
    let tail: Vec<&str> = execution.stdout.lines().rev().take(10).collect();
    if !tail.is_empty() {
        lines.push("构建输出末尾 10 行：".to_string());
        for line in tail.into_iter().rev() {
            lines.push(format!("  {}", line));
        }
    }
    if !execution.stderr.trim().is_empty() {
        lines.push(format!("错误输出：{}", execution.stderr.trim()));
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
