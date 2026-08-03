//! Delete command implementation (v2 multi-plugin)

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host::{self, PrimaryPlugin};
use crate::output;
use std::path::{Path, PathBuf};

fn is_branch_merged(repo_path: &Path, branch_name: &str) -> Result<bool> {
    let repo = git::open_repo(repo_path)?;
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .map_err(|e| UdfError::Other(format!("Failed to find branch '{}': {}", branch_name, e)))?;
    let branch_commit = branch.get().peel_to_commit().map_err(|e| {
        UdfError::Other(format!(
            "Failed to get commit for branch '{}': {}",
            branch_name, e
        ))
    })?;

    let head = repo
        .head()
        .map_err(|e| UdfError::Other(format!("Failed to get HEAD: {}", e)))?;
    let head_commit = head
        .peel_to_commit()
        .map_err(|e| UdfError::Other(format!("Failed to get HEAD commit: {}", e)))?;

    match repo.merge_base(head_commit.id(), branch_commit.id()) {
        Ok(merge_base) => Ok(merge_base == branch_commit.id()),
        Err(_) => Ok(false),
    }
}

fn unmerged_commit_count(repo_path: &Path, branch_name: &str) -> Result<usize> {
    let output = std::process::Command::new("git")
        .args(["log", "--oneline", &format!("HEAD..{}", branch_name)])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        return Ok(0);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().count())
}

fn has_uncommitted_changes(worktree_path: &Path) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()?;
    if !output.status.success() {
        return Err(UdfError::Other(format!(
            "Failed to check git status: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}

fn is_git_worktree(path: &Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn delete_with_retry(path: &PathBuf, max_retries: u32) -> Result<()> {
    for attempt in 0..max_retries {
        match std::fs::remove_dir_all(path) {
            Ok(_) => return Ok(()),
            Err(e) if attempt < max_retries - 1 => {
                output::print_warning(&format!(
                    "Delete attempt {} failed (file locked?), retrying in 2s: {}",
                    attempt + 1,
                    e
                ));
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!()
}

#[derive(Debug, Clone)]
struct PrimaryDangerReport {
    is_merged: bool,
    unmerged_count: usize,
    has_uncommitted: bool,
}

fn assess_primary(host_dir: &Path, primary: &PrimaryPlugin) -> Result<PrimaryDangerReport> {
    let worktree_abs = host_dir.join(&primary.worktree);
    let has_uncommitted = if worktree_abs.exists() {
        if is_git_worktree(&worktree_abs) {
            has_uncommitted_changes(&worktree_abs)?
        } else {
            output::print_warning(&format!(
                "Worktree path for '{}' is not a Git worktree, treating it as broken residue: {:?}",
                primary.name, worktree_abs
            ));
            false
        }
    } else {
        false
    };
    let (is_merged, unmerged_count) = if !primary.source_repo.as_os_str().is_empty() {
        let branch_exists =
            git::branch_exists(&primary.source_repo, &primary.branch).unwrap_or(false);
        if !branch_exists {
            (true, 0)
        } else {
            let merged = is_branch_merged(&primary.source_repo, &primary.branch).unwrap_or(false);
            let count = if merged {
                0
            } else {
                unmerged_commit_count(&primary.source_repo, &primary.branch).unwrap_or(0)
            };
            (merged, count)
        }
    } else {
        (false, 0)
    };
    Ok(PrimaryDangerReport {
        is_merged,
        unmerged_count,
        has_uncommitted,
    })
}

pub fn run(task_id: &str, force: bool, skip_confirm: bool, dry_run: bool) -> Result<()> {
    let config = Config::load()?;

    let (host_dir, mut meta, _task_context) = host::resolve_task(&config, task_id)?;
    crate::migration::backfill_source_repo(&mut meta, &config);

    let task_id_only = meta.id.clone();
    let expected_branches = vec![
        meta.branch.clone(),
        format!("task-{}", task_id_only),
        format!(
            "task/{}/{}",
            meta.workspace
                .as_deref()
                .unwrap_or(crate::config::DEFAULT_WORKSPACE),
            task_id_only
        ),
    ];
    for p in &meta.primary_plugins {
        if !expected_branches.contains(&p.branch) {
            return Err(UdfError::Other(format!(
                "Branch name '{}' for plugin '{}' does not match expected {:?}. Aborting delete.",
                p.branch, p.name, expected_branches
            )));
        }
    }

    let mut reports: Vec<(PrimaryPlugin, PrimaryDangerReport)> = Vec::new();
    for primary in &meta.primary_plugins {
        let r = assess_primary(&host_dir, primary)?;
        reports.push((primary.clone(), r));
    }

    if dry_run {
        output::print_info(&format!("Dry run: would delete task '{}'", task_id));
        output::print_info(&format!("  Host directory: {:?}", host_dir));
        for (p, r) in &reports {
            output::print_info(&format!(
                "  Plugin '{}' branch='{}' merged={} unmerged={} dirty={}",
                p.name, p.branch, r.is_merged, r.unmerged_count, r.has_uncommitted
            ));
        }
        return Ok(());
    }

    println!();
    output::print_info(&format!("Delete task '{}'", task_id));
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!("  Host dir: {:?}", host_dir));
    for (p, r) in &reports {
        output::print_info(&format!(
            "  Plugin '{}': branch={} merged={} unmerged={} dirty={}",
            p.name, p.branch, r.is_merged, r.unmerged_count, r.has_uncommitted
        ));
    }
    let any_unmerged = reports.iter().any(|(_, r)| !r.is_merged);
    let any_dirty = reports.iter().any(|(_, r)| r.has_uncommitted);
    if any_unmerged {
        println!();
        output::print_warning("⚠ One or more primary plugins have unmerged commits!");
    }
    if any_dirty {
        println!();
        output::print_warning("⚠ One or more worktrees have uncommitted changes!");
    }
    if any_unmerged || any_dirty {
        println!();
        output::print_warning("This operation CANNOT be undone!");
    }
    println!("─────────────────────────────────────────────────────────────");

    if !(force || skip_confirm) {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to delete task '{}'?",
                task_id
            ))
            .default(false)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
        if !confirmed {
            output::print_info("Delete cancelled.");
            return Ok(());
        }
        if any_unmerged || any_dirty {
            let double_confirmed = dialoguer::Confirm::new()
                .with_prompt("This will permanently delete unmerged work. Continue?")
                .default(false)
                .interact()
                .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
            if !double_confirmed {
                output::print_info("Delete cancelled.");
                return Ok(());
            }
        }
    }

    output::print_info(&format!("Deleting task '{}'...", task_id));

    // Step 0: Clean up junctions in all known projects that point to this task's worktrees.
    // This prevents "broken junctions" that would block future switch operations.
    output::print_info("Checking for junctions pointing to this task...");
    let mut state = crate::state::GlobalState::load()?;
    let mut junctions_cleaned = 0;

    for (project_name, project_state) in &state.projects {
        for junction_state in &project_state.junctions {
            // Check if this junction points to any worktree in the current task
            let points_to_task = meta.primary_plugins.iter().any(|p| {
                let worktree_abs = host_dir.join(&p.worktree);
                junction_state.junction_target == worktree_abs
            }) || meta.dependency_plugins.iter().any(|d| {
                if let Some(rel) = &d.junction {
                    let dep_abs = host_dir.join(rel);
                    junction_state.junction_target == dep_abs
                } else {
                    false
                }
            });

            if points_to_task {
                output::print_info(&format!(
                    "  Found junction in project '{}' for plugin '{}': {:?}",
                    project_name, junction_state.plugin_name, junction_state.junction_path
                ));

                // Try to delete the junction
                if junction_state.junction_path.exists() {
                    match crate::junction::delete(&junction_state.junction_path) {
                        Ok(_) => {
                            output::print_success(&format!(
                                "    ✓ Removed junction: {:?}",
                                junction_state.junction_path
                            ));
                            junctions_cleaned += 1;
                        }
                        Err(e) => {
                            output::print_warning(&format!(
                                "    ⚠ Failed to remove junction: {}",
                                e
                            ));
                        }
                    }
                } else {
                    output::print_info(&format!(
                        "    Junction already gone: {:?}",
                        junction_state.junction_path
                    ));
                }
            }
        }
    }

    if junctions_cleaned > 0 {
        output::print_success(&format!(
            "Cleaned up {} junction(s) from main project(s)",
            junctions_cleaned
        ));
    }

    // Remove stale task-owned Junction records as part of the same lifecycle.
    for project_state in state.projects.values_mut() {
        project_state.junctions.retain(|junction_state| {
            !meta
                .primary_plugins
                .iter()
                .any(|p| junction_state.junction_target == host_dir.join(&p.worktree))
                && !meta.dependency_plugins.iter().any(|d| {
                    d.junction
                        .as_ref()
                        .map(|rel| junction_state.junction_target == host_dir.join(rel))
                        .unwrap_or(false)
                })
        });
        let owned_active_task = project_state.active_task.as_deref() == Some(task_id)
            || project_state.active_task.as_deref() == meta.task_uid.as_deref()
            || project_state.active_task.as_deref() == Some(meta.id.as_str());
        if owned_active_task {
            project_state.active_task = None;
        }
        let first = project_state.junctions.first().cloned();
        project_state.junction_path = first
            .as_ref()
            .map(|entry| entry.junction_path.clone())
            .unwrap_or_default();
        project_state.junction_target = first.map(|entry| entry.junction_target);
    }
    state.save()?;

    // Step 1: Remove dependency junctions explicitly.
    for dep in &meta.dependency_plugins {
        if let Some(rel) = &dep.junction {
            let abs = host_dir.join(rel);
            if abs.exists()
                && let Err(e) = crate::junction::delete(&abs)
            {
                output::print_warning(&format!(
                    "Failed to remove junction for '{}': {}",
                    dep.name, e
                ));
            }
        }
    }

    // Step 2: Remove worktree + branch per primary plugin before deleting
    // Host. Git refuses branch deletion while a worktree still owns it.
    let mut all_worktrees_removed = true;
    let mut all_branches_deleted = true;
    for (primary, _) in &reports {
        let worktree_abs = host_dir.join(&primary.worktree);
        if worktree_abs.exists() {
            if is_git_worktree(&worktree_abs) {
                output::print_info(&format!(
                    "Removing worktree for '{}': {:?}",
                    primary.name, worktree_abs
                ));
                if let Err(e) = git::worktree::remove(&primary.source_repo, &worktree_abs) {
                    if is_git_worktree(&worktree_abs) {
                        output::print_warning(&format!("Failed to remove worktree: {}", e));
                        all_worktrees_removed = false;
                    } else {
                        output::print_warning(&format!(
                            "Worktree for '{}' became non-git residue after remove failure; Host cleanup will delete it: {}",
                            primary.name, e
                        ));
                    }
                }
            } else {
                output::print_warning(&format!(
                    "Skipping git worktree remove for '{}': path is not a Git worktree and will be deleted with Host: {:?}",
                    primary.name, worktree_abs
                ));
            }
        }
        if !primary.source_repo.as_os_str().is_empty() {
            if let Err(e) = git::worktree::prune(&primary.source_repo) {
                output::print_warning(&format!("Failed to prune worktrees: {}", e));
            }
            if git::branch_exists(&primary.source_repo, &primary.branch).unwrap_or(false) {
                output::print_info(&format!(
                    "Deleting branch '{}' in {:?}",
                    primary.branch, primary.source_repo
                ));
                if let Err(e) = git::delete_branch_safe(&primary.source_repo, &primary.branch) {
                    output::print_warning(&format!("Failed to delete branch: {}", e));
                    if let Err(prune_error) = git::worktree::prune(&primary.source_repo) {
                        output::print_warning(&format!(
                            "Failed to prune worktrees: {}",
                            prune_error
                        ));
                    }
                    if let Err(retry_error) =
                        git::delete_branch_safe(&primary.source_repo, &primary.branch)
                    {
                        output::print_warning(&format!(
                            "Failed to delete branch after prune: {}",
                            retry_error
                        ));
                        all_branches_deleted = false;
                    }
                }
            } else {
                output::print_info(&format!(
                    "Branch '{}' already gone in {:?}",
                    primary.branch, primary.source_repo
                ));
            }
        }
    }

    // Step 3: Delete Host directory.
    output::print_info("Deleting Host directory...");
    let host_deleted = if host_dir.exists() {
        match delete_with_retry(&host_dir, 3) {
            Ok(_) => true,
            Err(e) => {
                output::print_warning(&format!("Failed to delete Host directory: {}", e));
                false
            }
        }
    } else {
        true
    };

    println!();
    if host_deleted && all_worktrees_removed && all_branches_deleted {
        output::print_success(&format!("Task '{}' deleted successfully!", task_id));
    } else {
        output::print_warning(&format!(
            "Task '{}' partially deleted. Some resources may remain.",
            task_id
        ));
    }

    Ok(())
}
