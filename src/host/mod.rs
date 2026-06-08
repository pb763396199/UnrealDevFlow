//! Host project management module

pub mod uproject;

use crate::error::{HostError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    pub id: String,
    pub name: String,
    pub branch: String,
    pub created: String,
    pub based_on: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_built: Option<String>,
}

pub fn create_host(host_dir: &Path, task_id: &str, engine_version: &str) -> Result<()> {
    if host_dir.exists() {
        return Err(HostError::AlreadyExists(host_dir.to_path_buf()).into());
    }

    fs::create_dir_all(host_dir)?;

    // Create .uproject
    let uproject_path = host_dir.join(format!("T-{}_Host.uproject", task_id));
    let uproject_content = uproject::generate(engine_version);
    fs::write(&uproject_path, uproject_content)?;

    // Note: Plugins directory will be created by git worktree add

    Ok(())
}

pub fn write_meta(host_dir: &Path, meta: &TaskMeta) -> Result<()> {
    let meta_path = host_dir.join(".udf-meta.json");
    let content = serde_json::to_string_pretty(meta)?;
    fs::write(&meta_path, content)?;
    Ok(())
}

pub fn read_meta(host_dir: &Path) -> Result<TaskMeta> {
    let meta_path = host_dir.join(".udf-meta.json");
    if !meta_path.exists() {
        return Err(HostError::InvalidMeta("File not found".to_string()).into());
    }
    let content = fs::read_to_string(&meta_path)?;
    let meta: TaskMeta = serde_json::from_str(&content)
        .map_err(|e| HostError::InvalidMeta(format!("JSON parse error: {}", e)))?;
    Ok(meta)
}

pub fn list_tasks(hosts_root: &Path) -> Result<Vec<TaskMeta>> {
    let mut tasks = Vec::new();

    if !hosts_root.exists() {
        return Ok(tasks);
    }

    for entry in fs::read_dir(hosts_root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && path.file_name().unwrap().to_string_lossy().starts_with("T-") {
            match read_meta(&path) {
                Ok(meta) => tasks.push(meta),
                Err(_) => continue,
            }
        }
    }

    Ok(tasks)
}

pub fn get_task_host(hosts_root: &Path, task_id: &str) -> Result<PathBuf> {
    let host_dir = hosts_root.join(format!("T-{}_Host", task_id));
    if !host_dir.exists() {
        return Err(HostError::NotExists(host_dir).into());
    }
    Ok(host_dir)
}

pub fn delete_host(host_dir: &Path) -> Result<()> {
    if host_dir.exists() {
        fs::remove_dir_all(host_dir)?;
    }
    Ok(())
}
