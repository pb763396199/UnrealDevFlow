//! Git operations module

pub mod worktree;

use crate::error::{GitError, Result};
use git2::Repository;
use std::path::Path;

pub fn open_repo(path: &Path) -> Result<Repository> {
    Repository::open(path).map_err(|_| GitError::NotARepo(path.to_path_buf()).into())
}

pub fn get_current_commit(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}

pub fn get_current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    if head.is_branch() {
        Ok(head.shorthand().unwrap_or("HEAD").to_string())
    } else {
        Ok("HEAD".to_string())
    }
}

pub fn create_branch(repo: &Repository, name: &str, commit: &str) -> Result<()> {
    let commit_obj = repo.revparse_single(commit)?.peel_to_commit()?;
    repo.branch(name, &commit_obj, false)?;
    Ok(())
}

pub fn delete_branch(repo: &Repository, name: &str) -> Result<()> {
    let mut branch = repo.find_branch(name, git2::BranchType::Local)?;
    branch.delete()?;
    Ok(())
}

pub fn merge_branch(repo: &Repository, branch_name: &str) -> Result<()> {
    let branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let branch_commit = branch.get().peel_to_commit()?;

    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;

    // Find merge base
    let merge_base = repo.merge_base(head_commit.id(), branch_commit.id())?;
    let merge_base_commit = repo.find_commit(merge_base)?;

    // Check if already up to date
    if merge_base == branch_commit.id() {
        return Ok(());
    }

    // Perform merge
    let mut index = repo.merge_commits(&head_commit, &branch_commit, None)?;

    if index.has_conflicts() {
        return Err(GitError::MergeConflict(format!(
            "Merge conflicts in branch: {}",
            branch_name
        ))
        .into());
    }

    // Create merge commit
    let tree_id = index.write_tree_to(repo)?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;

    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Merge branch '{}'", branch_name),
        &tree,
        &[&head_commit, &branch_commit],
    )?;

    Ok(())
}

/// Rebase the current branch onto the specified branch
/// This is done by calling `git rebase` directly since libgit2 has limited rebase support.
/// On conflict, aborts the rebase and returns an error.
pub fn rebase_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    use std::process::Command;

    // First, make sure we're on the target branch (the one we want to rebase onto)
    // Actually for UnrealDevFlow: we want to rebase the TASK branch onto the CURRENT branch (dev)
    // The standard `git rebase dev` while on task branch will replay task commits onto dev

    // Step 1: Get the current branch (should be dev/main)
    // Step 2: Replay task branch commits onto it

    // For our use case, the user wants:
    // - Currently on dev
    // - Rebase task-xxx onto dev (which means: make dev have all task-xxx's commits on top)

    // The standard way is:
    //   git checkout task-xxx
    //   git rebase dev
    //   git checkout dev
    //   git merge --ff-only task-xxx  (fast-forward dev to task-xxx)
    //   git branch -d task-xxx

    // But since we don't want to checkout the task branch (it has worktree),
    // we can use: git rebase HEAD task-xxx (rebase task-xxx onto current branch)

    let output = Command::new("git")
        .args(["rebase", branch_name])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if it's a conflict
        if stderr.contains("conflict") || stderr.contains("CONFLICT") {
            // Abort the rebase
            let _ = Command::new("git")
                .args(["rebase", "--abort"])
                .current_dir(repo_path)
                .output();
            return Err(GitError::MergeConflict(format!(
                "Rebase conflicts in branch '{}'. Rebase aborted. Please resolve manually.",
                branch_name
            ))
            .into());
        }
        return Err(GitError::CommandFailed(format!("Rebase failed: {}", stderr)).into());
    }

    Ok(())
}

/// Squash merge: combines all commits from the branch into a single commit
pub fn squash_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    use std::process::Command;

    // Use `git merge --squash` which stages all changes but doesn't commit
    // Then we need to commit manually
    let output = Command::new("git")
        .args(["merge", "--squash", branch_name])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("conflict") || stderr.contains("CONFLICT") {
            // Abort the merge
            let _ = Command::new("git")
                .args(["merge", "--abort"])
                .current_dir(repo_path)
                .output();
            return Err(GitError::MergeConflict(format!(
                "Squash conflicts in branch '{}'. Merge aborted.",
                branch_name
            ))
            .into());
        }
        return Err(GitError::CommandFailed(format!("Squash failed: {}", stderr)).into());
    }

    // Commit the squashed changes with auto-generated message
    let commit_msg = format!("Squash task branch '{}'", branch_name);
    let commit_output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(repo_path)
        .output()?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        return Err(GitError::CommandFailed(format!(
            "Failed to commit squash: {}",
            stderr
        ))
        .into());
    }

    Ok(())
}

/// Fast-forward only merge: only succeeds if the target branch is directly ahead
pub fn ff_only_merge(repo_path: &Path, branch_name: &str) -> Result<()> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["merge", "--ff-only", branch_name])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Not possible to fast-forward") || stderr.contains("not a fast-forward") {
            return Err(GitError::MergeConflict(format!(
                "Fast-forward not possible for branch '{}'. Try a different strategy.",
                branch_name
            ))
            .into());
        }
        return Err(GitError::CommandFailed(format!("FF-only merge failed: {}", stderr)).into());
    }

    Ok(())
}

/// After a successful merge/rebase, delete the original task branch
pub fn delete_branch_safe(repo_path: &Path, branch_name: &str) -> Result<()> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["branch", "-D", branch_name])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!("Failed to delete branch: {}", stderr)).into());
    }

    Ok(())
}
