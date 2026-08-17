//! Git operations module

pub mod branch_ledger;
pub mod worktree;

use crate::error::{GitError, Result};
use crate::output;
use git2::Repository;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn resolve_commit(repo_path: &Path, reference: &str) -> Result<String> {
    git_stdout(
        repo_path,
        &[
            "rev-parse",
            "--verify",
            &format!("{}^{{commit}}", reference),
        ],
    )
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

/// Merge a task branch into the checked-out branch of `repo_path`.
///
/// This used to build the merge in memory with git2 and commit it with
/// `repo.commit(Some("HEAD"), ..)`. That moves the branch ref but touches
/// neither the index nor the working tree, so the repository was left with
/// HEAD ahead of both: every file the merge introduced showed up as a staged
/// change nobody made, and the next `task create` was refused for a dirty
/// working tree. Shell out like the other three strategies already do and let
/// git move all three together.
pub fn merge_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    let output = Command::new("git")
        .args([
            "merge",
            "--no-ff",
            "--no-edit",
            "-m",
            &format!("Merge branch '{}'", branch_name),
            branch_name,
        ])
        .current_dir(repo_path)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Leaving a half-finished merge behind would strand the repo in MERGING
    // state, which is worse than the failure we are already reporting.
    let _ = Command::new("git")
        .args(["merge", "--abort"])
        .current_dir(repo_path)
        .output();

    if stdout.contains("CONFLICT") || stderr.contains("CONFLICT") {
        return Err(
            GitError::MergeConflict(format!("Merge conflicts in branch: {}", branch_name)).into(),
        );
    }
    Err(GitError::CommandFailed(format!(
        "Failed to merge branch '{}': {}{}",
        branch_name, stdout, stderr
    ))
    .into())
}

fn git_stdout(repo_path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!("git {:?} failed: {}", args, stderr)).into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn command_stdout(repo_path: &Path, args: &[&str]) -> Result<String> {
    git_stdout(repo_path, args)
}

pub fn status_porcelain(repo_path: &Path) -> Result<String> {
    git_stdout(repo_path, &["status", "--porcelain"])
}

/// Return only tracked/index/conflict changes, excluding untracked files.
///
/// Task creation uses this boundary because untracked files are not part of
/// the commit from which a new worktree is created, and moving them would
/// introduce destructive stash/restore behavior.
pub fn status_porcelain_tracked(repo_path: &Path) -> Result<String> {
    git_stdout(
        repo_path,
        &["status", "--porcelain=v1", "--untracked-files=no"],
    )
}

pub fn branch_exists(repo_path: &Path, branch: &str) -> Result<bool> {
    git_success(
        repo_path,
        &["rev-parse", "--verify", &format!("refs/heads/{}", branch)],
    )
}

fn normalize_path_for_compare(path: &Path) -> String {
    let path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut text = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        text = text.to_ascii_lowercase();
    }
    text.trim_end_matches('/').to_string()
}

fn same_path(left: &Path, right: &Path) -> bool {
    normalize_path_for_compare(left) == normalize_path_for_compare(right)
}

fn main_worktree_from_common_dir(common_dir: &Path) -> Option<PathBuf> {
    if common_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case(".git"))
        .unwrap_or(false)
    {
        common_dir.parent().map(|path| path.to_path_buf())
    } else {
        None
    }
}

/// Return the main worktree path when `repo_path` is a linked worktree.
///
/// UnrealDevFlow must create new task worktrees from the primary plugin's main
/// checkout. If a workspace points at a UE project `Plugins` entry that is a
/// Junction to another task worktree, Git still opens it successfully, but HEAD
/// belongs to that task branch. This helper detects that situation.
pub fn linked_worktree_main(repo_path: &Path) -> Result<Option<PathBuf>> {
    let top_level = PathBuf::from(git_stdout(
        repo_path,
        &["rev-parse", "--path-format=absolute", "--show-toplevel"],
    )?);
    let common_dir = PathBuf::from(git_stdout(
        repo_path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?);

    let Some(main_worktree) = main_worktree_from_common_dir(&common_dir) else {
        return Ok(None);
    };

    if same_path(&top_level, &main_worktree) {
        Ok(None)
    } else {
        Ok(Some(main_worktree))
    }
}

fn git_success(repo_path: &Path, args: &[&str]) -> Result<bool> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()?;
    Ok(output.status.success())
}

fn git_is_ancestor(repo_path: &Path, ancestor: &str, descendant: &str) -> Result<bool> {
    git_success(
        repo_path,
        &["merge-base", "--is-ancestor", ancestor, descendant],
    )
}

fn restore_target_tip(repo_path: &Path, target_tip: &str) -> Result<()> {
    let _ = Command::new("git")
        .args(["cherry-pick", "--abort"])
        .current_dir(repo_path)
        .output();

    let restore_output = Command::new("git")
        .args(["reset", "--hard", target_tip])
        .current_dir(repo_path)
        .output()?;
    if !restore_output.status.success() {
        let stderr = String::from_utf8_lossy(&restore_output.stderr);
        return Err(GitError::CommandFailed(format!(
            "Failed to restore target branch to {}: {}",
            target_tip, stderr
        ))
        .into());
    }
    Ok(())
}

fn backup_ref_name(target_branch: &str) -> String {
    let safe_branch: String = target_branch
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("refs/udf/merge-backups/{}/{}", safe_branch, timestamp)
}

/// Replay task commits onto the current target branch without rewriting target
/// commits or the original task branch.
///
/// The replay range is derived from the actual merge-base between the target
/// branch and task branch. Task metadata is useful diagnostics, but it can be
/// stale after a task branch is manually reset or rebased.
///
/// On conflict, aborts and restores the target branch tip before returning an
/// error. A backup ref is also written before any replay work starts.
pub fn rebase_branch(repo_path: &Path, branch_name: &str, based_on: &str) -> Result<()> {
    let target_branch = git_stdout(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let target_tip = git_stdout(repo_path, &["rev-parse", "HEAD"])?;
    let task_tip = git_stdout(repo_path, &["rev-parse", branch_name])?;
    // 同 squash：只看已跟踪文件。restore_target_tip 用的 reset --hard 不碰未跟踪
    // 文件，把它们算进来只会为一个 workflow/ 目录拒绝整次重放。
    let clean_status = git_stdout(
        repo_path,
        &["status", "--porcelain", "--untracked-files=no"],
    )?;
    if !clean_status.is_empty() {
        return Err(GitError::CommandFailed(format!(
            "Cannot replay task branch '{}': target worktree is not clean.\n{}",
            branch_name, clean_status
        ))
        .into());
    }

    if git_is_ancestor(repo_path, &task_tip, &target_tip)? {
        output::print_info(&format!(
            "Task branch '{}' is already contained in '{}'.",
            branch_name, target_branch
        ));
        return Ok(());
    }

    let based_on_commit = git_stdout(repo_path, &["rev-parse", "--verify", based_on])?;
    let actual_merge_base = git_stdout(repo_path, &["merge-base", &target_tip, &task_tip])?;
    let based_on_matches_topology = based_on_commit == actual_merge_base;
    if !based_on_matches_topology {
        let based_on_in_target = git_is_ancestor(repo_path, &based_on_commit, &target_tip)?;
        output::print_warning(&format!(
            "Task metadata base {} {} target branch '{}'; replaying from actual merge-base {}.",
            &based_on_commit[..8.min(based_on_commit.len())],
            if based_on_in_target {
                "differs from actual merge-base for"
            } else {
                "is not in"
            },
            target_branch,
            &actual_merge_base[..8.min(actual_merge_base.len())]
        ));
    }
    let replay_base = actual_merge_base.clone();

    let merge_commits = git_stdout(
        repo_path,
        &[
            "rev-list",
            "--merges",
            &format!("{}..{}", replay_base, task_tip),
        ],
    )?;
    if !merge_commits.is_empty() {
        return Err(GitError::CommandFailed(format!(
            "Cannot safely replay task branch '{}': replay range contains merge commit(s):\n{}",
            branch_name, merge_commits
        ))
        .into());
    }

    let commits = git_stdout(
        repo_path,
        &[
            "rev-list",
            "--reverse",
            &format!("{}..{}", replay_base, task_tip),
        ],
    )?;
    let commits: Vec<&str> = commits.lines().filter(|line| !line.is_empty()).collect();
    if commits.is_empty() {
        output::print_info(&format!(
            "No commits to replay from task branch '{}'.",
            branch_name
        ));
        return Ok(());
    }

    let backup_ref = backup_ref_name(&target_branch);
    let backup_output = Command::new("git")
        .args(["update-ref", &backup_ref, &target_tip])
        .current_dir(repo_path)
        .output()?;
    if !backup_output.status.success() {
        let stderr = String::from_utf8_lossy(&backup_output.stderr);
        return Err(GitError::CommandFailed(format!(
            "Failed to create merge backup ref '{}': {}",
            backup_ref, stderr
        ))
        .into());
    }

    output::print_info(&format!(
        "Replaying {} commit(s) from '{}' onto '{}' ({})",
        commits.len(),
        branch_name,
        target_branch,
        &target_tip[..8.min(target_tip.len())]
    ));
    output::print_info(&format!("  Backup ref: {}", backup_ref));
    output::print_info(&format!(
        "  Replay base: {}{}",
        &replay_base[..8.min(replay_base.len())],
        if based_on_matches_topology {
            " (task metadata)"
        } else {
            " (actual merge-base)"
        }
    ));

    for commit in commits {
        let output = Command::new("git")
            .args(["cherry-pick", "--empty=drop", commit])
            .current_dir(repo_path)
            .output()?;
        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            restore_target_tip(repo_path, &target_tip)?;
            return Err(GitError::MergeConflict(format!(
                "Replay conflict while cherry-picking {} from branch '{}'. Restored '{}'.\n{}\n{}",
                &commit[..8.min(commit.len())],
                branch_name,
                target_branch,
                stdout,
                stderr
            ))
            .into());
        }
    }

    output::print_success("Replay completed successfully");
    Ok(())
}

/// Squash merge: combines all commits from the branch into a single commit
/// Squash a task branch into the checked-out branch of `repo_path`.
///
/// `git merge --squash` stages the result without committing, so every failure
/// after that point has to put the index and working tree back. Two things used
/// to go wrong: conflicts were looked for on stderr while git announces them on
/// stdout, and a failing follow-up commit left the staged tree in place. Both
/// handed the user a main repo full of changes they never made.
pub fn squash_branch(repo_path: &Path, branch_name: &str) -> Result<()> {
    // Knowing the tracked tree was clean going in is what makes the undo below
    // safe: anything staged afterwards is ours to remove. Untracked files are
    // deliberately excluded -- `reset --hard` never touches them, and git
    // already refuses on its own if the merge would overwrite one. Counting
    // them would refuse a squash for a stray `workflow/` directory.
    let dirty = git_stdout(
        repo_path,
        &["status", "--porcelain", "--untracked-files=no"],
    )?;
    if !dirty.is_empty() {
        return Err(GitError::CommandFailed(format!(
            "Cannot squash task branch '{}': target worktree is not clean.\n{}",
            branch_name, dirty
        ))
        .into());
    }

    let output = Command::new("git")
        .args(["merge", "--squash", branch_name])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // git reports CONFLICT on stdout; aborting unconditionally is simpler
        // than deciding first, and a no-op when there is nothing to abort.
        let _ = Command::new("git")
            .args(["merge", "--abort"])
            .current_dir(repo_path)
            .output();
        let _ = Command::new("git")
            .args(["reset", "--hard", "HEAD"])
            .current_dir(repo_path)
            .output();
        if stdout.contains("CONFLICT") || stderr.contains("CONFLICT") {
            return Err(GitError::MergeConflict(format!(
                "Squash conflicts in branch '{}'. Merge aborted.",
                branch_name
            ))
            .into());
        }
        return Err(GitError::CommandFailed(format!("Squash failed: {}{}", stdout, stderr)).into());
    }

    let commit_msg = format!("Squash task branch '{}'", branch_name);
    let commit_output = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(repo_path)
        .output()?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        // The squash is already staged. Failing out without this leaves the
        // whole merge sitting in the user's index.
        let _ = Command::new("git")
            .args(["reset", "--hard", "HEAD"])
            .current_dir(repo_path)
            .output();
        return Err(GitError::CommandFailed(format!(
            "Failed to commit squash: {}{}",
            stdout, stderr
        ))
        .into());
    }

    Ok(())
}

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
