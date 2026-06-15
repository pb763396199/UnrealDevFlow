//! Git operations module

pub mod worktree;

use crate::error::{GitError, Result};
use crate::output;
use git2::Repository;
use std::path::Path;

pub fn open_repo(path: &Path) -> Result<Repository> {
    Repository::open(path).map_err(|_| GitError::NotARepo(path.to_path_buf()).into())
}

/// Fetch latest from origin (git fetch origin)
pub fn fetch_origin(repo_path: &Path) -> Result<()> {
    use std::process::Command;

    output::print_info("Fetching latest from origin...");
    let output = Command::new("git")
        .args(["fetch", "origin"])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(
            GitError::CommandFailed(format!("Failed to fetch from origin: {}", stderr)).into(),
        );
    }

    output::print_success("Fetch completed");
    Ok(())
}

/// Fast-forward the current branch to its upstream when one is configured.
pub fn fast_forward_upstream(repo_path: &Path) -> Result<()> {
    use std::process::Command;

    let upstream_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(repo_path)
        .output()?;

    if !upstream_output.status.success() {
        return Ok(());
    }

    let upstream = String::from_utf8_lossy(&upstream_output.stdout)
        .trim()
        .to_string();
    if upstream.is_empty() {
        return Ok(());
    }

    output::print_info(&format!(
        "Fast-forwarding current branch from upstream '{}'...",
        upstream
    ));
    let output = Command::new("git")
        .args(["merge", "--ff-only", &upstream])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "Failed to fast-forward from upstream '{}': {}",
            upstream, stderr
        ))
        .into());
    }

    Ok(())
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
    // Check if already up to date
    if merge_base == branch_commit.id() {
        return Ok(());
    }

    // Perform merge
    let mut index = repo.merge_commits(&head_commit, &branch_commit, None)?;

    if index.has_conflicts() {
        return Err(
            GitError::MergeConflict(format!("Merge conflicts in branch: {}", branch_name)).into(),
        );
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

/// Rebase task commits onto the current target branch without rewriting the
/// already-published target commits or the original task branch.
///
/// Correct flow:
/// 1. Record the current target branch tip after it has been updated.
/// 2. Reset the target branch to the task branch tip.
/// 3. Rebase the target branch commits after `based_on` onto the recorded tip.
///
/// This preserves all existing target-branch commit ids and leaves the task
/// branch/worktree untouched. Only the task commits copied onto the target
/// branch receive new ids.
///
/// On conflict, aborts and restores the target branch tip before returning an
/// error.
pub fn rebase_branch(repo_path: &Path, branch_name: &str, based_on: &str) -> Result<()> {
    use std::process::Command;

    let target_branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()?;
    if !target_branch_output.status.success() {
        let stderr = String::from_utf8_lossy(&target_branch_output.stderr);
        return Err(
            GitError::CommandFailed(format!("Failed to read target branch: {}", stderr)).into(),
        );
    }
    let target_branch = String::from_utf8_lossy(&target_branch_output.stdout)
        .trim()
        .to_string();

    let target_tip_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()?;
    if !target_tip_output.status.success() {
        let stderr = String::from_utf8_lossy(&target_tip_output.stderr);
        return Err(
            GitError::CommandFailed(format!("Failed to read target tip: {}", stderr)).into(),
        );
    }
    let target_tip = String::from_utf8_lossy(&target_tip_output.stdout)
        .trim()
        .to_string();

    let task_tip_output = Command::new("git")
        .args(["rev-parse", branch_name])
        .current_dir(repo_path)
        .output()?;
    if !task_tip_output.status.success() {
        let stderr = String::from_utf8_lossy(&task_tip_output.stderr);
        return Err(
            GitError::CommandFailed(format!("Failed to read task branch tip: {}", stderr)).into(),
        );
    }
    let task_tip = String::from_utf8_lossy(&task_tip_output.stdout)
        .trim()
        .to_string();

    output::print_info(&format!(
        "Rebasing task branch '{}' onto '{}' ({})",
        branch_name,
        target_branch,
        &target_tip[..8.min(target_tip.len())]
    ));

    output::print_info(&format!(
        "  Step 1: Reset '{}' to task branch tip ({})",
        target_branch,
        &task_tip[..8.min(task_tip.len())]
    ));
    let reset_output = Command::new("git")
        .args(["reset", "--hard", &task_tip])
        .current_dir(repo_path)
        .output()?;
    if !reset_output.status.success() {
        let stderr = String::from_utf8_lossy(&reset_output.stderr);
        return Err(GitError::CommandFailed(format!(
            "Failed to reset '{}' to task branch '{}': {}",
            target_branch, branch_name, stderr
        ))
        .into());
    }

    output::print_info("  Step 2: Rebase task commits onto recorded target tip");
    let output = Command::new("git")
        .args(["rebase", "--onto", &target_tip, based_on])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("conflict") || stderr.contains("CONFLICT") {
            let _ = Command::new("git")
                .args(["rebase", "--abort"])
                .current_dir(repo_path)
                .output();
            let _ = Command::new("git")
                .args(["reset", "--hard", &target_tip])
                .current_dir(repo_path)
                .output();
            return Err(GitError::MergeConflict(format!(
                "Rebase conflict while replaying task branch '{}'. Aborted and restored '{}'.",
                branch_name, target_branch
            ))
            .into());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let _ = Command::new("git")
            .args(["rebase", "--abort"])
            .current_dir(repo_path)
            .output();
        let restore_output = Command::new("git")
            .args(["reset", "--hard", &target_tip])
            .current_dir(repo_path)
            .output()?;
        if !restore_output.status.success() {
            return Err(GitError::CommandFailed(format!(
                "Failed to rebase task branch '{}': {}\n{}\nAlso failed to restore '{}'.",
                branch_name, stderr, stdout, target_branch
            ))
            .into());
        }
        return Err(GitError::CommandFailed(format!(
            "Failed to rebase task branch '{}': {}\n{}",
            branch_name, stderr, stdout
        ))
        .into());
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
        return Err(GitError::CommandFailed(format!("Failed to commit squash: {}", stderr)).into());
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
        if stderr.contains("Not possible to fast-forward") || stderr.contains("not a fast-forward")
        {
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
