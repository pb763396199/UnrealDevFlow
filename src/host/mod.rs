//! Host project management module

pub mod uproject;

use crate::config::{Config, WorkspaceConfig};
use crate::error::{HostError, Result, UdfError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CURRENT_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub workspace: String,
    pub hosts_root: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_path: Option<PathBuf>,
    pub default_project: PathBuf,
    pub engine_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugins_root: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub plugin_overrides: std::collections::HashMap<String, PathBuf>,
}

impl TaskContext {
    pub fn from_workspace(name: &str, workspace: &WorkspaceConfig) -> Self {
        Self {
            workspace: name.to_string(),
            hosts_root: workspace.hosts_root.clone(),
            plugin_path: workspace.plugin_path.clone(),
            default_project: workspace.default_project.clone(),
            engine_path: workspace.engine_path.clone(),
            plugins_root: workspace.plugins_root.clone(),
            plugin_overrides: workspace.plugin_overrides.clone(),
        }
    }

    pub fn as_workspace_config(&self) -> WorkspaceConfig {
        WorkspaceConfig {
            hosts_root: self.hosts_root.clone(),
            plugin_path: self.plugin_path.clone(),
            default_project: self.default_project.clone(),
            engine_path: self.engine_path.clone(),
            plugins_root: self.plugins_root.clone(),
            plugin_overrides: self.plugin_overrides.clone(),
        }
    }
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_uid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<TaskContext>,
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
    let uproject_path = host_dir.join(format!("{}.uproject", task_project_name(task_id)));
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

pub fn task_project_name(task_id: &str) -> String {
    let sanitized = task_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("T_{}_Host", sanitized)
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

        if path.is_dir()
            && path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("T-")
        {
            match read_meta(&path) {
                Ok(meta) => tasks.push(meta),
                Err(_) => continue,
            }
        }

        if path.is_dir()
            && path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("W-")
        {
            for host_entry in fs::read_dir(&path)? {
                let host_entry = host_entry?;
                let host_path = host_entry.path();
                if host_path.is_dir()
                    && host_path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("T-")
                {
                    match read_meta(&host_path) {
                        Ok(meta) => tasks.push(meta),
                        Err(_) => continue,
                    }
                }
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

pub fn workspace_host_root(hosts_root: &Path, workspace: &str) -> PathBuf {
    hosts_root.join(format!(
        "W-{}",
        crate::config::sanitize_workspace_name(workspace)
    ))
}

pub fn task_host_dir(hosts_root: &Path, workspace: Option<&str>, task_id: &str) -> PathBuf {
    match workspace {
        Some(name) => workspace_host_root(hosts_root, name).join(format!("T-{}_Host", task_id)),
        None => hosts_root.join(format!("T-{}_Host", task_id)),
    }
}

pub fn parse_task_ref(task_ref: &str) -> (Option<String>, String) {
    if let Some((workspace, id)) = task_ref.split_once('/') {
        (Some(workspace.to_string()), id.to_string())
    } else {
        (None, task_ref.to_string())
    }
}

pub fn resolve_task(config: &Config, task_ref: &str) -> Result<(PathBuf, TaskMeta, TaskContext)> {
    let (workspace_name, task_id) = parse_task_ref(task_ref);
    let mut candidates: Vec<(PathBuf, String, WorkspaceConfig)> = Vec::new();

    if let Some(name) = workspace_name {
        let (resolved_name, workspace) = config.resolve_workspace(Some(&name))?;
        candidates.push((
            task_host_dir(&workspace.hosts_root, Some(&resolved_name), &task_id),
            resolved_name,
            workspace,
        ));
    } else {
        let legacy_host = task_host_dir(&config.hosts_root, None, &task_id);
        if legacy_host.exists() {
            candidates.push((
                legacy_host,
                crate::config::DEFAULT_WORKSPACE.to_string(),
                config.legacy_workspace(),
            ));
        }

        for (name, workspace) in &config.workspaces {
            let host = task_host_dir(&workspace.hosts_root, Some(name), &task_id);
            if host.exists() {
                candidates.push((host, name.clone(), workspace.clone()));
            }
        }
    }

    let existing: Vec<_> = candidates
        .into_iter()
        .filter(|(host_dir, _, _)| host_dir.exists())
        .collect();

    if existing.is_empty() {
        return Err(HostError::NotExists(PathBuf::from(task_ref)).into());
    }
    if existing.len() > 1 {
        let names = existing
            .iter()
            .map(|(_, name, _)| format!("{}/{}", name, task_id))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(UdfError::Other(format!(
            "任务 '{}' 在多个 workspace 中存在，请使用完整 task ref：{}",
            task_id, names
        )));
    }

    let (host_dir, fallback_workspace_name, fallback_workspace) =
        existing.into_iter().next().unwrap();
    let meta = read_meta(&host_dir)?;
    let context = meta.context.clone().unwrap_or_else(|| {
        TaskContext::from_workspace(&fallback_workspace_name, &fallback_workspace)
    });
    Ok((host_dir, meta, context))
}

pub fn delete_host(host_dir: &Path) -> Result<()> {
    if host_dir.exists() {
        fs::remove_dir_all(host_dir)?;
    }
    Ok(())
}
