//! Switch command implementation

use crate::config::Config;
use crate::editor;
use crate::error::{Result, UdfError};
use crate::host;
use crate::junction;
use crate::output;
use crate::state::{GlobalState, ProjectState};
use std::fs;
use std::path::PathBuf;

pub fn run(task_id: &str, projects: Option<Vec<PathBuf>>, force: bool) -> Result<()> {
    let config = Config::load()?;

    // Determine target project(s)
    let target_projects = match projects {
        Some(paths) => paths,
        None => {
            if let Some(default) = &config.default_project {
                vec![default.clone()]
            } else {
                return Err(UdfError::Other("No project specified and no default project configured".to_string()));
            }
        }
    };

    // Check if Editor is running
    if !force && editor::is_editor_running() {
        output::print_warning("UnrealEditor is currently running.");
        output::print_warning("Junction switch will only take effect on next Editor launch.");

        let should_continue = dialoguer::Confirm::new()
            .with_prompt("Continue with switch?")
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;

        if !should_continue {
            output::print_info("Switch cancelled.");
            return Ok(());
        }
    }

    // Get task host path
    let task_host = if task_id == "main" {
        config.plugin_path.clone()
    } else {
        let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
        host_dir.join("Plugins").join("AesWorld")
    };

    if !task_host.exists() {
        return Err(UdfError::TaskNotFound(task_id.to_string()));
    }

    // Switch Junction for each project
    for project_path in &target_projects {
        let junction_path = project_path.join("Plugins").join("AesWorld");
        let project_name = project_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        output::print_info(&format!("Switching project '{}' to task '{}'...", project_name, task_id));

        // Clear UBT intermediate cache
        let ubt_cache = project_path
            .join("Intermediate")
            .join("Build")
            .join("Win64")
            .join("UnrealEditor")
            .join("Development")
            .join("AesWorld");

        if ubt_cache.exists() {
            output::print_info("Clearing UBT intermediate cache...");
            fs::remove_dir_all(&ubt_cache)?;
        }

        // Switch Junction
        junction::switch(&task_host, &junction_path)?;

        // Update state
        let mut state = GlobalState::load()?;
        let project_state = ProjectState {
            path: project_path.clone(),
            active_task: Some(task_id.to_string()),
            junction_path: junction_path.clone(),
            junction_target: Some(task_host.clone()),
            last_switch: Some(chrono::Utc::now()),
            previous_task: state
                .get_project(&project_name)
                .and_then(|p| p.active_task.clone()),
        };
        state.set_project(project_name.clone(), project_state);
        state.save()?;

        output::print_success(&format!("Project '{}' switched to task '{}'", project_name, task_id));
    }

    output::print_info("Restart UnrealEditor to load the new task DLL.");

    Ok(())
}
