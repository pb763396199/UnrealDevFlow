//! Friendly workflow wrappers: start and next.

use crate::cli::DepOverride;
use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::host;
use crate::output;

#[allow(clippy::too_many_arguments)]
pub fn start(
    description: &str,
    workspace: Option<String>,
    id: Option<String>,
    branch: Option<String>,
    base_ref: Option<String>,
    primary: Option<Vec<String>>,
    overrides: Vec<DepOverride>,
    skip_confirm: bool,
) -> Result<()> {
    crate::commands::create::run(
        description,
        id,
        branch,
        base_ref,
        Some(description.to_string()),
        workspace,
        primary,
        overrides,
        skip_confirm,
    )
}

pub fn next(task_ref: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let task_ref = match task_ref {
        Some(value) => value,
        None => latest_task_ref(&config)?,
    };
    let (_host_dir, meta, _context) = host::resolve_task(&config, &task_ref)?;
    let display_ref = meta.task_uid.clone().unwrap_or_else(|| meta.id.clone());

    output::print_info(&format!("当前任务：{}", display_ref));
    match meta.build_status.as_ref().map(|s| s.state.as_str()) {
        None => {
            output::print_info("状态：已创建，还没有编译");
            output::print_info("下一步：");
            output::print_info(&format!("  unrealdevflow build {}", display_ref));
        }
        Some("building") => {
            output::print_info("状态：正在编译");
            output::print_info("下一步：");
            output::print_info(&format!("  unrealdevflow build-status {}", display_ref));
        }
        Some("failed") => {
            output::print_warning("状态：上次编译失败");
            output::print_info("下一步：查看日志并修复后重新编译");
            output::print_info(&format!("  unrealdevflow build-status {}", display_ref));
        }
        Some("success") => {
            output::print_success("状态：编译通过，等待 UE 验收");
            output::print_info("下一步：关闭 UE Editor 后切换项目并验收");
            output::print_info(&format!("  unrealdevflow switch {}", display_ref));
            output::print_info("验收通过后：");
            output::print_info(&format!("  unrealdevflow finish {}", display_ref));
        }
        Some(other) => {
            output::print_info(&format!("状态：{}", other));
            output::print_info("下一步：");
            output::print_info(&format!("  unrealdevflow build {}", display_ref));
        }
    }
    Ok(())
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
