//! Status command implementation

use crate::config::Config;
use crate::error::Result;
use crate::junction;
use crate::state::GlobalState;
use serde::Serialize;

#[derive(Serialize)]
struct StatusOutput {
    configured: bool,
    active_tasks: Vec<ProjectStatus>,
}

#[derive(Serialize)]
struct ProjectStatus {
    project: String,
    active_task: Option<String>,
    junction_valid: bool,
    junction_target: Option<String>,
}

pub fn run(format: &crate::cli::OutputFormat) -> Result<()> {
    let config = Config::load()?;
    let state = GlobalState::load()?;

    let mut project_statuses = Vec::new();

    for (project_name, project_state) in &state.projects {
        let junction_valid = junction::exists(&project_state.junction_path)?;
        let junction_target = if junction_valid {
            junction::get_target(&project_state.junction_path)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            None
        };

        project_statuses.push(ProjectStatus {
            project: project_name.clone(),
            active_task: project_state.active_task.clone(),
            junction_valid,
            junction_target,
        });
    }

    match format {
        crate::cli::OutputFormat::Json => {
            let output_data = StatusOutput {
                configured: true,
                active_tasks: project_statuses,
            };
            println!("{}", serde_json::to_string_pretty(&output_data)?);
        }
        crate::cli::OutputFormat::Human => {
            println!("UnrealDevFlow Status:");
            println!("  Configured: ✓");
            println!("  Hosts root: {:?}", config.hosts_root);
            println!("  Plugin path: {:?}", config.plugin_path);
            println!();

            if project_statuses.is_empty() {
                println!("No active projects.");
            } else {
                println!("Active Projects:");
                for status in &project_statuses {
                    println!("  Project: {}", status.project);
                    println!(
                        "    Active task: {}",
                        status.active_task.as_deref().unwrap_or("none")
                    );
                    println!(
                        "    Junction valid: {}",
                        if status.junction_valid { "✓" } else { "✗" }
                    );
                    if let Some(target) = &status.junction_target {
                        println!("    Junction target: {}", target);
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}
