//! List command implementation

use crate::config::Config;
use crate::error::Result;
use crate::host;
use crate::output;
use serde::Serialize;

#[derive(Serialize)]
struct TaskListOutput {
    tasks: Vec<host::TaskMeta>,
}

pub fn run(format: &crate::cli::OutputFormat, workspace: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let tasks = if let Some(workspace_name) = workspace {
        let (resolved_name, workspace_config) = config.resolve_workspace(Some(&workspace_name))?;
        let root = host::workspace_host_root(&workspace_config.hosts_root, &resolved_name);
        host::list_tasks(&root)?
    } else {
        let mut all = host::list_tasks(&config.hosts_root)?;
        for (name, workspace_config) in &config.workspaces {
            if workspace_config.hosts_root == config.hosts_root {
                continue;
            }
            let root = host::workspace_host_root(&workspace_config.hosts_root, name);
            all.extend(host::list_tasks(&root)?);
        }
        all
    };

    if tasks.is_empty() {
        output::print_info("No tasks found.");
        return Ok(());
    }

    match format {
        crate::cli::OutputFormat::Json => {
            let output_data = TaskListOutput { tasks };
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
        }
    }

    Ok(())
}
