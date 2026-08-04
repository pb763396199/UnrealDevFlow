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
    let mut regen_engine_path = config.engine_path.clone();
    let mut task_bound_project: Option<PathBuf> = None;

    let mut target_projects = projects;

    if !force && editor::is_editor_running() {
        output::print_warning("UnrealEditor is currently running.");
        output::print_warning("Junction switch will only take effect on next Editor launch.");

        let should_continue = dialoguer::Confirm::new()
            .with_prompt("Continue with switch?")
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;

        if !should_continue {
            output::emit(
                "task switch",
                SwitchOutcome {
                    task_ref: task_id.to_string(),
                    cancelled: true,
                    projects: Vec::new(),
                    junctions: Vec::new(),
                    project_files_regenerated: false,
                },
                render_switch,
            );
            return Ok(());
        }
    }

    // === Resolve switch targets ===
    let switch_plan = if task_id == "main" {
        if target_projects.is_none() {
            target_projects = Some(vec![config.default_project.clone()]);
        }
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
                if let Some(name) = legacy.file_name().and_then(|s| s.to_str()) {
                    plan.push((name.to_string(), legacy.clone()));
                } else {
                    output::print_warning(
                        "legacy plugin_path 没有可证明的插件目录名；不会假设它叫 AesWorld。",
                    );
                }
            }
        }
        plan
    } else {
        let (host_dir, mut meta, task_context) = host::resolve_task(&config, task_id)?;
        crate::migration::backfill_source_repo(&mut meta, &config);
        regen_engine_path = task_context.engine_path.clone();
        let bound_project = task_context.default_project.clone();
        validate_task_project_scope(task_id, &bound_project, target_projects.as_deref())?;
        target_projects = Some(vec![bound_project.clone()]);
        task_bound_project = Some(bound_project);

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
    let target_projects = target_projects.unwrap_or_else(|| vec![config.default_project.clone()]);
    let mut state = GlobalState::load()?;
    let shared_plugins_aliases = task_bound_project
        .as_deref()
        .map(|bound_project| {
            diagnose_shared_plugins_directories(&config, &state, bound_project, task_id)
        })
        .unwrap_or_default();

    // Validate the complete cross-project plan before touching any Junction.
    // A conflict on plugin N must not leave plugins 1..N-1 partially switched.
    if let Some(bound_project) = task_bound_project.as_deref() {
        diagnose_cross_project_task_routes(
            &config,
            &state,
            bound_project,
            &shared_plugins_aliases,
            &switch_plan,
            task_id,
        );
    }
    for project_path in &target_projects {
        let project_name = canonical_project_key(project_path);
        for (plugin_name, target_path) in &switch_plan {
            if !target_path.exists() {
                return Err(UdfError::Other(format!(
                    "Junction target does not exist for plugin '{}': {:?}",
                    plugin_name, target_path
                )));
            }
            let junction_path = junction_path_for(project_path, plugin_name);
            diagnose_existing_junction_ledger(&state, project_path, plugin_name, &junction_path);
            preflight_existing_path(&junction_path, &project_name)?;
        }
        report_if_already_active(&state, project_path, task_id, &switch_plan);
    }

    // === Switch every junction for every project ===
    for project_path in &target_projects {
        let project_name = canonical_project_key(project_path);

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
            // `exists()` follows the reparse point, so a junction whose target
            // was deleted reports false while the directory entry is still
            // there. Ask about the entry itself, or `junction::create` walks
            // into "file already exists" and the broken-junction recovery below
            // never runs.
            if entry_exists(&junction_path) {
                handle_existing_path(&junction_path, &project_name)?;
            }
            junction::create(target_path, &junction_path)?;
            let actual_target = junction::get_target(&junction_path)?;
            if !paths_equal(&actual_target, target_path) {
                let already_switched = junctions_state
                    .iter()
                    .map(|entry| entry.plugin_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(UdfError::Other(format!(
                    "Junction 创建后校验失败：'{}' 实际指向 '{}'，期望 '{}'。\n\
                     项目 '{}' 已经有 {} 个 Junction 被切到本任务（{}），但 state 没有写入，账本仍指向上一个任务。\n\
                     恢复动作：移除该异常 Junction 后重新执行同一条 switch；或者 `udf task switch main` 全部复位。",
                    junction_path.display(),
                    actual_target.display(),
                    target_path.display(),
                    project_name,
                    junctions_state.len(),
                    if already_switched.is_empty() {
                        "无"
                    } else {
                        &already_switched
                    }
                )));
            }
            output::print_success(&format!("  {} -> {:?}", plugin_name, target_path));
            junctions_state.push(JunctionState {
                plugin_name: plugin_name.clone(),
                junction_path,
                junction_target: target_path.clone(),
            });
        }

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

        output::print_success(&format!(
            "Project '{}' switched to task '{}'",
            project_name, task_id
        ));
    }
    clear_alias_project_state(&mut state, &shared_plugins_aliases);
    state.save()?;

    // === Regenerate IDE project files (last step) ===
    // After Junctions are re-pointed, VS/Rider/VSCode need to re-scan the
    // `<project>/Plugins/` directory so their IntelliSense / file view
    // reflects the new worktrees. We invoke UBT's GenerateProjectFiles mode
    // (one per target project). See UE 5.5 UnrealBuildTool.cs L252.
    if !skip_regen_project_files {
        let _ = regenerate_project_files(&regen_engine_path, &target_projects);
    } else {
        output::print_info("Skipped: --skip-regen-project-files (run UBT manually to refresh IDE)");
    }

    output::emit(
        "task switch",
        SwitchOutcome {
            task_ref: task_id.to_string(),
            cancelled: false,
            projects: target_projects
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
            junctions: switch_plan
                .iter()
                .map(|(plugin_name, target)| SwitchedJunction {
                    plugin: plugin_name.clone(),
                    target: target.to_string_lossy().to_string(),
                })
                .collect(),
            project_files_regenerated: !skip_regen_project_files,
        },
        render_switch,
    );
    Ok(())
}

/// Which Junctions now point where, so a caller does not have to inspect the
/// filesystem to find out what changed.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SwitchOutcome {
    task_ref: String,
    cancelled: bool,
    projects: Vec<String>,
    junctions: Vec<SwitchedJunction>,
    project_files_regenerated: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SwitchedJunction {
    plugin: String,
    target: String,
}

fn render_switch(data: &SwitchOutcome) -> String {
    if data.cancelled {
        return format!("Switch to '{}' cancelled. Nothing moved.", data.task_ref);
    }
    let mut lines = vec![format!(
        "✓ Switched to '{}' ({} junction(s))",
        data.task_ref,
        data.junctions.len()
    )];
    for junction in &data.junctions {
        lines.push(format!("  {} -> {}", junction.plugin, junction.target));
    }
    lines.push("Restart UnrealEditor to load the new task DLLs.".to_string());
    lines.join("\n")
}

/// A task belongs to exactly one UE project — the one frozen in its context.
///
/// Pointing that task's plugin worktree into some other project produces two
/// projects sharing one Host, which is the cross-project routing mess the
/// diagnostics below exist to detect. Refuse it up front instead.
fn validate_task_project_scope(
    task_id: &str,
    bound_project: &Path,
    requested_projects: Option<&[PathBuf]>,
) -> Result<()> {
    let Some(requested_projects) = requested_projects else {
        return Ok(());
    };
    if requested_projects.len() == 1 && paths_equal(&requested_projects[0], bound_project) {
        return Ok(());
    }

    let requested = requested_projects
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(UdfError::Other(format!(
        "任务 '{}' 已绑定主项目 '{}'，不能切换或改写其他项目 Junction（请求：{}）。\n\
         恢复动作：去掉本次 --project 参数并重试；如果确实要为其他项目开发，请为该项目创建独立任务。未修改任何 Junction。",
        task_id,
        bound_project.display(),
        requested
    )))
}

/// Say so when this project is already on the task being switched to.
///
/// The work still happens — re-pointing a Junction at where it already points
/// is harmless, and regenerating project files is sometimes exactly why you
/// re-ran the command. But without this line the output looks identical to a
/// real switch, so you cannot tell whether anything moved.
fn report_if_already_active(
    state: &GlobalState,
    project_path: &Path,
    task_id: &str,
    switch_plan: &[(String, PathBuf)],
) {
    let project_key = canonical_project_key(project_path);
    let Some(project_state) = state.get_project(&project_key) else {
        return;
    };
    if project_state.active_task.as_deref() != Some(task_id) {
        return;
    }
    let all_on_target = switch_plan.iter().all(|(plugin_name, target_path)| {
        let junction_path = junction_path_for(project_path, plugin_name);
        junction::get_target(&junction_path)
            .map(|actual| paths_equal(&actual, target_path))
            .unwrap_or(false)
    });
    if !all_on_target {
        return;
    }
    output::print_info(&format!(
        "项目 '{}' 已经在任务 '{}' 上，{} 个 Junction 都指向正确目标。下面的重建是幂等的，若只想确认状态可用 `udf workspace status`。",
        project_path.display(),
        task_id,
        switch_plan.len()
    ));
}

/// Warn when the Junction on disk disagrees with what state.json recorded.
///
/// Read-only: an explicit `switch` is authorization to take the route over, so
/// the caller rebuilds both the Junction and the ledger afterwards.
fn diagnose_existing_junction_ledger(
    state: &GlobalState,
    project_path: &Path,
    plugin_name: &str,
    junction_path: &Path,
) {
    if !junction::exists(junction_path).unwrap_or(false) {
        return;
    }

    let actual_target = match junction::get_target(junction_path) {
        Ok(target) => target,
        Err(error) => {
            output::print_warning(&format!(
                "现有 Junction '{}' 无法读取目标（{}）；本次显式 switch 将在绑定项目内安全接管。",
                junction_path.display(),
                error
            ));
            return;
        }
    };
    let project_key = canonical_project_key(project_path);
    let Some(project_state) = state.get_project(&project_key).or_else(|| {
        state
            .projects
            .values()
            .find(|entry| paths_equal(&entry.path, project_path))
    }) else {
        warn_untracked_junction(project_path, plugin_name, junction_path, &actual_target);
        return;
    };
    let Some(recorded) = project_state
        .junctions
        .iter()
        .find(|entry| entry.plugin_name.eq_ignore_ascii_case(plugin_name))
    else {
        warn_untracked_junction(project_path, plugin_name, junction_path, &actual_target);
        return;
    };

    if project_state.active_task.is_none()
        || !paths_equal(&recorded.junction_path, junction_path)
        || !paths_equal(&recorded.junction_target, &actual_target)
    {
        output::print_warning(&format!(
            "项目 '{}' 的插件 '{}' Junction 与 state 账本不一致。实际：'{}' -> '{}'；账本：'{}' -> '{}'（activeTask={:?}）。本次显式 switch 将在绑定项目内重建 Junction 和 state。",
            project_path.display(),
            plugin_name,
            junction_path.display(),
            actual_target.display(),
            recorded.junction_path.display(),
            recorded.junction_target.display(),
            project_state.active_task
        ));
    }
}

fn warn_untracked_junction(
    project_path: &Path,
    plugin_name: &str,
    junction_path: &Path,
    actual_target: &Path,
) {
    output::print_warning(&format!(
        "发现未记账 Junction：项目 '{}' 的插件 '{}'，'{}' -> '{}'。本次显式 switch 将在绑定项目内安全接管并写入 state。",
        project_path.display(),
        plugin_name,
        junction_path.display(),
        actual_target.display()
    ));
}

/// Warn when another project's Junction already points at this task's Host.
///
/// Never touches those projects: they are outside the task's scope.
fn diagnose_cross_project_task_routes(
    config: &Config,
    state: &GlobalState,
    bound_project: &Path,
    shared_plugins_aliases: &[PathBuf],
    switch_plan: &[(String, PathBuf)],
    task_id: &str,
) {
    let mut projects = Vec::new();
    push_unique_project(&mut projects, config.default_project.clone());
    for workspace in config.workspaces.values() {
        push_unique_project(&mut projects, workspace.default_project.clone());
    }
    for project_state in state.projects.values() {
        push_unique_project(&mut projects, project_state.path.clone());
    }

    for project_path in projects {
        if paths_equal(&project_path, bound_project)
            || shared_plugins_aliases
                .iter()
                .any(|alias| paths_equal(alias, &project_path))
        {
            continue;
        }
        for (plugin_name, target_path) in switch_plan {
            let other_junction = junction_path_for(&project_path, plugin_name);
            if !junction::exists(&other_junction).unwrap_or(false) {
                continue;
            }
            let actual_target = match junction::get_target(&other_junction) {
                Ok(target) => target,
                Err(error) => {
                    output::print_warning(&format!(
                        "无法核对其他项目 Junction '{}'：{}。该项目不属于本任务，本次 switch 不会修改它。",
                        other_junction.display(),
                        error
                    ));
                    continue;
                }
            };
            if paths_equal(&actual_target, target_path) {
                output::print_warning(&format!(
                    "检测到跨项目任务路由：任务 '{}' 绑定项目 '{}'，但其他项目 '{}' 的插件 '{}' 也指向该任务 Host '{}'。本次 switch 只修改绑定项目；建议随后将其他项目切回它自己的任务或主线。",
                    task_id,
                    bound_project.display(),
                    project_path.display(),
                    plugin_name,
                    actual_target.display()
                ));
            }
        }
    }
}

/// Find projects whose `Plugins` directory is physically the same directory as
/// the bound project's, so switching one silently switches the others.
fn diagnose_shared_plugins_directories(
    config: &Config,
    state: &GlobalState,
    bound_project: &Path,
    task_id: &str,
) -> Vec<PathBuf> {
    let bound_plugins = bound_project.join("Plugins");
    let Some(bound_plugins_physical) = canonical_existing_path(&bound_plugins) else {
        return Vec::new();
    };

    let mut projects = Vec::new();
    push_unique_project(&mut projects, config.default_project.clone());
    for workspace in config.workspaces.values() {
        push_unique_project(&mut projects, workspace.default_project.clone());
    }
    for project_state in state.projects.values() {
        push_unique_project(&mut projects, project_state.path.clone());
    }

    let mut aliases = Vec::new();
    for project_path in projects {
        if paths_equal(&project_path, bound_project) {
            continue;
        }
        let Some(other_plugins_physical) = canonical_existing_path(&project_path.join("Plugins"))
        else {
            continue;
        };
        if paths_equal(&other_plugins_physical, &bound_plugins_physical) {
            push_unique_project(&mut aliases, project_path);
        }
    }

    if !aliases.is_empty() {
        let affected = aliases
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        output::print_warning(&format!(
            "shared_plugins_directory: 任务 '{}' 的绑定项目 '{}' 与以下项目共享物理 Plugins 目录 '{}'：{}。本次显式 switch 仍会执行，并会通过该共享目录影响这些项目；成功后将清除别名项目的陈旧 state 路由。",
            task_id,
            bound_project.display(),
            bound_plugins_physical.display(),
            affected
        ));
    }

    aliases
}

fn clear_alias_project_state(state: &mut GlobalState, aliases: &[PathBuf]) {
    if aliases.is_empty() {
        return;
    }
    state.projects.retain(|_, project_state| {
        !aliases
            .iter()
            .any(|alias| paths_equal(&project_state.path, alias))
    });
}

fn canonical_existing_path(path: &Path) -> Option<PathBuf> {
    dunce::canonicalize(path).ok()
}

fn push_unique_project(projects: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !projects
        .iter()
        .any(|existing| paths_equal(existing, &candidate))
    {
        projects.push(candidate);
    }
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    canonical_project_key(left) == canonical_project_key(right)
}

/// Is there a directory entry at this path, whatever it points at?
///
/// `Path::exists` resolves the reparse point, so a junction left dangling by a
/// previous `task cleanup` answers "no" while still occupying the name.
fn entry_exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
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

fn canonical_project_key(project_path: &Path) -> String {
    dunce::canonicalize(project_path)
        .unwrap_or_else(|_| project_path.to_path_buf())
        .to_string_lossy()
        .to_string()
        .to_lowercase()
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

fn preflight_existing_path(junction_path: &Path, project_name: &str) -> Result<()> {
    if !entry_exists(junction_path)
        || junction::is_broken(junction_path)
        || junction::exists(junction_path).unwrap_or(false)
    {
        return Ok(());
    }

    let is_empty = fs::read_dir(junction_path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false);
    if is_empty {
        return Ok(());
    }

    output::print_warning(&format!(
        "Conflict: '{:?}' already contains a plugin (not a Junction) for project '{}'.",
        junction_path, project_name
    ));
    output::print_info("Switch preflight aborted before changing any plugin Junction.");
    output::print_info(
        "To resolve this conflict, move or back up the existing plugin directory, then retry.",
    );
    Err(UdfError::Other(format!(
        "Switch aborted without changes. Resolve the conflict at '{:?}' and try again.",
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
fn regenerate_project_files(engine_path: &Path, target_projects: &[PathBuf]) -> Result<()> {
    let ubt_exe = engine_path
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn test_config(default_project: PathBuf, other_project: PathBuf) -> Config {
        let mut workspaces = HashMap::new();
        workspaces.insert(
            "other".to_string(),
            crate::config::WorkspaceConfig {
                hosts_root: PathBuf::from("hosts-other"),
                plugin_path: None,
                default_project: other_project,
                engine_path: PathBuf::from("engine"),
                plugins_root: None,
                plugin_overrides: HashMap::new(),
            },
        );
        Config {
            hosts_root: PathBuf::from("hosts"),
            plugin_path: None,
            default_project,
            engine_path: PathBuf::from("engine"),
            plugins_root: None,
            plugin_overrides: HashMap::new(),
            workspaces,
            last_used_workspace: None,
        }
    }

    fn empty_state() -> GlobalState {
        GlobalState {
            version: 2,
            projects: HashMap::new(),
        }
    }

    #[test]
    fn task_switch_rejects_second_project_with_same_plugin() {
        let root = tempdir().expect("temp dir");
        let project_a = root.path().join("ProjectA");
        let project_b = root.path().join("ProjectB");
        fs::create_dir_all(&project_a).expect("project A");
        fs::create_dir_all(&project_b).expect("project B");

        let error = validate_task_project_scope(
            "workspace-a/task-a",
            &project_a,
            Some(std::slice::from_ref(&project_b)),
        )
        .expect_err("cross-project switch must be rejected");

        let message = error.to_string();
        assert!(message.contains("已绑定主项目"));
        assert!(message.contains("未修改任何 Junction"));
        assert!(
            validate_task_project_scope(
                "workspace-a/task-a",
                &project_a,
                Some(std::slice::from_ref(&project_a)),
            )
            .is_ok()
        );
    }

    #[test]
    fn task_switch_only_diagnoses_same_host_routed_from_second_project() {
        let root = tempdir().expect("temp dir");
        let project_a = root.path().join("ProjectA");
        let project_b = root.path().join("ProjectB");
        let host_plugin = root.path().join("Host").join("Plugins").join("AesWorld");
        fs::create_dir_all(project_a.join("Plugins")).expect("project A plugins");
        fs::create_dir_all(project_b.join("Plugins")).expect("project B plugins");
        fs::create_dir_all(&host_plugin).expect("host plugin");
        let project_b_junction = junction_path_for(&project_b, "AesWorld");
        junction::create(&host_plugin, &project_b_junction).expect("project B junction");

        let config = test_config(project_a.clone(), project_b.clone());
        diagnose_cross_project_task_routes(
            &config,
            &empty_state(),
            &project_a,
            &[],
            &[("AesWorld".to_string(), host_plugin.clone())],
            "workspace-a/task-a",
        );

        // Diagnostics must never mutate or block the unrelated project.
        assert_eq!(
            junction::get_target(&project_b_junction).expect("junction target"),
            host_plugin
        );
        junction::delete(&project_b_junction).expect("junction cleanup");
    }

    #[test]
    fn existing_junction_is_adopted_when_state_ledger_is_stale() {
        let root = tempdir().expect("temp dir");
        let project = root.path().join("ProjectA");
        let actual_target = root.path().join("HostA").join("Plugins").join("AesWorld");
        let recorded_target = root.path().join("HostB").join("Plugins").join("AesWorld");
        fs::create_dir_all(project.join("Plugins")).expect("project plugins");
        fs::create_dir_all(&actual_target).expect("actual target");
        fs::create_dir_all(&recorded_target).expect("recorded target");
        let junction_path = junction_path_for(&project, "AesWorld");
        junction::create(&actual_target, &junction_path).expect("junction");

        let project_key = canonical_project_key(&project);
        let mut state = empty_state();
        state.set_project(
            project_key,
            ProjectState {
                path: project.clone(),
                active_task: Some("workspace-a/old-task".to_string()),
                junction_path: junction_path.clone(),
                junction_target: Some(recorded_target.clone()),
                last_switch: None,
                previous_task: None,
                junctions: vec![JunctionState {
                    plugin_name: "AesWorld".to_string(),
                    junction_path: junction_path.clone(),
                    junction_target: recorded_target,
                }],
            },
        );

        diagnose_existing_junction_ledger(&state, &project, "AesWorld", &junction_path);
        // Explicit switch is authorization to replace this route; diagnosis is
        // read-only and the caller will rebuild both Junction and state.
        assert_eq!(
            junction::get_target(&junction_path).expect("junction target"),
            actual_target
        );
        junction::delete(&junction_path).expect("junction cleanup");
    }

    #[test]
    fn shared_plugins_parent_alias_is_reported_and_stale_alias_state_is_removed() {
        let root = tempdir().expect("temp dir");
        let project_a = root.path().join("ProjectA");
        let project_b = root.path().join("ProjectB");
        let shared_plugins = project_a.join("Plugins");
        let host_plugin = root.path().join("Host").join("Plugins").join("AesWorld");
        fs::create_dir_all(&shared_plugins).expect("shared plugins");
        fs::create_dir_all(&project_b).expect("project B");
        fs::create_dir_all(&host_plugin).expect("host plugin");
        junction::create(&shared_plugins, &project_b.join("Plugins"))
            .expect("shared Plugins junction");

        let config = test_config(project_a.clone(), project_b.clone());
        let mut state = empty_state();
        state.set_project(
            canonical_project_key(&project_b),
            ProjectState {
                path: project_b.clone(),
                active_task: Some("workspace-b/stale-task".to_string()),
                junction_path: project_b.join("Plugins").join("AesWorld"),
                junction_target: Some(PathBuf::from("stale-target")),
                last_switch: None,
                previous_task: None,
                junctions: Vec::new(),
            },
        );

        let aliases =
            diagnose_shared_plugins_directories(&config, &state, &project_a, "workspace-a/task-a");
        assert_eq!(aliases.len(), 1);
        assert!(paths_equal(&aliases[0], &project_b));

        let bound_junction = junction_path_for(&project_a, "AesWorld");
        junction::create(&host_plugin, &bound_junction).expect("bound project switch");
        assert_eq!(
            junction::get_target(&junction_path_for(&project_b, "AesWorld"))
                .expect("alias observes same switched plugin"),
            host_plugin
        );

        clear_alias_project_state(&mut state, &aliases);
        assert!(state.projects.is_empty());
        assert!(project_b.join("Plugins").exists());
        junction::delete(&bound_junction).expect("plugin junction cleanup");
        junction::delete(&project_b.join("Plugins")).expect("shared Plugins cleanup");
    }

    #[test]
    fn a_dangling_junction_still_counts_as_an_existing_entry() {
        let root = tempdir().expect("temp dir");
        let target = root.path().join("Target");
        let link = root.path().join("Link");
        fs::create_dir_all(&target).expect("target");
        junction::create(&target, &link).expect("junction");
        assert!(entry_exists(&link));

        // Delete what it points at: the entry survives, `exists()` does not.
        fs::remove_dir_all(&target).expect("remove target");
        assert!(!link.exists(), "Path::exists follows the reparse point");
        assert!(
            entry_exists(&link),
            "the directory entry is still there, so switch must clean it up"
        );
        junction::delete(&link).expect("a dangling junction must be removable");
    }
}
