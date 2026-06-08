//! Git operations module

pub mod worktree;

use crate::error::{GitError, Result};
use crate::output;
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

/// Rebase the task branch's commits onto the current branch (dev).
///
/// Correct flow (per user requirements):
/// 1. Reset current branch (dev) to the based_on commit (task creation point)
/// 2. Cherry-pick dev's original new commits (in chronological order)
/// 3. Cherry-pick task branch's commits (puts them at the top)
///
/// This produces a linear history with task commits at the top.
///
/// On conflict, aborts and returns an error.
pub fn rebase_branch(repo_path: &Path, branch_name: &str, based_on: &str) -> Result<()> {
    use std::process::Command;

    output::print_info(&format!("Rebasing '{}' onto dev (3-step process)", branch_name));

    // Step 1: Reset dev to based_on
    output::print_info(&format!("  Step 1: Reset dev to based_on ({})", &based_on[..8.min(based_on.len())]));
    let output = Command::new("git")
        .args(["reset", "--hard", based_on])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!("Failed to reset dev to based_on: {}", stderr)).into());
    }

    // Step 2: Cherry-pick dev's original new commits (chronological order)
    output::print_info("  Step 2: Cherry-pick dev's original new commits");

    // Get the list of commits that were in HEAD before reset, in reverse order (oldest first)
    let log_output = Command::new("git")
        .args(["log", "--oneline", "--reverse", &format!("{}..HEAD@{{1}}", based_on)])
        .current_dir(repo_path)
        .output()?;

    let log_stdout = String::from_utf8_lossy(&log_output.stdout);
    let commits_to_pick: Vec<String> = log_stdout
        .lines()
        .filter_map(|line| line.split_whitespace().next().map(|s| s.to_string()))
        .collect();

    for commit in &commits_to_pick {
        output::print_info(&format!("    Cherry-picking {}", &commit[..8.min(commit.len())]));
        let cherry_output = Command::new("git")
            .args(["cherry-pick", commit])
            .current_dir(repo_path)
            .output()?;

        if !cherry_output.status.success() {
            let stderr = String::from_utf8_lossy(&cherry_output.stderr);
            // Check if it's a conflict
            if stderr.contains("conflict") || stderr.contains("CONFLICT") {
                // Abort the cherry-pick
                let _ = Command::new("git")
                    .args(["cherry-pick", "--abort"])
                    .current_dir(repo_path)
                    .output();
                return Err(GitError::MergeConflict(format!(
                    "Cherry-pick conflict on {}. Aborted. Please resolve manually.",
                                    &commit[..8.min(commit.len())]
                                ))
                                .into());
            }
            return Err(GitError::CommandFailed(format!("Cherry-pick failed for {}: {}", commit, stderr)).into());
        }
    }

    // Step 3: Cherry-pick task branch's commits
    output::print_info(&format!("  Step 3: Cherry-pick task branch '{}' commits", branch_name));

    // Get task branch commits (in chronological order, oldest first)
    let task_log_output = Command::new("git")
        .args(["log", "--oneline", "--reverse", &format!("{}..{}", based_on, branch_name)])
        .current_dir(repo_path)
        .output()?;

    let task_log_stdout = String::from_utf8_lossy(&task_log_output.stdout);
    let task_commits: Vec<String> = task_log_stdout
        .lines()
        .filter_map(|line| line.split_whitespace().next().map(|s| s.to_string()))
        .collect();

    for commit in &task_commits {
        output::print_info(&format!("    Cherry-picking task commit {}", &commit[..8.min(commit.len())]));
        let cherry_output = Command::new("git")
            .args(["cherry-pick", commit])
            .current_dir(repo_path)
            .output()?;

        if !cherry_output.status.success() {
            let stderr = String::from_utf8_lossy(&cherry_output.stderr);
            if stderr.contains("conflict") || stderr.contains("CONFLICT") {
                let _ = Command::new("git")
                    .args(["cherry-pick", "--abort"])
                    .current_dir(repo_path)
                    .output();
                return Err(GitError::MergeConflict(format!(
                    "Cherry-pick conflict on task commit {}. Aborted. Please resolve manually.",
                                    &commit[..8.min(commit.len())]
                                ))
                                .into());
            }
            return Err(GitError::CommandFailed(format!("Cherry-pick failed for {}: {}", commit, stderr)).into());
        }
    }

    output::print_success("Rebase completed successfully");
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

/// Get commits that are in the current branch but not in the given branch
pub fn get_commits_ahead(repo_path: &Path, branch_name: &str) -> Result<Vec<String>> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["log", "--oneline", &format!("{}..HEAD", branch_name)])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}

/// Get commits that are in the given branch but not in the current branch
pub fn get_commits_behind(repo_path: &Path, branch_name: &str) -> Result<Vec<String>> {
    use std::process::Command;

    let output = Command::new("git")
        .args(["log", "--oneline", &format!("HEAD..{}", branch_name)])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}
