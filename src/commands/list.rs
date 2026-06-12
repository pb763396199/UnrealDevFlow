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

pub fn run(format: &crate::cli::OutputFormat) -> Result<()> {
    let config = Config::load()?;
    let tasks = host::list_tasks(&config.hosts_root)?;

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
                "{:<20} {:<30} {:<10} {:<20}",
                "ID", "Name", "Status", "Created"
            );
            println!("{}", "-".repeat(80));
            for task in &tasks {
                println!(
                    "{:<20} {:<30} {:<10} {:<20}",
                    task.id,
                    task.name.chars().take(28).collect::<String>(),
                    task.status,
                    task.created.chars().take(19).collect::<String>()
                );
            }
        }
    }

    Ok(())
}
