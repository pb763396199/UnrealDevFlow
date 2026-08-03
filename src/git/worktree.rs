//! Git worktree management

use crate::error::{GitError, Result};
use std::path::Path;
use std::process::Command;

pub fn add(repo_path: &Path, worktree_path: &Path, commit: &str, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            &worktree_path.to_string_lossy(),
            commit,
            "-b",
            branch,
        ])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Worktree(format!("Failed to add worktree: {}", stderr)).into());
    }

    Ok(())
}

pub fn update_submodules(worktree_path: &Path) -> Result<()> {
    if !worktree_path.join(".gitmodules").exists() {
        return Ok(());
    }
    let output = Command::new("git")
        .args(["submodule", "update", "--init", "--recursive"])
        .current_dir(worktree_path)
        .output()?;
    if !output.status.success() {
        return Err(GitError::Worktree(format!(
            "Failed to initialize submodules: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
        .into());
    }
    Ok(())
}

/// Remove a worktree, driving git from the owning repository.
///
/// `repo_path` is not a convenience: on Windows a process whose working
/// directory is inside the worktree holds that directory open, so git cannot
/// delete it and the removal fails halfway.
pub fn remove(repo_path: &Path, worktree_path: &Path) -> Result<()> {
    // Migrated v1 metadata can carry an empty source_repo. Fall back to the
    // worktree's parent — still outside the directory being deleted.
    let git_cwd = if repo_path.is_dir() {
        repo_path
    } else {
        worktree_path.parent().unwrap_or(worktree_path)
    };

    // First, try the standard `git worktree remove --force` command
    let output = Command::new("git")
        .args([
            "worktree",
            "remove",
            "--force",
            &worktree_path.to_string_lossy(),
        ])
        .current_dir(git_cwd)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    // The directory may already be gone; prune stale references from the owning
    // repository and re-check before reporting failure.
    let _ = Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(git_cwd)
        .output();

    // Check if the worktree reference is gone now
    let check_output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(git_cwd)
        .output()?;

    if check_output.status.success() {
        let stdout = String::from_utf8_lossy(&check_output.stdout);
        if !stdout.contains(&worktree_path.to_string_lossy().to_string()) {
            // Worktree reference is gone
            return Ok(());
        }
    }

    // If still failing, return the original error
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(GitError::Worktree(format!("Failed to remove worktree: {}", stderr)).into())
}

pub fn lock(worktree_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "lock", &worktree_path.to_string_lossy()])
        .current_dir(worktree_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Worktree(format!("Failed to lock worktree: {}", stderr)).into());
    }

    Ok(())
}

pub fn unlock(worktree_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "unlock", &worktree_path.to_string_lossy()])
        .current_dir(worktree_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Worktree(format!("Failed to unlock worktree: {}", stderr)).into());
    }

    Ok(())
}

pub fn list(repo_path: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Worktree(format!("Failed to list worktrees: {}", stderr)).into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let worktrees: Vec<String> = stdout
        .lines()
        .filter(|line| line.starts_with("worktree "))
        .map(|line| line.trim_start_matches("worktree ").to_string())
        .collect();

    Ok(worktrees)
}

pub fn prune(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Worktree(format!("Failed to prune worktrees: {}", stderr)).into());
    }

    Ok(())
}
