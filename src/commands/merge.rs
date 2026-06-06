//! Merge command implementation

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host;
use crate::output;
use std::fs;

pub fn run(task_id: &str, force: bool, _skip_confirm: bool) -> Result<()> {
    let config = Config::load()?;

    // Get task host
    let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
    let meta = host::read_meta(&host_dir)?;

    output::print_info(&format!("Merging task '{}' into main repo...", task_id));

    // Open main repo
    let repo = git::open_repo(&config.plugin_path)?;

    // Merge branch
    match git::merge_branch(&repo, &meta.branch) {
        Ok(_) => {
            output::print_success(&format!("Branch '{}' merged successfully", meta.branch));
        }
        Err(e) => {
            if force {
                output::print_warning(&format!("Merge failed: {}. Forcing deletion...", e));
            } else {
                return Err(e);
            }
        }
    }

    // Delete worktree
    let worktree_path = host_dir.join("Plugins").join("AesWorld");
    if worktree_path.exists() {
        output::print_info("Removing worktree...");
        git::worktree::remove(&worktree_path)?;
    }

    // Delete branch
    output::print_info(&format!("Deleting branch '{}'...", meta.branch));
    git::delete_branch(&repo, &meta.branch)?;

    // Delete Host directory
    output::print_info("Deleting Host directory...");
    host::delete_host(&host_dir)?;

    output::print_success(&format!("Task '{}' merged and cleaned up successfully!", task_id));

    Ok(())
}
