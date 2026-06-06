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
