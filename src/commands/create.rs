//! Create command implementation

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host::{self, TaskMeta};
use crate::output;
use chrono::Utc;
use std::path::PathBuf;

pub fn run(description: &str, custom_id: Option<String>, skip_confirm: bool) -> Result<()> {
    let config = Config::load()?;

    // Suggest task ID from description
    let task_id = match custom_id {
        Some(id) => id,
        None => suggest_task_id(description),
    };

    // Check if task already exists
    let host_dir = config.hosts_root.join(format!("T-{}_Host", task_id));
    if host_dir.exists() {
        return Err(UdfError::TaskAlreadyExists(task_id));
    }

    // Confirm with user (skip if --yes)
    let confirmed = if skip_confirm {
        true
    } else {
        dialoguer::Confirm::new()
            .with_prompt(&format!(
                "Create task '{}' with ID '{}'?\n  Based on current commit (no WIP included)",
                description, task_id
            ))
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?
    };

    if !confirmed {
        output::print_info("Task creation cancelled.");
        return Ok(());
    }

    // Open main repo
    let repo = git::open_repo(&config.plugin_path)?;
    let commit = git::get_current_commit(&repo)?;
    let branch_name = format!("task-{}", task_id);

    output::print_info(&format!("Creating worktree from commit {}...", &commit[..8]));

    // Create Host project first (creates the directory structure)
    let engine_version = config.engine_path
        .file_name()
        .map(|n| n.to_string_lossy().replace("UE_", ""))
        .unwrap_or_else(|| "5.5".to_string());

    host::create_host(&host_dir, &task_id, &engine_version)?;

    // Create worktree inside the Host's Plugins directory
    let worktree_path = host_dir.join("Plugins").join("AesWorld");
    git::worktree::add(&config.plugin_path, &worktree_path, &commit, &branch_name)?;

    // Create .udf-meta.json
    let meta = TaskMeta {
        id: task_id.clone(),
        name: description.to_string(),
        branch: branch_name.clone(),
        created: Utc::now().to_rfc3339(),
        based_on: commit.clone(),
        status: "active".to_string(),
        last_built: None,
    };
    host::write_meta(&host_dir, &meta)?;

    output::print_success(&format!("Task '{}' created successfully!", task_id));
    output::print_info(&format!("  Host: {:?}", host_dir));
    output::print_info(&format!("  Branch: {}", branch_name));
    output::print_info(&format!("  Based on: {}", &commit[..8]));
    output::print_info(&format!("  Build: unrealdevflow build {}", task_id));
    output::print_info(&format!("  Switch: unrealdevflow switch {}", task_id));

    Ok(())
}

fn suggest_task_id(description: &str) -> String {
    // Simple suggestion: take first few words, lowercase, kebab-case
    let words: Vec<&str> = description
        .split_whitespace()
        .take(3)
        .collect();

    let id: String = words
        .iter()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("-");

    // Remove non-alphanumeric except hyphens
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}
