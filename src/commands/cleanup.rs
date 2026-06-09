//! Cleanup command implementation
//! 
//! Manually cleanup worktree and branch after merge verification.

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host;
use crate::output;
use std::path::PathBuf;

/// Delete directory with retry logic for Windows file locking
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

    // Get task host
    let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
    let meta = host::read_meta(&host_dir)?;

    // Validate branch name matches expected pattern
    let expected_branch = format!("task-{}", task_id);
    if meta.branch != expected_branch {
        return Err(UdfError::Other(format!(
            "Branch name '{}' does not match expected '{}'. Aborting to prevent accidental deletion.",
            meta.branch, expected_branch
        )));
    }

    // Open main repo
    let repo = git::open_repo(&config.plugin_path)?;

    let worktree_path = host_dir.join("Plugins").join("AesWorld");

    // === PREVIEW AND CONFIRMATION ===
    println!();
    output::print_info(&format!("Cleanup task '{}'", task_id));
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!("  Branch:   {}", meta.branch));
    output::print_info(&format!("  Worktree: {:?}", worktree_path));
    output::print_info(&format!("  Host dir: {:?}", host_dir));
    println!("─────────────────────────────────────────────────────────────");

    // Confirmation
    if !(force && skip_confirm) {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(&format!("Cleanup task '{}'? This will delete worktree and branch.", task_id))
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;

        if !confirmed {
            output::print_info("Cleanup cancelled.");
            return Ok(());
        }
    }

    output::print_info(&format!("Cleaning up task '{}'...", task_id));

    // === CLEANUP (safe order: host dir → worktree → branch) ===

    // Step 1: Delete Host directory (with retry for Windows file locking)
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

    // Step 2: Remove worktree
    let worktree_removed = if worktree_path.exists() {
        output::print_info("Removing worktree...");
        match git::worktree::remove(&worktree_path) {
            Ok(_) => true,
            Err(e) => {
                output::print_warning(&format!("Failed to remove worktree: {}", e));
                output::print_info("Attempting manual cleanup...");

                if let Err(e) = git::worktree::prune(&config.plugin_path) {
                    output::print_warning(&format!("Failed to prune worktrees: {}", e));
                }

                false
            }
        }
    } else {
        true
    };

    // Step 3: Delete task branch
    output::print_info(&format!("Deleting branch '{}'...", meta.branch));
    let branch_deleted = match git::delete_branch_safe(&config.plugin_path, &meta.branch) {
        Ok(_) => true,
        Err(e) => {
            output::print_warning(&format!("Failed to delete branch: {}", e));
            false
        }
    };

    // Step 4: Always run worktree prune to clean up any stale metadata
    output::print_info("Pruning worktree metadata...");
    if let Err(e) = git::worktree::prune(&config.plugin_path) {
        output::print_warning(&format!("Failed to prune worktrees: {}", e));
    }

    // === REPORT RESULTS ===
    println!();
    if worktree_removed && branch_deleted && host_deleted {
        output::print_success(&format!("Task '{}' cleaned up successfully!", task_id));
    } else {
        output::print_warning(&format!(
            "Task '{}' cleanup incomplete. Some resources may remain:",
            task_id
        ));
        if !worktree_removed {
            output::print_warning(&format!("  - Worktree: {:?}", worktree_path));
            output::print_info("    Run: git worktree prune");
        }
        if !branch_deleted {
            output::print_warning(&format!("  - Branch: {}", meta.branch));
            output::print_info(&format!("    Run: git branch -D {}", meta.branch));
        }
        if !host_deleted {
            output::print_warning(&format!("  - Host directory: {:?}", host_dir));
            output::print_info(&format!(
                "    Run: Remove-Item -Recurse -Force {:?}",
                host_dir
            ));
        }
    }

    Ok(())
}
