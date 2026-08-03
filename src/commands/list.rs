//! List command implementation

use crate::config::Config;
use crate::error::Result;
use crate::host;
use crate::output;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TaskListOutput {
    tasks: Vec<host::TaskMeta>,
    damaged_tasks: Vec<host::DamagedTask>,
}

pub fn run(workspace: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let inventory = if let Some(workspace_name) = workspace {
        let (resolved_name, workspace_config) = config.resolve_workspace(Some(&workspace_name))?;
        let root = host::workspace_host_root(&workspace_config.hosts_root, &resolved_name);
        host::list_task_inventory(&root)?
    } else {
        let mut all = host::list_task_inventory(&config.hosts_root)?;
        for (name, workspace_config) in &config.workspaces {
            if workspace_config.hosts_root == config.hosts_root {
                continue;
            }
            let root = host::workspace_host_root(&workspace_config.hosts_root, name);
            let inventory = host::list_task_inventory(&root)?;
            all.tasks.extend(inventory.tasks);
            all.damaged_tasks.extend(inventory.damaged_tasks);
        }
        all
    };

    let data = TaskListOutput {
        tasks: inventory.tasks,
        damaged_tasks: inventory.damaged_tasks,
    };
    output::emit("task list", data, render);
    Ok(())
}

fn render(data: &TaskListOutput) -> String {
    if data.tasks.is_empty() && data.damaged_tasks.is_empty() {
        return "No tasks found.".to_string();
    }

    let mut lines = vec![
        "Tasks:".to_string(),
        format!(
            "{:<28} {:<30} {:<10} {:<20}",
            "Task", "Name", "Status", "Created"
        ),
        "-".repeat(80),
    ];
    for task in &data.tasks {
        let task_ref = task.task_uid.clone().unwrap_or_else(|| task.id.clone());
        lines.push(format!(
            "{:<28} {:<30} {:<10} {:<20}",
            task_ref,
            task.name.chars().take(28).collect::<String>(),
            task.status,
            task.created.chars().take(19).collect::<String>()
        ));
    }
    if !data.damaged_tasks.is_empty() {
        lines.push("⚠ 以下任务元数据已损坏，需要恢复；它们没有被静默隐藏：".to_string());
        for damaged in &data.damaged_tasks {
            lines.push(format!(
                "⚠   {}: {}",
                damaged.host_dir.display(),
                damaged.error
            ));
        }
    }
    lines.join("\n")
}
