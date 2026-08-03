//! Host project management module

pub mod uproject;

use crate::config::{Config, WorkspaceConfig};
use crate::error::{HostError, Result, UdfError};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const CURRENT_SCHEMA_VERSION: u32 = 3;
const META_FILE_NAME: &str = ".udf-meta.json";
const META_BACKUP_FILE_NAME: &str = ".udf-meta.json.bak";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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

/// Locate a Host's `.uproject`.
///
/// Prefers the canonical `T_<task>_Host.uproject` name. Hosts created before
/// that naming rule keep a single legacy file, which is accepted as long as it
/// is unambiguous — guessing between several would risk building the wrong
/// project.
pub fn resolve_host_uproject(host_dir: &Path, task_id: &str) -> Result<PathBuf> {
    let canonical = host_dir.join(format!("{}.uproject", task_project_name(task_id)));
    if canonical.is_file() {
        return Ok(canonical);
    }

    let mut candidates = fs::read_dir(host_dir)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("uproject"))
        })
        .collect::<Vec<_>>();
    candidates.sort();

    match candidates.as_slice() {
        [legacy] => Ok(legacy.clone()),
        [] => Err(UdfError::Other(format!(
            "任务 Host 缺少 .uproject：{}",
            host_dir.display()
        ))),
        _ => Err(UdfError::Other(format!(
            "任务 Host 包含多个非规范 .uproject，无法安全选择：{}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

pub fn write_meta(host_dir: &Path, meta: &TaskMeta) -> Result<()> {
    let meta_path = host_dir.join(META_FILE_NAME);
    let backup_path = host_dir.join(META_BACKUP_FILE_NAME);
    let content = serde_json::to_vec_pretty(meta)?;

    // Only a parseable primary file is allowed to replace the last-known-good
    // backup. An interrupted/corrupt primary must never poison recovery.
    if let Ok(previous_content) = fs::read(&meta_path)
        && parse_meta(&previous_content).is_ok()
    {
        write_file_atomically(&backup_path, &previous_content)?;
    }

    write_file_atomically(&meta_path, &content)
}

/// Read `.udf-meta.json` from a Host directory, auto-migrating v1 metadata to
/// v2 in-memory (the caller decides whether to persist the upgrade).
///
/// A primary file that is empty or unparseable falls back to the `.bak` written
/// by the previous successful `write_meta`, and the recovered content is put
/// back in place so the next read no longer needs the backup.
pub fn read_meta(host_dir: &Path) -> Result<TaskMeta> {
    let meta_path = host_dir.join(META_FILE_NAME);
    let backup_path = host_dir.join(META_BACKUP_FILE_NAME);

    match read_meta_file(&meta_path) {
        Ok(meta) => Ok(meta),
        Err(primary_error) => match fs::read(&backup_path) {
            Ok(backup_content) => match parse_meta(&backup_content) {
                Ok(mut meta) => {
                    write_file_atomically(&meta_path, &backup_content).map_err(|error| {
                        HostError::InvalidMeta(format!(
                            "主文件无效（{primary_error}），备份有效但自动恢复失败：{error}；\
                             可从 '{}' 手动恢复到 '{}'",
                            backup_path.display(),
                            meta_path.display()
                        ))
                    })?;
                    crate::migration::migrate_in_place(&mut meta);
                    Ok(meta)
                }
                Err(backup_error) => Err(HostError::InvalidMeta(format!(
                    "主文件无效（{primary_error}），备份也无效（{backup_error}）；\
                     请从任务记录或版本控制恢复 '{}'",
                    meta_path.display()
                ))
                .into()),
            },
            Err(backup_error) => Err(HostError::InvalidMeta(format!(
                "主文件无效（{primary_error}），且无法读取备份 '{}'（{backup_error}）；\
                 请从任务记录或版本控制恢复元数据",
                backup_path.display()
            ))
            .into()),
        },
    }
}

fn read_meta_file(path: &Path) -> std::result::Result<TaskMeta, String> {
    let content =
        fs::read(path).map_err(|error| format!("读取 '{}' 失败：{error}", path.display()))?;
    let mut meta = parse_meta(&content)?;
    crate::migration::migrate_in_place(&mut meta);
    Ok(meta)
}

fn parse_meta(content: &[u8]) -> std::result::Result<TaskMeta, String> {
    if content.is_empty() {
        return Err("文件为空（可能是写入进程中断）".to_string());
    }
    serde_json::from_slice(content).map_err(|error| format!("JSON 解析失败：{error}"))
}

/// Write via a uniquely named temp file plus rename, so a crash mid-write
/// leaves either the old file or the new one, never a truncated one.
fn write_file_atomically(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        UdfError::Other(format!("无法确定元数据文件 '{}' 的父目录", path.display()))
    })?;
    fs::create_dir_all(parent)?;

    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("udf-meta");
    let temp_path = parent.join(format!(
        ".{file_name}.tmp.{}.{}",
        std::process::id(),
        sequence
    ));

    let result = (|| -> Result<()> {
        let mut temp_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;
        temp_file.write_all(content)?;
        temp_file.flush()?;
        temp_file.sync_all()?;
        drop(temp_file);

        replace_file(&temp_path, path)?;
        sync_parent_directory(parent)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn sync_parent_directory(_parent: &Path) -> std::io::Result<()> {
    // Windows does not support opening directories through std::fs::File.
    // The temporary file itself has already been flushed and sync_all'ed.
    Ok(())
}

#[cfg(not(windows))]
fn sync_parent_directory(parent: &Path) -> std::io::Result<()> {
    fs::File::open(parent)?.sync_all()
}

/// A Host directory that looks like a task but whose metadata cannot be read.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamagedTask {
    pub host_dir: PathBuf,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskInventory {
    pub tasks: Vec<TaskMeta>,
    pub damaged_tasks: Vec<DamagedTask>,
}

pub fn list_tasks(hosts_root: &Path) -> Result<Vec<TaskMeta>> {
    Ok(list_task_inventory(hosts_root)?.tasks)
}

/// Like `list_tasks`, but keeps unreadable tasks instead of silently dropping
/// them: a task that disappears from `list` looks deleted, which it is not.
pub fn list_task_inventory(hosts_root: &Path) -> Result<TaskInventory> {
    let mut inventory = TaskInventory::default();

    if !hosts_root.exists() {
        return Ok(inventory);
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
                Ok(meta) => inventory.tasks.push(meta),
                Err(error) => inventory.damaged_tasks.push(DamagedTask {
                    host_dir: path.clone(),
                    error: error.to_string(),
                }),
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
                        Ok(meta) => inventory.tasks.push(meta),
                        Err(error) => inventory.damaged_tasks.push(DamagedTask {
                            host_dir: host_path,
                            error: error.to_string(),
                        }),
                    }
                }
            }
        }
    }

    Ok(inventory)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn task_meta(name: &str) -> TaskMeta {
        TaskMeta {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: "atomic-meta".to_string(),
            name: name.to_string(),
            branch: "task/test/atomic-meta".to_string(),
            created: "2026-07-15T00:00:00Z".to_string(),
            based_on: "0123456789abcdef".to_string(),
            status: "active".to_string(),
            prompt: None,
            last_built: None,
            build_pid: None,
            build_log: None,
            console_log: None,
            build_status: None,
            primary_plugins: Vec::new(),
            dependency_plugins: Vec::new(),
            workspace: Some("test-workspace".to_string()),
            task_uid: Some("test-workspace/atomic-meta".to_string()),
            context: None,
        }
    }

    #[test]
    fn write_meta_atomically_preserves_last_valid_primary_as_backup() {
        let host = tempfile::tempdir().unwrap();
        write_meta(host.path(), &task_meta("first")).unwrap();
        write_meta(host.path(), &task_meta("second")).unwrap();

        let current = read_meta_file(&host.path().join(META_FILE_NAME)).unwrap();
        let backup = read_meta_file(&host.path().join(META_BACKUP_FILE_NAME)).unwrap();
        assert_eq!(current.name, "second");
        assert_eq!(backup.name, "first");

        let temporary_files = fs::read_dir(host.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
            .count();
        assert_eq!(temporary_files, 0);
    }

    #[test]
    fn read_meta_recovers_empty_primary_from_last_valid_backup() {
        let host = tempfile::tempdir().unwrap();
        write_meta(host.path(), &task_meta("recoverable")).unwrap();
        write_meta(host.path(), &task_meta("latest")).unwrap();
        fs::write(host.path().join(META_FILE_NAME), []).unwrap();

        let recovered = read_meta(host.path()).unwrap();
        assert_eq!(recovered.name, "recoverable");

        let restored_primary = read_meta_file(&host.path().join(META_FILE_NAME)).unwrap();
        assert_eq!(restored_primary.name, "recoverable");
        let backup = read_meta_file(&host.path().join(META_BACKUP_FILE_NAME)).unwrap();
        assert_eq!(backup.name, "recoverable");
    }

    #[test]
    fn read_meta_recovers_malformed_primary_from_last_valid_backup() {
        let host = tempfile::tempdir().unwrap();
        write_meta(host.path(), &task_meta("recoverable")).unwrap();
        write_meta(host.path(), &task_meta("latest")).unwrap();
        fs::write(host.path().join(META_FILE_NAME), b"{truncated").unwrap();

        let recovered = read_meta(host.path()).unwrap();
        assert_eq!(recovered.name, "recoverable");
        assert_eq!(read_meta(host.path()).unwrap().name, "recoverable");
    }

    #[test]
    fn write_meta_does_not_replace_valid_backup_with_corrupt_primary() {
        let host = tempfile::tempdir().unwrap();
        write_meta(host.path(), &task_meta("last-known-good")).unwrap();
        write_meta(host.path(), &task_meta("interrupted-update")).unwrap();
        fs::write(host.path().join(META_FILE_NAME), b"{truncated").unwrap();

        write_meta(host.path(), &task_meta("new-valid-state")).unwrap();

        let current = read_meta_file(&host.path().join(META_FILE_NAME)).unwrap();
        let backup = read_meta_file(&host.path().join(META_BACKUP_FILE_NAME)).unwrap();
        assert_eq!(current.name, "new-valid-state");
        assert_eq!(backup.name, "last-known-good");
    }

    #[test]
    fn read_meta_reports_both_primary_and_backup_failures() {
        let host = tempfile::tempdir().unwrap();
        fs::write(host.path().join(META_FILE_NAME), []).unwrap();
        fs::write(host.path().join(META_BACKUP_FILE_NAME), b"not-json").unwrap();

        let error = read_meta(host.path()).unwrap_err().to_string();
        assert!(error.contains("主文件无效"), "{error}");
        assert!(error.contains("备份也无效"), "{error}");
        assert!(error.contains("文件为空"), "{error}");
        assert!(error.contains("JSON 解析失败"), "{error}");
    }

    #[test]
    fn task_inventory_reports_corrupt_metadata_without_hiding_healthy_tasks() {
        let root = tempfile::tempdir().unwrap();
        let healthy = root.path().join("T-healthy_Host");
        fs::create_dir_all(&healthy).unwrap();
        write_meta(&healthy, &task_meta("healthy")).unwrap();
        let host = root.path().join("T-corrupt_Host");
        fs::create_dir_all(&host).unwrap();
        fs::write(host.join(META_FILE_NAME), b"{broken").unwrap();

        let inventory = list_task_inventory(root.path()).unwrap();
        assert_eq!(inventory.tasks.len(), 1);
        assert_eq!(inventory.tasks[0].name, "healthy");
        assert_eq!(inventory.damaged_tasks.len(), 1);
        assert!(
            inventory.damaged_tasks[0]
                .host_dir
                .ends_with("T-corrupt_Host")
        );
        assert!(inventory.damaged_tasks[0].error.contains("JSON 解析失败"));
    }

    #[test]
    fn resolve_host_uproject_accepts_legacy_host_filename() {
        let host = tempfile::tempdir().unwrap();
        let legacy = host.path().join("T-hier-anchor-rebase_Host.uproject");
        fs::write(&legacy, "{}").unwrap();

        let resolved = resolve_host_uproject(host.path(), "hier-anchor-rebase").unwrap();

        assert_eq!(resolved, legacy);
    }

    #[test]
    fn resolve_host_uproject_prefers_canonical_filename() {
        let host = tempfile::tempdir().unwrap();
        let canonical = host
            .path()
            .join(format!("{}.uproject", task_project_name("current-task")));
        fs::write(&canonical, "{}").unwrap();
        fs::write(host.path().join("LegacyHost.uproject"), "{}").unwrap();

        let resolved = resolve_host_uproject(host.path(), "current-task").unwrap();

        assert_eq!(resolved, canonical);
    }

    #[test]
    fn resolve_host_uproject_rejects_ambiguous_legacy_filenames() {
        let host = tempfile::tempdir().unwrap();
        fs::write(host.path().join("First.uproject"), "{}").unwrap();
        fs::write(host.path().join("Second.uproject"), "{}").unwrap();

        let error = resolve_host_uproject(host.path(), "legacy-task")
            .unwrap_err()
            .to_string();

        assert!(error.contains("多个非规范 .uproject"), "{error}");
        assert!(error.contains("First.uproject"), "{error}");
        assert!(error.contains("Second.uproject"), "{error}");
    }
}
