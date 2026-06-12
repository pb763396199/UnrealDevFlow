//! Switch command implementation (v2 multi-junction)

use crate::config::Config;
use crate::editor;
use crate::error::{Result, UdfError};
use crate::host;
use crate::junction;
use crate::output;
use crate::state::{GlobalState, JunctionState, ProjectState, junction_path_for};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(
    task_id: &str,
    projects: Option<Vec<PathBuf>>,
    force: bool,
    skip_regen_project_files: bool,
) -> Result<()> {
    let config = Config::load()?;

    let target_projects = match projects {
        Some(paths) => paths,
        None => vec![config.default_project.clone()],
    };

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

    // === Resolve switch targets ===
    let switch_plan = if task_id == "main" {
        // For "main", we revert every known junction target back to its main repo source.
        // We need to know which plugins were active; we derive them from GlobalState.
        let state = GlobalState::load()?;
        let mut plan: Vec<(String, PathBuf)> = Vec::new();
        for (_proj, ps) in state.projects.iter() {
            for j in &ps.junctions {
                if !plan.iter().any(|(n, _)| n == &j.plugin_name) {
                    let main_path = config
                        .effective_plugins_root()
                        .map(|r| r.join(&j.plugin_name))
                        .unwrap_or_else(|| j.junction_target.clone());
                    plan.push((j.plugin_name.clone(), main_path));
                }
            }
        }
        if plan.is_empty() {
            // Nothing in state; fall back to v1 single plugin if configured.
            if let Some(legacy) = &config.plugin_path {
                let name = legacy
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("AesWorld")
                    .to_string();
                plan.push((name, legacy.clone()));
            }
        }
        plan
    } else {
        let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
        let mut meta = host::read_meta(&host_dir)?;
        crate::migration::backfill_source_repo(&mut meta, &config);

        let mut plan: Vec<(String, PathBuf)> = Vec::new();
        for primary in &meta.primary_plugins {
            plan.push((primary.name.clone(), host_dir.join(&primary.worktree)));
        }
        for dep in &meta.dependency_plugins {
            if let Some(rel) = &dep.junction {
                plan.push((dep.name.clone(), host_dir.join(rel)));
            }
        }
        plan
    };

    if switch_plan.is_empty() {
        return Err(UdfError::Other(format!(
            "No junctions to switch for task '{}'",
            task_id
        )));
    }

    // === Switch every junction for every project ===
    for project_path in &target_projects {
        let project_name = project_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        output::print_info(&format!(
            "Switching project '{}' to task '{}' ({} junction(s))...",
            project_name,
            task_id,
            switch_plan.len()
        ));

        clear_ubt_cache(project_path);

        let mut junctions_state: Vec<JunctionState> = Vec::new();
        for (plugin_name, target_path) in &switch_plan {
            if !target_path.exists() {
                return Err(UdfError::Other(format!(
                    "Junction target does not exist for plugin '{}': {:?}",
                    plugin_name, target_path
                )));
            }
            let junction_path = junction_path_for(project_path, plugin_name);
            if junction_path.exists() {
                handle_existing_path(&junction_path, &project_name)?;
            }
            junction::create(target_path, &junction_path)?;
            output::print_success(&format!("  {} -> {:?}", plugin_name, target_path));
            junctions_state.push(JunctionState {
                plugin_name: plugin_name.clone(),
                junction_path,
                junction_target: target_path.clone(),
            });
        }

        let mut state = GlobalState::load()?;
        let previous_task = state
            .get_project(&project_name)
            .and_then(|p| p.active_task.clone());
        let first = junctions_state.first().cloned();
        let project_state = ProjectState {
            path: project_path.clone(),
            active_task: Some(task_id.to_string()),
            junction_path: first
                .as_ref()
                .map(|j| j.junction_path.clone())
                .unwrap_or_default(),
            junction_target: first.as_ref().map(|j| j.junction_target.clone()),
            last_switch: Some(chrono::Utc::now()),
            previous_task,
            junctions: junctions_state,
        };
        state.set_project(project_name.clone(), project_state);
        state.save()?;

        output::print_success(&format!(
            "Project '{}' switched to task '{}'",
            project_name, task_id
        ));
    }

    // === Regenerate IDE project files (last step) ===
    // After Junctions are re-pointed, VS/Rider/VSCode need to re-scan the
    // `<project>/Plugins/` directory so their IntelliSense / file view
    // reflects the new worktrees. We invoke UBT's GenerateProjectFiles mode
    // (one per target project). See UE 5.5 UnrealBuildTool.cs L252.
    if !skip_regen_project_files {
        let _ = regenerate_project_files(&config, &target_projects);
    } else {
        output::print_info("Skipped: --skip-regen-project-files (run UBT manually to refresh IDE)");
    }

    output::print_info("Restart UnrealEditor to load the new task DLLs.");
    Ok(())
}

fn clear_ubt_cache(project_path: &Path) {
    let ubt_cache = project_path
        .join("Intermediate")
        .join("Build")
        .join("Win64")
        .join("UnrealEditor")
        .join("Development");

    if ubt_cache.exists() {
        output::print_info("Clearing UBT intermediate cache (Development/*)...");
        if let Ok(entries) = fs::read_dir(&ubt_cache) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                }
            }
        }
    }
}

fn handle_existing_path(junction_path: &Path, project_name: &str) -> Result<()> {
    // Check if it's a broken junction (target no longer exists)
    if junction::is_broken(junction_path) {
        output::print_warning(&format!(
            "Found broken junction at {:?} (target no longer exists)",
            junction_path
        ));
        output::print_info("Removing broken junction...");
        return junction::delete(junction_path);
    }

    if junction::exists(junction_path).unwrap_or(false) {
        output::print_info(&format!(
            "Removing existing junction at {:?}",
            junction_path
        ));
        return junction::delete(junction_path);
    }

    let is_empty = fs::read_dir(junction_path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false);

    if is_empty {
        output::print_info(&format!("Removing empty directory at {:?}", junction_path));
        return fs::remove_dir(junction_path)
            .map_err(|e| UdfError::Other(format!("Failed to remove empty directory: {}", e)));
    }

    output::print_warning(&format!(
        "Conflict: '{:?}' already contains a plugin (not a Junction) for project '{}'.",
        junction_path, project_name
    ));

    let locking_processes = detect_locking_processes(junction_path);
    if !locking_processes.is_empty() {
        output::print_warning("Detected processes that may be using this path:");
        for proc in &locking_processes {
            output::print_warning(&format!("  - {} (PID: {})", proc.name, proc.pid));
        }
        output::print_info("Please close these processes and run switch again.");
    }

    output::print_info("To resolve this conflict:");
    output::print_info("  A) Close IDEs / file explorers holding the path, then re-run switch");
    output::print_info(&format!(
        "  B) Manually move/backup: move {:?} {{backup-location}}",
        junction_path
    ));
    output::print_info(&format!(
        "  C) DESTRUCTIVE delete: Remove-Item -Recurse -Force {:?}",
        junction_path
    ));

    Err(UdfError::Other(format!(
        "Switch aborted. Please resolve the conflict at '{:?}' and try again.",
        junction_path
    )))
}

#[derive(Debug, Clone)]
struct ProcessInfo {
    name: String,
    pid: u32,
}

fn detect_locking_processes(path: &Path) -> Vec<ProcessInfo> {
    let mut result = Vec::new();
    let path_str = path.to_string_lossy().to_string();

    if let Some(parent) = path.parent() {
        let project_name = parent
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        for proc in sysinfo_processes() {
            let proc_name = proc.name.to_lowercase();
            let proc_path = proc.path.as_deref().unwrap_or("").to_lowercase();

            if proc.path.is_none() {
                continue;
            }

            if !project_name.is_empty()
                && (proc_path.contains(&project_name.to_lowercase())
                    || proc_path.contains(&path_str.to_lowercase()))
            {
                result.push(ProcessInfo {
                    name: proc.name,
                    pid: proc.pid,
                });
                continue;
            }

            let known_locks = [
                "rider",
                "rider64",
                "clion",
                "intellij",
                "code",
                "codex",
                "devenv",
                "explorer",
                "smartgit",
                "sourcetree",
            ];
            for lock in &known_locks {
                if proc_name.contains(lock) {
                    result.push(ProcessInfo {
                        name: proc.name,
                        pid: proc.pid,
                    });
                    break;
                }
            }
        }
    }

    result
}

struct SimpleProcess {
    name: String,
    pid: u32,
    path: Option<String>,
}

fn sysinfo_processes() -> Vec<SimpleProcess> {
    use sysinfo::System;
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

/// Find the main `.uproject` file inside a UE project directory.
/// Returns the first `.uproject` found in the top-level project dir.
/// (Engine projects like `F:\ShanghaiP4\neon\UGA\DEV\` contain exactly one.)
fn find_main_uproject(project_dir: &Path) -> Result<PathBuf> {
    for entry in fs::read_dir(project_dir)
        .map_err(|e| UdfError::Other(format!("读取项目目录失败 {:?}: {}", project_dir, e)))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "uproject").unwrap_or(false) {
            return Ok(path);
        }
    }
    Err(UdfError::Other(format!(
        "项目目录里找不到 .uproject: {:?}",
        project_dir
    )))
}

/// Invoke UnrealBuildTool to regenerate IDE project files for each target
/// project. After Junction rewiring, VS/Rider/VSCode need a fresh
/// `*.sln` / `*.vcxproj` / `compile_commands.json` that re-scans
/// `<project>/Plugins/` (which now points at the new worktree).
///
/// We call `UnrealBuildTool.exe -Mode=GenerateProjectFiles` directly
/// (not through `Build.bat`) so we don't have to fake a target name.
/// `UnrealBuildTool.exe` is shipped at
/// `<engine>/Engine/Binaries/DotNET/UnrealBuildTool/UnrealBuildTool.exe`.
///
/// Per UE 5.5 `UnrealBuildTool.cs:252`, `-ProjectFiles` (alias of
/// `-Mode=GenerateProjectFiles`) auto-detects the IDE installed on the
/// current machine (VS / Rider / VSCode / CLion / etc.).
fn regenerate_project_files(config: &Config, target_projects: &[PathBuf]) -> Result<()> {
    let ubt_exe = config
        .engine_path
        .join("Engine")
        .join("Binaries")
        .join("DotNET")
        .join("UnrealBuildTool")
        .join("UnrealBuildTool.exe");

    if !ubt_exe.exists() {
        output::print_warning(&format!(
            "UnrealBuildTool.exe 不存在：{:?}，跳过 IDE 项目文件重新生成。",
            ubt_exe
        ));
        return Ok(());
    }

    let mut any_regen = false;
    for project_path in target_projects {
        let uproject = match find_main_uproject(project_path) {
            Ok(p) => p,
            Err(e) => {
                output::print_warning(&format!("无法为 {:?} 生成项目文件：{}", project_path, e));
                continue;
            }
        };

        output::print_info(&format!(
            "Regenerating IDE project files for {:?} ...",
            uproject
        ));

        let status = Command::new(&ubt_exe)
            .arg("-Mode=GenerateProjectFiles")
            .arg(format!("-Project={}", uproject.to_string_lossy()))
            .arg("-Game")
            // `-NoLog` is critical: UBT's `Log.BackupLogFile()` calls
            // `File.Move()` on the existing log file at startup. If the
            // previous UBT log is held by an IDE (Rider/VS/Explorer) that
            // shares the `Log_<Mode>.txt` path under
            // `%LOCALAPPDATA%\UnrealBuildTool\`, the move throws
            // `IOException: file used by another process` and UBT aborts
            // before generating any project files. `-NoLog` is a real
            // UBT flag (see `Modes/BuildMode.cs:139`,
            // `Modes/UnrealHeaderToolMode.cs:505`) that skips log creation
            // and bypasses the backup entirely.
            //
            // We confirmed this empirically with the manual Epic Launcher
            // entry "Generate Visual Studio project files" succeeding while
            // Rider was open — the right-click path is a wrapper that
            // doesn't trip the log-backup race either. `-NoLog` is the
            // simplest equivalent for our unattended invocation.
            .arg("-NoLog")
            .status();

        match status {
            Ok(s) if s.success() => {
                output::print_success(&format!(
                    "  ✓ {:?} 的项目文件已重新生成（IDE 重新扫描完成）",
                    project_path.file_name().unwrap_or_default()
                ));
                any_regen = true;
            }
            Ok(s) => {
                output::print_warning(&format!(
                    "  ⚠ GenerateProjectFiles 退出码 {}（{:?}）",
                    s.code().unwrap_or(-1),
                    project_path
                ));
            }
            Err(e) => {
                output::print_warning(&format!("  ⚠ 启动 UBT 失败：{}（{:?}）", e, project_path));
            }
        }
    }

    if !any_regen {
        output::print_warning("未重新生成任何 IDE 项目文件——请手动跑 UBT 一次。");
    }

    Ok(())
}
