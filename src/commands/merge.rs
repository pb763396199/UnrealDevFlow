//! Merge command implementation

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host;
use crate::output;
use std::path::PathBuf;

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

/// Get commit count in branch (what will be merged)
fn get_commit_count(repo: &git2::Repository, branch_name: &str) -> Result<usize> {
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

pub fn run(
    task_id: &str,
    strategy: &crate::cli::MergeStrategy,
    force: bool,
    skip_confirm: bool,
    dry_run: bool,
) -> Result<()> {
    let config = Config::load()?;

    // Get task host
    let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
    let meta = host::read_meta(&host_dir)?;

    // Validate branch name matches expected pattern
    let expected_branch = format!("task-{}", task_id);
    if meta.branch != expected_branch {
        return Err(UdfError::Other(format!(
            "Branch name '{}' does not match expected '{}'. Aborting to prevent accidental merge.",
            meta.branch, expected_branch
        )));
    }

    // Open main repo
    let repo = git::open_repo(&config.plugin_path)?;

    // === PRE-FLIGHT CHECKS ===

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

    // Get commit count
    let commit_count = get_commit_count(&repo, &meta.branch)?;

    // === BRANCH STATE COMPARISON ===
    // Check what's ahead (in current branch but not in task branch) and behind
    let commits_ahead = git::get_commits_ahead(&config.plugin_path, &meta.branch).unwrap_or_default();
    let commits_behind = git::get_commits_behind(&config.plugin_path, &meta.branch).unwrap_or_default();

    // === DRY RUN MODE ===
    if dry_run {
        output::print_info(&format!("Dry run: would merge task '{}'", task_id));
        output::print_info(&format!("  Strategy: {:?}", strategy));
        output::print_info(&format!("  Branch: {} ({} commit(s) to merge)", meta.branch, commit_count));
        output::print_info(&format!("  Worktree: {:?}", worktree_path));
        output::print_info(&format!("  Host directory: {:?}", host_dir));

        if !commits_ahead.is_empty() {
            output::print_warning(&format!(
                "  Current branch is {} commit(s) ahead of task branch:",
                commits_ahead.len()
            ));
            for commit in &commits_ahead {
                output::print_warning(&format!("    {}", commit));
            }
        }

        if !commits_behind.is_empty() {
            output::print_warning(&format!(
                "  Task branch is {} commit(s) ahead of current branch:",
                commits_behind.len()
            ));
            for commit in &commits_behind {
                output::print_warning(&format!("    {}", commit));
            }
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

        // Strategy-specific preview
        match strategy {
            crate::cli::MergeStrategy::Rebase => {
                output::print_info("  Rebase will replay task commits onto current branch (linear history)");
            }
            crate::cli::MergeStrategy::Merge => {
                output::print_info("  Merge will create a merge commit, preserving task history");
            }
            crate::cli::MergeStrategy::Squash => {
                output::print_info("  Squash will combine all task commits into a single commit");
            }
            crate::cli::MergeStrategy::FfOnly => {
                output::print_info("  FF-only will only succeed if fast-forward is possible");
            }
        }

        return Ok(());
    }

    // === PREVIEW AND CONFIRMATION ===
    println!();
    output::print_info(&format!("Merge task '{}'", task_id));
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!("  Strategy: {:?}", strategy));
    output::print_info(&format!("  Branch:   {} ({} commit(s) to merge)", meta.branch, commit_count));
    output::print_info(&format!("  Worktree: {:?}", worktree_path));
    output::print_info(&format!("  Host dir: {:?}", host_dir));

    // Show branch divergence
    if !commits_ahead.is_empty() {
        println!();
        output::print_warning(&format!(
            "Current branch is {} commit(s) ahead of '{}':",
            commits_ahead.len(),
            meta.branch
        ));
        for commit in &commits_ahead {
            output::print_info(&format!("  ↑ {}", commit));
        }
    }

    if !commits_behind.is_empty() {
        println!();
        output::print_warning(&format!(
            "Task branch '{}' is {} commit(s) ahead of current:",
            meta.branch,
            commits_behind.len()
        ));
        for commit in &commits_behind {
            output::print_info(&format!("  ↓ {}", commit));
        }
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
        output::print_warning("  These changes will be permanently lost after merge!");
    }

    println!("─────────────────────────────────────────────────────────────");

    // Confirmation
    if !(force && skip_confirm) {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(&format!("Merge task '{}' using {:?} strategy?", task_id, strategy))
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;

        if !confirmed {
            output::print_info("Merge cancelled.");
            return Ok(());
        }

        // Second confirmation if there are uncommitted changes
        if has_uncommitted {
            let double_confirmed = dialoguer::Confirm::new()
                .with_prompt("Uncommitted changes will be lost. Continue?")
                .default(false)
                .interact()
                .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;

            if !double_confirmed {
                output::print_info("Merge cancelled.");
                return Ok(());
            }
        }
    }

    output::print_info(&format!(
        "Merging task '{}' using {:?} strategy...",
        task_id, strategy
    ));

    // === EXECUTE STRATEGY ===
    let merge_success = match strategy {
        crate::cli::MergeStrategy::Rebase => {
            output::print_info(&format!("Rebasing '{}' onto current branch...", meta.branch));
            match git::rebase_branch(&config.plugin_path, &meta.branch, &meta.based_on) {
                Ok(_) => {
                    output::print_success(&format!(
                        "Branch '{}' rebased successfully",
                        meta.branch
                    ));
                    true
                }
                Err(e) => {
                    output::print_error(&format!("Rebase failed: {}", e));
                    output::print_info("The rebase has been aborted. Please resolve conflicts manually.");
                    if force {
                        output::print_warning("Force flag set, continuing with cleanup...");
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        crate::cli::MergeStrategy::Merge => {
            match git::merge_branch(&repo, &meta.branch) {
                Ok(_) => {
                    output::print_success(&format!(
                        "Branch '{}' merged successfully",
                        meta.branch
                    ));
                    true
                }
                Err(e) => {
                    if force {
                        output::print_warning(&format!("Merge failed: {}. Forcing deletion...", e));
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        crate::cli::MergeStrategy::Squash => {
            output::print_info(&format!("Squashing '{}' into current branch...", meta.branch));
            match git::squash_branch(&config.plugin_path, &meta.branch) {
                Ok(_) => {
                    output::print_success(&format!(
                        "Branch '{}' squashed successfully",
                        meta.branch
                    ));
                    true
                }
                Err(e) => {
                    output::print_error(&format!("Squash failed: {}", e));
                    if force {
                        output::print_warning("Force flag set, continuing with cleanup...");
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        crate::cli::MergeStrategy::FfOnly => {
            output::print_info(&format!(
                "Fast-forward merging '{}' into current branch...",
                meta.branch
            ));
            match git::ff_only_merge(&config.plugin_path, &meta.branch) {
                Ok(_) => {
                    output::print_success(&format!(
                        "Branch '{}' fast-forward merged successfully",
                        meta.branch
                    ));
                    true
                }
                Err(e) => {
                    if force {
                        output::print_warning(&format!("FF merge failed: {}. Forcing deletion...", e));
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    };

    // === NO CLEANUP - merge only ===
    // Worktree and branch are RETAINED for user inspection
    // User must manually run: unrealdevflow cleanup <task-id>

    println!();
    if merge_success {
        output::print_success(&format!(
            "Task '{}' merged using {:?} successfully!",
            task_id, strategy
        ));
        output::print_info("⚠️  Worktree and branch RETAINED for inspection.");
        output::print_info(&format!("  Worktree: {:?}", worktree_path));
        output::print_info(&format!("  Branch:   {}", meta.branch));
        output::print_info(&format!("  Host dir: {:?}", host_dir));
        output::print_info("");
        output::print_info("To cleanup after verification:");
        output::print_info(&format!("  unrealdevflow cleanup {}", task_id));
    } else {
        output::print_warning(&format!(
            "Task '{}' merge failed. Worktree and branch retained for debugging.",
            task_id
        ));
        output::print_info(&format!("  Worktree: {:?}", worktree_path));
        output::print_info(&format!("  Branch:   {}", meta.branch));
    }

    Ok(())
}
