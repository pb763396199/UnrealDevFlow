//! Git worktree management

use crate::error::{GitError, Result};
use git2::Repository;
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

pub fn remove(worktree_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "remove", "--force", &worktree_path.to_string_lossy()])
        .current_dir(worktree_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::Worktree(format!("Failed to remove worktree: {}", stderr)).into());
    }

    Ok(())
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
