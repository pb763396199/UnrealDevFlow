//! Status command implementation

use crate::config::Config;
use crate::error::Result;
use crate::junction;
use crate::output;
use crate::state::GlobalState;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusOutput {
    configured: bool,
    hosts_root: String,
    plugin_path: Option<String>,
    active_tasks: Vec<ProjectStatus>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectStatus {
    project: String,
    active_task: Option<String>,
    junction_valid: bool,
    junction_target: Option<String>,
}

pub fn run() -> Result<()> {
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

    let data = StatusOutput {
        configured: true,
        hosts_root: config.hosts_root.to_string_lossy().to_string(),
        plugin_path: config
            .plugin_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        active_tasks: project_statuses,
    };
    output::emit("status", data, render);
    Ok(())
}

fn render(data: &StatusOutput) -> String {
    let mut lines = vec![
        "UnrealDevFlow Status:".to_string(),
        "  Configured: ✓".to_string(),
        format!("  Hosts root: {}", data.hosts_root),
        format!(
            "  Plugin path: {}",
            data.plugin_path.as_deref().unwrap_or("none")
        ),
        String::new(),
    ];

    if data.active_tasks.is_empty() {
        lines.push("No active projects.".to_string());
        return lines.join("\n");
    }

    lines.push("Active Projects:".to_string());
    for status in &data.active_tasks {
        lines.push(format!("  Project: {}", status.project));
        lines.push(format!(
            "    Active task: {}",
            status.active_task.as_deref().unwrap_or("none")
        ));
        lines.push(format!(
            "    Junction valid: {}",
            if status.junction_valid { "✓" } else { "✗" }
        ));
        if let Some(target) = &status.junction_target {
            lines.push(format!("    Junction target: {}", target));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}
