//! Delete command implementation

use crate::config::Config;
use crate::error::Result;
use crate::git;
use crate::host;
use crate::output;

pub fn run(task_id: &str, force: bool, skip_confirm: bool) -> Result<()> {
    let config = Config::load()?;

    // Get task host
    let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
    let meta = host::read_meta(&host_dir)?;

    if !force && !skip_confirm {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(&format!(
                "Delete task '{}' without merging?\n  This will permanently delete all changes in this task.",
                task_id
            ))
            .default(false)
            .interact()
            .map_err(|e| crate::error::UdfError::Other(format!("Dialog error: {}", e)))?;

        if !confirmed {
            output::print_info("Delete cancelled.");
            return Ok(());
        }
    }

    output::print_info(&format!("Deleting task '{}'...", task_id));

    // Open main repo
    let repo = git::open_repo(&config.plugin_path)?;

    // Try to delete worktree
    let worktree_path = host_dir.join("Plugins").join("AesWorld");
    let worktree_removed = if worktree_path.exists() {
        output::print_info("Removing worktree...");
        match git::worktree::remove(&worktree_path) {
            Ok(_) => true,
            Err(e) => {
                output::print_warning(&format!("Failed to remove worktree: {}", e));
                output::print_info("Attempting manual cleanup...");
                
                // Manual cleanup: prune worktrees
                if let Err(e) = git::worktree::prune(&config.plugin_path) {
                    output::print_warning(&format!("Failed to prune worktrees: {}", e));
                }
                
                false
            }
        }
    } else {
        true
    };

    // Try to delete branch
    output::print_info(&format!("Deleting branch '{}'...", meta.branch));
    let branch_deleted = match git::delete_branch(&repo, &meta.branch) {
        Ok(_) => true,
        Err(e) => {
            output::print_warning(&format!("Failed to delete branch: {}", e));
            false
        }
    };

    // Try to delete Host directory
    output::print_info("Deleting Host directory...");
    let host_deleted = match host::delete_host(&host_dir) {
        Ok(_) => true,
        Err(e) => {
            output::print_warning(&format!("Failed to delete Host directory: {}", e));
            false
        }
    };

    // Report results
    if worktree_removed && branch_deleted && host_deleted {
        output::print_success(&format!("Task '{}' deleted successfully!", task_id));
    } else {
        output::print_warning(&format!("Task '{}' partially deleted. Some resources may remain:", task_id));
        if !worktree_removed {
            output::print_warning(&format!("  - Worktree: {:?}", worktree_path));
            output::print_info("    Run: git worktree prune");
        }
        if !branch_deleted {
            output::print_warning(&format!("  - Branch: {}", meta.branch));
            output::print_info(&format!("    Run: git branch -D {}", meta.branch));
        }
        if !host_deleted {
            output::print_warning(&format!("  - Host directory: {:?}", host_dir));
            output::print_info(&format!("    Run: Remove-Item -Recurse -Force {:?}", host_dir));
        }
    }

    Ok(())
}
