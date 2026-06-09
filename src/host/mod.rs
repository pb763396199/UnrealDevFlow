//! Host project management module

pub mod uproject;

use crate::error::{HostError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Build status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStatus {
    pub state: String, // "building" | "success" | "failed" | "unknown"
    pub started: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub mutex_mode: String, // "WaitMutex" | "NoMutex"
}

/// A primary (writable) plugin participating in a task.
///
/// Each primary plugin gets its own Git worktree + branch, and participates in
/// the merge workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryPlugin {
    pub name: String,
    pub source_repo: PathBuf,
    /// Relative to the Host directory, e.g. `Plugins/AesWorld`.
    pub worktree: PathBuf,
    pub branch: String,
    pub based_on: String,
}

/// Source of a dependency plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencySource {
    Engine,
    Project,
}

/// A dependency (read-only) plugin participating in a task.
///
/// Engine dependencies are automatically enabled via `.uproject` and need no
/// junction. Project dependencies get a Junction in the Host plugins folder
/// pointing at the main repository checkout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyPlugin {
    pub name: String,
    pub source: DependencySource,
    pub source_path: PathBuf,
    /// Relative to the Host directory; only present for `Project` source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub junction: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    /// Schema version. Absent in v1 metadata; v2+ writes this explicitly.
    #[serde(default)]
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    /// v1 legacy single-branch field. Still emitted for tooling that reads it
    /// (matches the first primary plugin's branch in v2).
    pub branch: String,
    pub created: String,
    /// v1 legacy single-commit field (matches the first primary plugin).
    pub based_on: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_built: Option<String>,
    // Build tracking fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_log: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub console_log: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_status: Option<BuildStatus>,
    /// v2 primary plugins. For v1 metadata loaded via migration, contains a
    /// single entry derived from the legacy fields.
    #[serde(default)]
    pub primary_plugins: Vec<PrimaryPlugin>,
    /// v2 dependency plugins. Always empty in migrated v1 metadata.
    #[serde(default)]
    pub dependency_plugins: Vec<DependencyPlugin>,
}

impl TaskMeta {
    /// Iterator over absolute worktree paths for all primary plugins.
    pub fn primary_worktree_paths(&self, host_dir: &Path) -> Vec<PathBuf> {
        self.primary_plugins
            .iter()
            .map(|p| host_dir.join(&p.worktree))
            .collect()
    }
}

pub fn create_host(host_dir: &Path, task_id: &str, engine_version: &str) -> Result<()> {
    create_host_with_plugins(host_dir, task_id, engine_version, &[], &[])
}

pub fn create_host_with_plugins(
    host_dir: &Path,
    task_id: &str,
    engine_version: &str,
    primary_names: &[String],
    project_dependency_names: &[String],
) -> Result<()> {
    if host_dir.exists() {
        return Err(HostError::AlreadyExists(host_dir.to_path_buf()).into());
    }

    fs::create_dir_all(host_dir)?;

    // Create .uproject with all primary + project-dependency plugins enabled.
    let uproject_path = host_dir.join(format!("T-{}_Host.uproject", task_id));
    let mut enabled = Vec::new();
    enabled.extend(primary_names.iter().cloned());
    enabled.extend(project_dependency_names.iter().cloned());
    if enabled.is_empty() {
        // Backward-compatible fallback used by tests and v1 callers.
        enabled.push("AesWorld".to_string());
    }
    let uproject_content = uproject::generate(engine_version, &enabled);
    fs::write(&uproject_path, uproject_content)?;

    Ok(())
}

pub fn write_meta(host_dir: &Path, meta: &TaskMeta) -> Result<()> {
    let meta_path = host_dir.join(".udf-meta.json");
    let content = serde_json::to_string_pretty(meta)?;
    fs::write(&meta_path, content)?;
    Ok(())
}

/// Read `.udf-meta.json` from a Host directory, auto-migrating v1 metadata to
/// v2 in-memory (the caller decides whether to persist the upgrade).
pub fn read_meta(host_dir: &Path) -> Result<TaskMeta> {
    let meta_path = host_dir.join(".udf-meta.json");
    if !meta_path.exists() {
        return Err(HostError::InvalidMeta("File not found".to_string()).into());
    }
    let content = fs::read_to_string(&meta_path)?;
    let mut meta: TaskMeta = serde_json::from_str(&content)
        .map_err(|e| HostError::InvalidMeta(format!("JSON parse error: {}", e)))?;
    crate::migration::migrate_in_place(&mut meta);
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
