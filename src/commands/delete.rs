//! Delete command implementation

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host;
use crate::output;
use std::path::PathBuf;

/// Check if a branch has been merged into the current HEAD
fn is_branch_merged(repo: &git2::Repository, branch_name: &str) -> Result<bool> {
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .map_err(|e| UdfError::Other(format!("Failed to find branch '{}': {}", branch_name, e)))?;
    let branch_commit = branch.get().peel_to_commit().map_err(|e| {
        UdfError::Other(format!("Failed to get commit for branch '{}': {}", branch_name, e))
    })?;

    let head = repo
        .head()
        .map_err(|e| UdfError::Other(format!("Failed to get HEAD: {}", e)))?;
    let head_commit = head.peel_to_commit().map_err(|e| {
        UdfError::Other(format!("Failed to get HEAD commit: {}", e))
    })?;

    match repo.merge_base(head_commit.id(), branch_commit.id()) {
        Ok(merge_base) => Ok(merge_base == branch_commit.id()),
        Err(_) => Ok(false), // If no merge base, branch is not merged
    }
}

/// Check if worktree has uncommitted changes
fn has_uncommitted_changes(worktree_path: &PathBuf) -> Result<bool> {
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

/// Get list of uncommitted changes
fn get_uncommitted_changes(worktree_path: &PathBuf) -> Result<Vec<String>> {
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
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}

/// Get unmerged commit count
fn get_unmerged_commit_count(repo: &git2::Repository, branch_name: &str) -> Result<usize> {
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .map_err(|e| UdfError::Other(format!("Failed to find branch '{}': {}", branch_name, e)))?;
    let branch_commit = branch.get().peel_to_commit().map_err(|e| {
        UdfError::Other(format!("Failed to get commit for branch '{}': {}", branch_name, e))
    })?;

    let head = repo
        .head()
        .map_err(|e| UdfError::Other(format!("Failed to get HEAD: {}", e)))?;
    let head_commit = head.peel_to_commit().map_err(|e| {
        UdfError::Other(format!("Failed to get HEAD commit: {}", e))
    })?;

    // Count commits in branch that are not in HEAD
    let mut count = 0;
    let mut current = branch_commit;
    while current.id() != head_commit.id() {
        count += 1;
        if current.parent_count() == 0 {
            break;
        }
        current = current.parent(0).map_err(|e| {
            UdfError::Other(format!("Failed to get parent commit: {}", e))
        })?;
    }

    Ok(count)
}

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

pub fn run(task_id: &str, force: bool, skip_confirm: bool, dry_run: bool) -> Result<()> {
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

    // Open main repo for checks
    let repo = git::open_repo(&config.plugin_path)?;

    // === PRE-FLIGHT CHECKS ===

    // Check merge status
    let is_merged = is_branch_merged(&repo, &meta.branch)?;
    let unmerged_count = if !is_merged {
        get_unmerged_commit_count(&repo, &meta.branch)?
    } else {
        0
    };

    // Check for uncommitted changes
    let worktree_path = host_dir.join("Plugins").join("AesWorld");
    let has_uncommitted = if worktree_path.exists() {
        has_uncommitted_changes(&worktree_path)?
    } else {
        false
    };
    let uncommitted_files = if has_uncommitted {
        get_uncommitted_changes(&worktree_path)?
    } else {
        vec![]
    };

    // === DRY RUN MODE ===
    if dry_run {
        output::print_info(&format!("Dry run: would delete task '{}'", task_id));
        output::print_info(&format!("  Worktree: {:?}", worktree_path));
        output::print_info(&format!("  Branch: {} (merged: {})", meta.branch, is_merged));
        output::print_info(&format!("  Host directory: {:?}", host_dir));

        if !is_merged {
            output::print_warning(&format!(
                "  ⚠ Branch has {} unmerged commit(s) that will be lost!",
                unmerged_count
            ));
        }

        if has_uncommitted {
            output::print_warning(&format!(
                "  ⚠ Worktree has {} uncommitted change(s) that will be lost:",
                uncommitted_files.len()
            ));
            for file in &uncommitted_files {
                output::print_warning(&format!("    {}", file));
            }
        }

        return Ok(());
    }

    // === PREVIEW AND CONFIRMATION ===
    println!();
    output::print_info(&format!("Delete task '{}'", task_id));
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!("  Worktree: {:?}", worktree_path));
    output::print_info(&format!("  Branch:   {} (merged: {})", meta.branch, is_merged));
    output::print_info(&format!("  Host dir: {:?}", host_dir));

    if !is_merged {
        println!();
        output::print_warning(&format!(
            "⚠ WARNING: Branch '{}' has {} unmerged commit(s)!",
            meta.branch, unmerged_count
        ));
        output::print_warning("  These commits will be permanently lost!");
    }

    if has_uncommitted {
        println!();
        output::print_warning(&format!(
            "⚠ WARNING: Worktree has {} uncommitted change(s):",
            uncommitted_files.len()
        ));
        for file in &uncommitted_files {
            output::print_warning(&format!("  {}", file));
        }
        output::print_warning("  These changes will be permanently lost!");
    }

    if !is_merged || has_uncommitted {
        println!();
        output::print_warning("This operation CANNOT be undone!");
    }

    println!("─────────────────────────────────────────────────────────────");

    // Confirmation (skip only with --force --yes, not just one)
    if !(force && skip_confirm) {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(&format!(
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

        // Second confirmation for dangerous operations
        if !is_merged || has_uncommitted {
            let double_confirmed = dialoguer::Confirm::new()
                .with_prompt("This will permanently delete unmerged work. Type 'yes' to confirm")
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

    // === DELETION (safe order: host dir → worktree → branch) ===

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

                // Manual cleanup: prune worktrees
                if let Err(e) = git::worktree::prune(&config.plugin_path) {
                    output::print_warning(&format!("Failed to prune worktrees: {}", e));
                }

                false
            }
        }
    } else {
        true
    };

    // Step 3: Delete branch (last, as it's the hardest to recover from)
    output::print_info(&format!("Deleting branch '{}'...", meta.branch));
    let branch_deleted = match git::delete_branch(&repo, &meta.branch) {
        Ok(_) => true,
        Err(e) => {
            output::print_warning(&format!("Failed to delete branch: {}", e));
            false
        }
    };

    // === REPORT RESULTS ===
    println!();
    if worktree_removed && branch_deleted && host_deleted {
        output::print_success(&format!("Task '{}' deleted successfully!", task_id));
    } else {
        output::print_warning(&format!(
            "Task '{}' partially deleted. Some resources may remain:",
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
