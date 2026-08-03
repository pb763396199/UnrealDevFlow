//! List command implementation

use crate::config::Config;
use crate::error::Result;
use crate::host;
use crate::output;
use serde::Serialize;

#[derive(Serialize)]
struct TaskListOutput {
    tasks: Vec<host::TaskMeta>,
    #[serde(rename = "damagedTasks")]
    damaged_tasks: Vec<host::DamagedTask>,
}

pub fn run(format: &crate::cli::OutputFormat, workspace: Option<String>) -> Result<()> {
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
    let tasks = inventory.tasks;
    let damaged_tasks = inventory.damaged_tasks;

    if tasks.is_empty() && damaged_tasks.is_empty() {
        output::print_info("No tasks found.");
        return Ok(());
    }

    match format {
        crate::cli::OutputFormat::Json => {
            let output_data = TaskListOutput {
                tasks,
                damaged_tasks,
            };
            println!("{}", serde_json::to_string_pretty(&output_data)?);
        }
        crate::cli::OutputFormat::Human => {
            println!("Tasks:");
            println!(
                "{:<28} {:<30} {:<10} {:<20}",
                "Task", "Name", "Status", "Created"
            );
            println!("{}", "-".repeat(80));
            for task in &tasks {
                let task_ref = task.task_uid.clone().unwrap_or_else(|| task.id.clone());
                println!(
                    "{:<28} {:<30} {:<10} {:<20}",
                    task_ref,
                    task.name.chars().take(28).collect::<String>(),
                    task.status,
                    task.created.chars().take(19).collect::<String>()
                );
            }
            if !damaged_tasks.is_empty() {
                output::print_warning("以下任务元数据已损坏，需要恢复；它们没有被静默隐藏：");
                for damaged in &damaged_tasks {
                    output::print_warning(&format!(
                        "  {}: {}",
                        damaged.host_dir.display(),
                        damaged.error
                    ));
                }
            }
        }
    }

    Ok(())
}
