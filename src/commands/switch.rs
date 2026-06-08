//! Switch command implementation

use crate::config::Config;
use crate::editor;
use crate::error::{Result, UdfError};
use crate::host;
use crate::junction;
use crate::output;
use crate::state::{GlobalState, ProjectState};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(task_id: &str, projects: Option<Vec<PathBuf>>, force: bool) -> Result<()> {
    let config = Config::load()?;

    // Determine target project(s)
    let target_projects = match projects {
        Some(paths) => paths,
        None => vec![config.default_project.clone()],
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
            if let Err(e) = fs::remove_dir_all(&ubt_cache) {
                output::print_warning(&format!("  Failed to clear UBT cache: {}", e));
            }
        }

        // Handle existing path at junction target
        if junction_path.exists() {
            match handle_existing_path(&junction_path, &project_name) {
                Ok(()) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }

        // Create Junction
        junction::create(&task_host, &junction_path)?;

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

/// Handle existing path at the junction target location.
/// Strategy:
/// 1. If it's a Junction → delete it
/// 2. If it's an empty directory → delete it directly
/// 3. If it's a non-empty directory → prompt user to handle it
fn handle_existing_path(junction_path: &Path, project_name: &str) -> Result<()> {
    // Case 1: It's a Junction
    if junction::exists(junction_path).unwrap_or(false) {
        output::print_info(&format!("Removing existing junction at {:?}", junction_path));
        return junction::delete(junction_path);
    }

    // Case 2: Empty directory → delete directly
    let is_empty = fs::read_dir(junction_path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false);

    if is_empty {
        output::print_info(&format!("Removing empty directory at {:?}", junction_path));
        return fs::remove_dir(junction_path)
            .map_err(|e| UdfError::Other(format!("Failed to remove empty directory: {}", e)));
    }

    // Case 3: Non-empty directory → prompt user to handle it
    output::print_warning(&format!(
        "Conflict: '{:?}' already contains an AesWorld plugin (not a Junction).",
        junction_path
    ));

    // Try to detect which processes are locking the path
    let locking_processes = detect_locking_processes(junction_path);
    if !locking_processes.is_empty() {
        output::print_warning("Detected processes that may be using this path:");
        for proc in &locking_processes {
            output::print_warning(&format!("  - {} (PID: {})", proc.name, proc.pid));
        }
        output::print_info("Please close these processes and run switch again.");
    }

    output::print_info("To resolve this conflict, choose ONE of the following options:");
    output::print_info("");
    output::print_info("  Option A: Close all processes using the path, then re-run switch");
    output::print_info("    Steps:");
    output::print_info("      1. Close IDEs (Rider, VSCode) that may have the path open");
    output::print_info("      2. Close any file explorer windows on the path");
    output::print_info("      3. Close UE Editor if running");
    output::print_info("      4. Run: unrealdevflow switch again");
    output::print_info("");
    output::print_info(&format!(
        "  Option B: Manually move/backup the directory, then re-run switch"
    ));
    output::print_info("    Steps:");
    output::print_info(&format!(
        "      1. Move or rename: move {:?} {{backup-location}}",
        junction_path
    ));
    output::print_info("      2. Run: unrealdevflow switch again");
    output::print_info("");
    output::print_info(&format!(
        "  Option C: Manually delete the directory (DESTRUCTIVE - will lose any uncommitted changes)"
    ));
    output::print_info("    Steps:");
    output::print_info(&format!(
        "      1. Delete: Remove-Item -Recurse -Force {:?}",
        junction_path
    ));
    output::print_info("      2. Run: unrealdevflow switch again");
    output::print_info("");

    Err(UdfError::Other(format!(
        "Switch aborted. Please resolve the conflict at '{:?}' and try again.",
        junction_path
    )))
}

/// Information about a process that may be locking a path
#[derive(Debug, Clone)]
struct ProcessInfo {
    name: String,
    pid: u32,
    path: Option<String>,
}

/// Try to detect which processes are using a given path
fn detect_locking_processes(path: &Path) -> Vec<ProcessInfo> {
    let mut result = Vec::new();
    let path_str = path.to_string_lossy().to_string();

    // Method 1: Check processes whose executable path contains the project name
    // or contains path components of the target
    if let Some(parent) = path.parent() {
        let project_name = parent
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Get all processes and check various heuristics
        for proc in sysinfo_processes() {
            let proc_name = proc.name.to_lowercase();
            let proc_path = proc.path.as_deref().unwrap_or("").to_lowercase();

            // Skip system processes
            if proc.path.is_none() {
                continue;
            }

            // Check if process path is within the project
            if !project_name.is_empty()
                && (proc_path.contains(&project_name.to_lowercase())
                    || proc_path.contains(&path_str.to_lowercase()))
            {
                result.push(ProcessInfo {
                    name: proc.name,
                    pid: proc.pid,
                    path: proc.path,
                });
                continue;
            }

            // Check by process name (common IDEs, editors, etc.)
            let known_locks = [
                "rider", "rider64", "clion", "intellij",
                "code", "codex", "devenv", "explorer",
                "devenv", "smartgit", "sourcetree",
            ];
            for lock in &known_locks {
                if proc_name.contains(lock) {
                    result.push(ProcessInfo {
                        name: proc.name,
                        pid: proc.pid,
                        path: proc.path.clone(),
                    });
                    break;
                }
            }
        }
    }

    result
}

/// Simple process info
struct SimpleProcess {
    name: String,
    pid: u32,
    path: Option<String>,
}

/// Get all running processes (cross-platform best-effort)
fn sysinfo_processes() -> Vec<SimpleProcess> {
    use sysinfo::{Pid, System};
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    sys.processes()
        .iter()
        .map(|(pid, process)| SimpleProcess {
            name: process.name().to_string_lossy().to_string(),
            pid: pid.as_u32(),
            path: process.exe().map(|p| p.to_string_lossy().to_string()),
        })
        .collect()
}
