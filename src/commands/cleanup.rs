//! Cleanup command implementation (v2 multi-plugin)
//!
//! Manually cleanup worktrees and branches after merge verification.

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host;
use crate::host::PrimaryPlugin;
use crate::output;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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

fn expected_branch_names(config: &Config, task_ref: &str) -> Vec<String> {
    let (workspace, task_id) = host::parse_task_ref(task_ref);
    let mut branches = BTreeSet::new();
    branches.insert(format!("task-{}", task_id));

    if let Some(workspace) = workspace {
        branches.insert(format!(
            "task/{}/{}",
            crate::config::sanitize_workspace_name(&workspace),
            task_id
        ));
    } else {
        for workspace in config.workspace_names() {
            branches.insert(format!("task/{}/{}", workspace, task_id));
        }
    }

    branches.into_iter().collect()
}

fn candidate_plugin_roots(config: &Config, task_ref: &str) -> Vec<PathBuf> {
    let (workspace, _) = host::parse_task_ref(task_ref);
    let mut roots = BTreeSet::new();

    if let Some(workspace) = workspace {
        if let Ok((_, ws)) = config.resolve_workspace(Some(&workspace))
            && let Some(root) = ws.effective_plugins_root()
        {
            roots.insert(root);
        }
    } else {
        if let Some(root) = config.effective_plugins_root() {
            roots.insert(root);
        }
        for ws in config.workspaces.values() {
            if let Some(root) = ws.effective_plugins_root() {
                roots.insert(root);
            }
        }
    }

    roots.into_iter().collect()
}

fn local_branch_exists(repo_path: &Path, branch: &str) -> bool {
    git::open_repo(repo_path)
        .and_then(|repo| {
            repo.find_branch(branch, git2::BranchType::Local)
                .map(|_| ())
                .map_err(Into::into)
        })
        .is_ok()
}

fn remove_worktree_and_branch(
    host_dir: &Path,
    primary: &PrimaryPlugin,
    all_worktrees_removed: &mut bool,
    all_branches_deleted: &mut bool,
) {
    let worktree_abs = host_dir.join(&primary.worktree);
    if worktree_abs.exists() {
        output::print_info(&format!(
            "Removing worktree for '{}': {:?}",
            primary.name, worktree_abs
        ));
        if let Err(e) = git::worktree::remove(&primary.source_repo, &worktree_abs) {
            output::print_warning(&format!("Failed to remove worktree: {}", e));
            *all_worktrees_removed = false;
        }
    }

    if !primary.source_repo.as_os_str().is_empty() {
        if let Err(e) = git::worktree::prune(&primary.source_repo) {
            output::print_warning(&format!("Failed to prune worktrees: {}", e));
        }

        output::print_info(&format!(
            "Deleting branch '{}' in {:?}",
            primary.branch, primary.source_repo
        ));
        if let Err(e) = git::delete_branch_safe(&primary.source_repo, &primary.branch) {
            output::print_warning(&format!("Failed to delete branch: {}", e));
            if let Err(prune_error) = git::worktree::prune(&primary.source_repo) {
                output::print_warning(&format!("Failed to prune worktrees: {}", prune_error));
            }
            if let Err(retry_error) = git::delete_branch_safe(&primary.source_repo, &primary.branch)
            {
                output::print_warning(&format!(
                    "Failed to delete branch after prune: {}",
                    retry_error
                ));
                *all_branches_deleted = false;
            }
        }
    }
}

fn cleanup_orphaned_branches(
    config: &Config,
    task_ref: &str,
    force: bool,
    skip_confirm: bool,
) -> Result<()> {
    let branches = expected_branch_names(config, task_ref);
    let roots = candidate_plugin_roots(config, task_ref);
    let mut matches: Vec<(PathBuf, String)> = Vec::new();

    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            let repo_path = entry.path();
            if !repo_path.is_dir() {
                continue;
            }
            if git::open_repo(&repo_path).is_err() {
                continue;
            }
            for branch in &branches {
                if local_branch_exists(&repo_path, branch) {
                    matches.push((repo_path.clone(), branch.clone()));
                }
            }
        }
    }

    println!();
    output::print_warning(&format!(
        "Host for task '{}' is missing. Attempting orphan branch cleanup.",
        task_ref
    ));
    if matches.is_empty() {
        output::print_info("No matching local task branches found.");
        return Ok(());
    }

    for (repo_path, branch) in &matches {
        output::print_info(&format!(
            "  Residual branch '{}' in {:?}",
            branch, repo_path
        ));
    }

    if !(force || skip_confirm) {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Delete {} residual local task branch(es) for '{}'?",
                matches.len(),
                task_ref
            ))
            .default(false)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
        if !confirmed {
            output::print_info("Orphan cleanup cancelled.");
            return Ok(());
        }
    }

    let mut all_deleted = true;
    for (repo_path, branch) in &matches {
        if let Err(e) = git::worktree::prune(repo_path) {
            output::print_warning(&format!(
                "Failed to prune worktrees in {:?}: {}",
                repo_path, e
            ));
        }
        output::print_info(&format!("Deleting residual branch '{}'...", branch));
        if let Err(e) = git::delete_branch_safe(repo_path, branch) {
            output::print_warning(&format!("Failed to delete residual branch: {}", e));
            all_deleted = false;
        }
    }

    if all_deleted {
        output::print_success(&format!("Task '{}' orphan branches cleaned up.", task_ref));
    } else {
        output::print_warning(&format!(
            "Task '{}' orphan cleanup incomplete. Some branches may remain.",
            task_ref
        ));
    }
    Ok(())
}

pub fn run(task_id: &str, force: bool, skip_confirm: bool) -> Result<()> {
    let config = Config::load()?;

    let (host_dir, mut meta, _task_context) = match host::resolve_task(&config, task_id) {
        Ok(value) => value,
        Err(UdfError::Host(crate::error::HostError::NotExists(_))) => {
            return cleanup_orphaned_branches(&config, task_id, force, skip_confirm);
        }
        Err(e) => return Err(e),
    };
    crate::migration::backfill_source_repo(&mut meta, &config);

    let task_id_only = meta.id.clone();
    let expected_branches = vec![
        meta.branch.clone(),
        format!("task-{}", task_id_only),
        format!(
            "task/{}/{}",
            meta.workspace
                .as_deref()
                .unwrap_or(crate::config::DEFAULT_WORKSPACE),
            task_id_only
        ),
    ];
    for p in &meta.primary_plugins {
        if !expected_branches.contains(&p.branch) {
            return Err(UdfError::Other(format!(
                "Branch name '{}' for plugin '{}' does not match expected {:?}. Aborting cleanup.",
                p.branch, p.name, expected_branches
            )));
        }
    }

    println!();
    output::print_info(&format!("Cleanup task '{}'", task_id));
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!(
        "  Primary plugins ({}): {}",
        meta.primary_plugins.len(),
        meta.primary_plugins
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if !meta.dependency_plugins.is_empty() {
        output::print_info(&format!(
            "  Dependency junctions ({}): {}",
            meta.dependency_plugins.len(),
            meta.dependency_plugins
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    output::print_info(&format!("  Host dir: {:?}", host_dir));
    println!("─────────────────────────────────────────────────────────────");

    if !(force || skip_confirm) {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Cleanup task '{}'? This will delete worktrees, branches, junctions and Host dir.",
                task_id
            ))
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
        if !confirmed {
            output::print_info("Cleanup cancelled.");
            return Ok(());
        }
    }

    output::print_info(&format!("Cleaning up task '{}'...", task_id));

    // Step 1: Remove dependency junctions explicitly so we don't accidentally
    // recurse into the main repo when we later delete the Host dir.
    for dep in &meta.dependency_plugins {
        if let Some(rel) = &dep.junction {
            let junction_abs = host_dir.join(rel);
            if junction_abs.exists()
                && let Err(e) = crate::junction::delete(&junction_abs)
            {
                output::print_warning(&format!(
                    "Failed to remove junction for '{}': {}",
                    dep.name, e
                ));
            }
        }
    }

    // Step 2: Remove primary worktrees before deleting the Host directory.
    // Git refuses to delete a branch while any worktree still owns it.
    let mut all_worktrees_removed = true;
    let mut all_branches_deleted = true;
    for primary in &meta.primary_plugins {
        remove_worktree_and_branch(
            &host_dir,
            primary,
            &mut all_worktrees_removed,
            &mut all_branches_deleted,
        );
    }

    // Step 3: Delete Host directory (with retry for Windows file locking).
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

    println!();
    if host_deleted && all_worktrees_removed && all_branches_deleted {
        output::print_success(&format!("Task '{}' cleaned up successfully!", task_id));
    } else {
        output::print_warning(&format!(
            "Task '{}' cleanup incomplete. Some resources may remain.",
            task_id
        ));
    }

    Ok(())
}
