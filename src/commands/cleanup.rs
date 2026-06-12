//! Cleanup command implementation (v2 multi-plugin)
//!
//! Manually cleanup worktrees and branches after merge verification.

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host;
use crate::output;
use std::path::PathBuf;

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

pub fn run(task_id: &str, force: bool, skip_confirm: bool) -> Result<()> {
    let config = Config::load()?;

    let (host_dir, mut meta, _task_context) = host::resolve_task(&config, task_id)?;
    crate::migration::backfill_source_repo(&mut meta, &config);

    let task_id_only = meta.id.clone();
    let expected_branches = vec![
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
                "Branch name '{}' for plugin '{}' does not match expected {:?}. Aborting cleanup.",
                p.branch, p.name, expected_branches
            )));
        }
    }

    println!();
    output::print_info(&format!("Cleanup task '{}'", task_id));
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!(
        "  Primary plugins ({}): {}",
        meta.primary_plugins.len(),
        meta.primary_plugins
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if !meta.dependency_plugins.is_empty() {
        output::print_info(&format!(
            "  Dependency junctions ({}): {}",
            meta.dependency_plugins.len(),
            meta.dependency_plugins
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    output::print_info(&format!("  Host dir: {:?}", host_dir));
    println!("─────────────────────────────────────────────────────────────");

    if !(force && skip_confirm) {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Cleanup task '{}'? This will delete worktrees, branches, junctions and Host dir.",
                task_id
            ))
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
        if !confirmed {
            output::print_info("Cleanup cancelled.");
            return Ok(());
        }
    }

    output::print_info(&format!("Cleaning up task '{}'...", task_id));

    // Step 1: Remove dependency junctions explicitly so we don't accidentally
    // recurse into the main repo when we later delete the Host dir.
    for dep in &meta.dependency_plugins {
        if let Some(rel) = &dep.junction {
            let junction_abs = host_dir.join(rel);
            if junction_abs.exists() {
                if let Err(e) = crate::junction::delete(&junction_abs) {
                    output::print_warning(&format!(
                        "Failed to remove junction for '{}': {}",
                        dep.name, e
                    ));
                }
            }
        }
    }

    // Step 2: Delete Host directory (with retry for Windows file locking)
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

    // Step 3: Remove worktree for each primary plugin, then delete the branch.
    let mut all_worktrees_removed = true;
    let mut all_branches_deleted = true;
    for primary in &meta.primary_plugins {
        let worktree_abs = host_dir.join(&primary.worktree);
        if worktree_abs.exists() {
            output::print_info(&format!(
                "Removing worktree for '{}': {:?}",
                primary.name, worktree_abs
            ));
            if let Err(e) = git::worktree::remove(&worktree_abs) {
                output::print_warning(&format!("Failed to remove worktree: {}", e));
                all_worktrees_removed = false;
            }
        }
        if !primary.source_repo.as_os_str().is_empty() {
            output::print_info(&format!(
                "Deleting branch '{}' in {:?}",
                primary.branch, primary.source_repo
            ));
            if let Err(e) = git::delete_branch_safe(&primary.source_repo, &primary.branch) {
                output::print_warning(&format!("Failed to delete branch: {}", e));
                all_branches_deleted = false;
            }
            if let Err(e) = git::worktree::prune(&primary.source_repo) {
                output::print_warning(&format!("Failed to prune worktrees: {}", e));
            }
        }
    }

    println!();
    if host_deleted && all_worktrees_removed && all_branches_deleted {
        output::print_success(&format!("Task '{}' cleaned up successfully!", task_id));
    } else {
        output::print_warning(&format!(
            "Task '{}' cleanup incomplete. Some resources may remain.",
            task_id
        ));
    }

    Ok(())
}
