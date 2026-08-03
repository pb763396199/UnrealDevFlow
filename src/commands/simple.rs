//! The `task next` advisor: where a task stands and what to run next.

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::host;
use crate::output;
use serde::Serialize;

/// Where the task stands, plus the single next command to run.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NextStep {
    task_ref: String,
    state: String,
    summary: String,
    commands: Vec<String>,
}

pub fn next(task_ref: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let task_ref = match task_ref {
        Some(value) => value,
        None => latest_task_ref(&config)?,
    };
    let (_host_dir, meta, _context) = host::resolve_task(&config, &task_ref)?;
    let display_ref = meta.task_uid.clone().unwrap_or_else(|| meta.id.clone());

    let state = meta
        .build_status
        .as_ref()
        .map(|status| status.state.clone())
        .unwrap_or_else(|| "created".to_string());

    let (summary, commands) = match state.as_str() {
        "created" => (
            "已创建，还没有编译".to_string(),
            vec![format!("udf build task {}", display_ref)],
        ),
        "building" => (
            "正在编译".to_string(),
            vec![format!("udf build status {}", display_ref)],
        ),
        "failed" => (
            "上次编译失败，看日志修复后重新编译".to_string(),
            vec![format!("udf build status {}", display_ref)],
        ),
        "success" => (
            "编译通过，等待 UE 验收".to_string(),
            vec![
                format!("udf task switch {}", display_ref),
                format!("udf task finish {}", display_ref),
            ],
        ),
        other => (
            other.to_string(),
            vec![format!("udf build task {}", display_ref)],
        ),
    };

    output::emit(
        "task next",
        NextStep {
            task_ref: display_ref,
            state,
            summary,
            commands,
        },
        render_next,
    );
    Ok(())
}

fn render_next(data: &NextStep) -> String {
    let mut lines = vec![
        format!("当前任务：{}", data.task_ref),
        format!("状态：{}", data.summary),
        "下一步：".to_string(),
    ];
    for command in &data.commands {
        lines.push(format!("  {}", command));
    }
    lines.join("\n")
}

pub fn latest_task_ref(config: &Config) -> Result<String> {
    let mut tasks = host::list_tasks(&config.hosts_root)?;
    for (name, workspace) in &config.workspaces {
        if workspace.hosts_root == config.hosts_root {
            continue;
        }
        let root = host::workspace_host_root(&workspace.hosts_root, name);
        tasks.extend(host::list_tasks(&root)?);
    }
    tasks.sort_by(|a, b| b.created.cmp(&a.created));
    match tasks.first() {
        Some(task) => Ok(task.task_uid.clone().unwrap_or_else(|| task.id.clone())),
        None => Err(UdfError::Other("没有找到任何任务".to_string())),
    }
}
