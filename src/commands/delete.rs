//! Delete command implementation

use crate::config::Config;
use crate::error::Result;
use crate::git;
use crate::host;
use crate::output;

pub fn run(task_id: &str, force: bool) -> Result<()> {
    let config = Config::load()?;

    // Get task host
    let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
    let meta = host::read_meta(&host_dir)?;

    if !force {
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

    output::print_success(&format!("Task '{}' deleted successfully!", task_id));

    Ok(())
}
