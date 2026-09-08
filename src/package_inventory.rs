//! Read-only inventory of package data owned (or potentially owned) by UDF.
//!
//! The inventory is intentionally conservative.  It walks only fixed UDF
//! roots and paths already recorded by an execution.  User output directories
//! are never inferred from a name alone; a manifest or an explicit execution
//! reference is required before they are shown as final output.

use crate::package_cache;
use crate::package_storage::tree_bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{ProcessesToUpdate, System};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageInventoryItem {
    pub category: String,
    pub path: PathBuf,
    pub bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    pub owner_kind: String,
    pub owner_id: String,
    pub execution_ids: Vec<String>,
    pub state: String,
    pub active: bool,
    pub protection: Option<String>,
    pub reclaim_reason: Option<String>,
    pub confidence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageInventoryReport {
    pub items: Vec<PackageInventoryItem>,
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub protected_bytes: u64,
    pub unknown_bytes: u64,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone)]
struct ExecutionRef {
    id: String,
    workspace: String,
    task_ref: Option<String>,
    state: String,
    target_kind: String,
    paths: Vec<PathBuf>,
    outputs: Vec<PathBuf>,
    logs: Vec<PathBuf>,
    commands: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct InventoryFilter {
    pub task_ref: Option<String>,
    pub workspace: Option<String>,
}

impl InventoryFilter {
    fn matches(&self, item: &PackageInventoryItem) -> bool {
        if let Some(task) = self.task_ref.as_deref() {
            if item.owner_kind == "unknown" {
                return false;
            }
            if item.owner_kind == "task" && item.owner_id != task {
                return false;
            }
            if item.owner_kind == "execution"
                && !item.execution_ids.iter().any(|id| id == task)
                && item.owner_id != task
            {
                return false;
            }
        }
        if let Some(workspace) = self.workspace.as_deref() {
            if item.owner_kind == "unknown" {
                return false;
            }
            if item.owner_kind == "workspace" && item.owner_id != workspace {
                return false;
            }
            if item.owner_kind == "task" && !item.owner_id.starts_with(&format!("{workspace}/")) {
                return false;
            }
            // Execution owner ids are execution IDs; workspace matching is
            // handled while parsing records and encoded in the item owner.
            if item.owner_kind == "execution"
                && !item.execution_ids.iter().any(|id| id == workspace)
                && item.owner_id != workspace
            {
                return false;
            }
        }
        true
    }
}

fn read_json_value(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn string_field(value: &Value, name: &str) -> Option<String> {
    value.get(name).and_then(Value::as_str).map(str::to_string)
}

fn path_array(value: &Value, name: &str) -> Vec<PathBuf> {
    value
        .get(name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect()
}

fn command_array(value: &Value) -> Vec<String> {
    value
        .get("commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|command| {
            if let Some(parts) = command.as_array() {
                parts
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                command.as_str().unwrap_or_default().to_string()
            }
        })
        .filter(|command| !command.is_empty())
        .collect()
}

fn parse_executions(config_dir: &Path) -> Vec<ExecutionRef> {
    let root = config_dir.join("executions").join("package");
    let mut refs = Vec::new();
    for entry in fs::read_dir(root).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(value) = read_json_value(&path) else {
            continue;
        };
        let Some(id) = string_field(&value, "executionId") else {
            continue;
        };
        // Only cleanupTargets are disposable package paths.  `artifacts` is
        // deliberately excluded: legacy engine/package records sometimes
        // point at the entire UE installation or a user output root there.
        // Such paths are evidence, not deletion authorization.
        let mut paths = path_array(&value, "cleanupTargets");
        let outputs = path_array(&value, "outputs");
        let logs = path_array(&value, "logs");
        paths.sort();
        paths.dedup();
        refs.push(ExecutionRef {
            id,
            workspace: string_field(&value, "workspace").unwrap_or_default(),
            task_ref: value
                .get("taskRef")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    value
                        .get("metadata")
                        .and_then(|meta| meta.get("taskRef"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                }),
            state: string_field(&value, "state").unwrap_or_else(|| "unknown".into()),
            target_kind: value
                .get("targetKind")
                .and_then(Value::as_str)
                .or_else(|| {
                    value
                        .get("metadata")
                        .and_then(|meta| meta.get("targetKind"))
                        .and_then(Value::as_str)
                })
                .unwrap_or_default()
                .to_string(),
            paths,
            outputs,
            logs,
            commands: command_array(&value),
        });
    }
    refs.sort_by(|left, right| left.id.cmp(&right.id));
    refs
}

fn metadata_time(path: &Path) -> Option<String> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let timestamp = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    DateTime::<Utc>::from_timestamp(timestamp, 0).map(|value| value.to_rfc3339())
}

fn older_than(path: &Path, seconds: u64) -> bool {
    let Ok(modified) = fs::metadata(path).and_then(|metadata| metadata.modified()) else {
        return false;
    };
    modified
        .elapsed()
        .map(|elapsed| elapsed.as_secs() >= seconds)
        .unwrap_or(false)
}

fn file_bytes(path: &Path) -> u64 {
    if path.is_file() {
        return fs::metadata(path)
            .map(|meta| meta.len())
            .unwrap_or_default();
    }
    tree_bytes(path)
}

fn is_under(path: &Path, root: &Path) -> bool {
    let path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let root = dunce::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    path.starts_with(root)
}

fn protected_output(path: &Path) -> bool {
    path.join(".udf-manifest.json").is_file()
}

#[allow(clippy::too_many_arguments)]
fn add_item(
    items: &mut BTreeMap<PathBuf, PackageInventoryItem>,
    path: PathBuf,
    category: &str,
    owner_kind: &str,
    owner_id: &str,
    execution_ids: &[String],
    state: &str,
    active: bool,
    protection: Option<String>,
    reclaim_reason: Option<String>,
    confidence: &str,
) {
    if !path.exists() && !crate::junction::exists(&path).unwrap_or(false) {
        return;
    }
    let path = dunce::canonicalize(&path).unwrap_or(path);
    let item = PackageInventoryItem {
        category: category.into(),
        bytes: file_bytes(&path),
        last_modified: metadata_time(&path),
        path: path.clone(),
        owner_kind: owner_kind.into(),
        owner_id: owner_id.into(),
        execution_ids: execution_ids.to_vec(),
        state: state.into(),
        active,
        protection,
        reclaim_reason,
        confidence: confidence.into(),
        outcome: None,
    };
    items
        .entry(path)
        .and_modify(|existing| {
            existing.bytes = existing.bytes.max(item.bytes);
            existing.execution_ids.extend(item.execution_ids.clone());
            existing.execution_ids.sort();
            existing.execution_ids.dedup();
            if existing.owner_id.is_empty() {
                existing.owner_id = item.owner_id.clone();
            }
            if existing.owner_kind == "unknown" {
                existing.owner_kind = item.owner_kind.clone();
            }
            if existing.protection.is_none() {
                existing.protection = item.protection.clone();
            }
            if existing.reclaim_reason.is_none() {
                existing.reclaim_reason = item.reclaim_reason.clone();
            }
            if existing.confidence == "unknown" {
                existing.confidence = item.confidence.clone();
            }
            existing.active |= item.active;
            if existing.active {
                existing.state = "building".into();
            }
        })
        .or_insert(item);
}

fn profile_paths(config_dir: &Path) -> Vec<PathBuf> {
    let mut paths = scan_direct_children(&config_dir.join("package").join("profiles"))
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .collect::<Vec<_>>();
    let mut hosts_roots = Vec::new();
    if let Some(value) = fs::read_to_string(config_dir.join("config.toml"))
        .ok()
        .and_then(|text| toml::from_str::<toml::Value>(&text).ok())
    {
        if let Some(root) = value.get("hosts_root").and_then(toml::Value::as_str) {
            hosts_roots.push(PathBuf::from(root));
        }
        if let Some(workspaces) = value.get("workspaces").and_then(toml::Value::as_table) {
            for workspace in workspaces.values() {
                if let Some(root) = workspace.get("hosts_root").and_then(toml::Value::as_str) {
                    hosts_roots.push(PathBuf::from(root));
                }
            }
        }
    }
    hosts_roots.sort();
    hosts_roots.dedup();
    for hosts_root in hosts_roots {
        if let Ok(workspaces) = fs::read_dir(&hosts_root) {
            for workspace in workspaces.flatten() {
                if !workspace.path().is_dir() {
                    continue;
                }
                if let Ok(tasks) = fs::read_dir(workspace.path()) {
                    for task in tasks.flatten() {
                        let profile = task.path().join(".udf-package.toml");
                        if profile.is_file() {
                            paths.push(profile);
                        }
                    }
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn current_profile_bindings(config_dir: &Path) -> Vec<(String, PathBuf, PathBuf, String, String)> {
    let mut bindings = Vec::new();
    for path in profile_paths(config_dir) {
        let Ok(value) = fs::read_to_string(&path)
            .and_then(|text| toml::from_str::<toml::Value>(&text).map_err(std::io::Error::other))
        else {
            continue;
        };
        let get = |name: &str| {
            value
                .get(name)
                .and_then(toml::Value::as_str)
                .map(PathBuf::from)
        };
        let nested_str = |section: &str, name: &str| {
            value
                .get(section)
                .and_then(|section| section.get(name))
                .and_then(toml::Value::as_str)
        };
        let Some(task_uid) = value.get("task_uid").and_then(toml::Value::as_str) else {
            continue;
        };
        let Some(project) =
            get("project").or_else(|| nested_str("source", "project").map(PathBuf::from))
        else {
            continue;
        };
        let Some(engine) = get("engine") else {
            continue;
        };
        let configuration = nested_str("build", "configuration")
            .or_else(|| value.get("configuration").and_then(toml::Value::as_str))
            .unwrap_or("Development")
            .to_string();
        let container = nested_str("package", "container")
            .or_else(|| value.get("container").and_then(toml::Value::as_str))
            .unwrap_or("pak")
            .to_string();
        bindings.push((
            task_uid.to_string(),
            project,
            engine,
            configuration,
            container,
        ));
    }
    bindings
}

fn matching_profile_binding(
    root: &Path,
    bindings: &[(String, PathBuf, PathBuf, String, String)],
) -> Option<String> {
    let value = read_json_value(&root.join(".udf-cook-cache.json"))?;
    let task_uid = string_field(&value, "taskUid").or_else(|| string_field(&value, "task_uid"));
    let project = string_field(&value, "project").map(PathBuf::from);
    let engine = string_field(&value, "engine").map(PathBuf::from);
    let configuration = string_field(&value, "configuration");
    let container = string_field(&value, "container");
    let project_matches = |path: &Path, expected: &Path| {
        let path = if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("uproject"))
        {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
            == dunce::canonicalize(expected).unwrap_or_else(|_| expected.to_path_buf())
    };
    bindings.iter().find_map(
        |(binding, expected_project, expected_engine, expected_config, expected_container)| {
            let matches = task_uid.as_deref().is_none_or(|value| value == binding)
                && project
                    .as_ref()
                    .is_some_and(|path| project_matches(path, expected_project))
                && engine.as_ref().is_some_and(|path| {
                    dunce::canonicalize(path).unwrap_or_else(|_| path.clone())
                        == dunce::canonicalize(expected_engine)
                            .unwrap_or_else(|_| expected_engine.clone())
                })
                && configuration.as_deref() == Some(expected_config.as_str())
                && container.as_deref() == Some(expected_container.as_str());
            if matches { Some(binding.clone()) } else { None }
        },
    )
}

fn cache_matches_profile(
    root: &Path,
    bindings: &[(String, PathBuf, PathBuf, String, String)],
) -> bool {
    matching_profile_binding(root, bindings).is_some()
}

fn scan_direct_children(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect()
}

fn live_process_commands() -> Vec<String> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    let mut commands = system
        .processes()
        .values()
        .map(|process| {
            process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase()
        })
        .collect::<Vec<_>>();
    #[cfg(windows)]
    {
        // sysinfo intentionally redacts some Windows command lines when the
        // caller lacks PROCESS_QUERY_LIMITED_INFORMATION.  The read-only CIM
        // query fills that gap so an active UAT/UBT child can still protect a
        // temp staging path.
        if let Ok(output) = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-CimInstance Win32_Process | ForEach-Object { $_.CommandLine }",
            ])
            .output()
        {
            commands.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_ascii_lowercase),
            );
        }
    }
    commands
}

fn process_uses_path(commands: &[String], path: &Path, execution_ids: &[String]) -> bool {
    let path = path.to_string_lossy().to_ascii_lowercase();
    commands.iter().any(|command| {
        // The cleanup process itself names the execution ID on its command
        // line.  It is not a package producer and must not protect a failed
        // stage from an explicitly requested cleanup.
        if command.contains("udf") && command.contains("package") && command.contains("clean") {
            return false;
        }
        command.contains(&path)
            || execution_ids
                .iter()
                .any(|execution_id| command.contains(&execution_id.to_ascii_lowercase()))
    })
}

fn legacy_target_verified(reference: &ExecutionRef, path: &Path) -> bool {
    let canonical_path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let recorded = reference.paths.iter().any(|recorded| {
        dunce::canonicalize(recorded).unwrap_or_else(|_| recorded.clone()) == canonical_path
    }) || (path.file_name().and_then(|name| name.to_str())
        == Some("delivery-backup")
        && reference
            .logs
            .iter()
            .any(|log| log.parent() == path.parent()));
    if reference.id.is_empty() || !recorded {
        return false;
    }
    let id = reference.id.to_ascii_lowercase();
    let target = path.to_string_lossy().to_ascii_lowercase();
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    reference.commands.iter().any(|command| {
        let command = command.to_ascii_lowercase();
        command.contains(&target)
            || (!name.is_empty() && command.contains(&name))
            || command.contains(&id)
    })
}

fn classify_path(
    path: &Path,
    cache_root: &Path,
    temp_root: &Path,
    legacy_root: &Path,
) -> &'static str {
    if is_under(path, cache_root) {
        "cook-cache"
    } else if is_under(path, temp_root) || is_under(path, legacy_root) {
        "execution-stage"
    } else if path.file_name().and_then(|name| name.to_str()) == Some("delivery-backup") {
        "delivery-backup"
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
        "profile"
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        "record"
    } else if path.file_name().and_then(|name| name.to_str()) == Some(".udf-manifest.json") {
        "final-output"
    } else {
        "log"
    }
}

pub fn scan(
    config_dir: &Path,
    temp_dir: &Path,
    filter: &InventoryFilter,
) -> PackageInventoryReport {
    let cache_root = config_dir.join("package").join("cook-cache");
    let executions_root = config_dir.join("executions").join("package");
    let temp_root = temp_dir.join("UDF");
    let legacy_root = temp_dir.join("UnrealDevFlow").join("package-stage");
    let refs = parse_executions(config_dir);
    let bindings = current_profile_bindings(config_dir);
    let live_commands = live_process_commands();
    let package_wrapper_active = live_commands
        .iter()
        .any(|command| command.contains("udf") && command.contains("package"));
    let mut items = BTreeMap::new();

    // Execution records and profiles are audit data, never reclamation
    // targets.  They are included so a dry-run accounts for all package data.
    for path in scan_direct_children(&executions_root) {
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            add_item(
                &mut items,
                path,
                "record",
                "execution",
                "",
                &[],
                "ready",
                false,
                Some("execution audit record".into()),
                None,
                "verified",
            );
        }
    }
    for path in profile_paths(config_dir) {
        if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            let owner =
                toml::from_str::<toml::Value>(&fs::read_to_string(&path).unwrap_or_default())
                    .ok()
                    .and_then(|value| {
                        value
                            .get("task_uid")
                            .and_then(toml::Value::as_str)
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
            add_item(
                &mut items,
                path,
                "profile",
                if owner.starts_with("workspace/") {
                    "workspace"
                } else {
                    "task"
                },
                &owner,
                &[],
                "ready",
                false,
                Some("当前 package profile".into()),
                None,
                "verified",
            );
        }
    }

    let mut referenced: HashMap<PathBuf, Vec<ExecutionRef>> = HashMap::new();
    for reference in &refs {
        for path in &reference.paths {
            let key = dunce::canonicalize(path).unwrap_or_else(|_| path.clone());
            referenced.entry(key).or_default().push(reference.clone());
        }
    }

    // Persistent Cook slots.  A slot matching any current profile is hard
    // protected even when the old cache state predates schema 2.
    for path in scan_direct_children(&cache_root) {
        if !path.is_dir() {
            continue;
        }
        let active = package_cache::active_lease(&path).is_some()
            || process_uses_path(&live_commands, &path, &[]);
        let current = cache_matches_profile(&path, &bindings);
        let state = read_json_value(&path.join(".udf-cook-cache.json"));
        let owner = state
            .as_ref()
            .and_then(|value| {
                string_field(value, "taskUid").or_else(|| string_field(value, "task_uid"))
            })
            .or_else(|| {
                if current {
                    matching_profile_binding(&path, &bindings)
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let state_name = state
            .as_ref()
            .and_then(|value| string_field(value, "state"))
            .unwrap_or_else(|| {
                if current {
                    "ready".into()
                } else {
                    "orphan".into()
                }
            });
        let (protection, reason, confidence) = if active {
            (Some("活动 Cook lease".into()), None, "verified")
        } else if current {
            (Some("当前 profile 对应的缓存槽".into()), None, "verified")
        } else if state.is_some() {
            (
                None,
                Some("非当前 profile 的历史缓存，可按 stale 规则回收".into()),
                "legacy-matched",
            )
        } else {
            (
                None,
                Some("缺少状态文件的 orphan 缓存，需要显式 legacy 确认".into()),
                "unknown",
            )
        };
        let owner_kind = if owner.starts_with("workspace/") {
            "workspace"
        } else if owner.is_empty() {
            "unknown"
        } else {
            "task"
        };
        add_item(
            &mut items,
            path,
            "cook-cache",
            owner_kind,
            owner.trim_start_matches("workspace/"),
            &[],
            &state_name,
            active,
            protection,
            reason,
            confidence,
        );
    }

    // Legacy temporary roots may contain both execution stage and old
    // plugin stage hashes.  Their exact paths are retained in the inventory.
    for (root, confidence) in [
        (&temp_root, "legacy-matched"),
        (&legacy_root, "legacy-matched"),
    ] {
        for path in scan_direct_children(root) {
            let key = dunce::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let matches = referenced.get(&key).cloned().unwrap_or_default();
            let execution_ids = matches
                .iter()
                .map(|reference| reference.id.clone())
                .collect::<Vec<_>>();
            let owner = matches
                .iter()
                .find_map(|reference| reference.task_ref.clone())
                .unwrap_or_else(|| {
                    matches
                        .first()
                        .map(|reference| reference.workspace.clone())
                        .unwrap_or_default()
                });
            let active = package_cache::active_lease(&path).is_some()
                || process_uses_path(&live_commands, &path, &execution_ids)
                // Legacy UDF wrappers did not persist a lease or execution
                // path in their command line.  While any package wrapper is
                // alive, an unreferenced temp child is conservatively held.
                || (matches.is_empty() && package_wrapper_active);
            let stale_running = matches.iter().any(|reference| reference.state == "running")
                && older_than(&path, 24 * 60 * 60);
            let recent_orphan = matches.is_empty() && !older_than(&path, 24 * 60 * 60);
            let base_state = matches
                .iter()
                .find(|reference| reference.state != "running")
                .map(|reference| reference.state.as_str())
                .unwrap_or("running");
            let state = if active {
                "building"
            } else if stale_running {
                "stale-running"
            } else if matches.is_empty() {
                if recent_orphan { "unknown" } else { "stale" }
            } else if base_state == "failed" && !older_than(&path, 24 * 60 * 60) {
                "failed-recent"
            } else {
                base_state
            };
            let legacy_reason = if matches.is_empty() {
                "未能映射到 execution，需 legacy 确认"
            } else {
                "旧 execution 记录可追溯，已过渡到 legacy 清理"
            };
            let item_protection = if active {
                Some("活动执行或 lease".into())
            } else if recent_orphan {
                Some("未关联 staging 未满 24 小时，暂缓清理".into())
            } else if state == "failed-recent" {
                Some("失败 staging 保留 24 小时供诊断".into())
            } else {
                None
            };
            let item_confidence = if recent_orphan {
                "unknown"
            } else if matches
                .iter()
                .any(|reference| !reference.target_kind.is_empty())
            {
                "verified"
            } else {
                confidence
            };
            let item_confidence = if !matches.is_empty()
                && matches.iter().all(|reference| {
                    reference.target_kind.is_empty() && legacy_target_verified(reference, &path)
                }) {
                "legacy-matched"
            } else if !matches.is_empty()
                && matches
                    .iter()
                    .any(|reference| reference.target_kind.is_empty())
            {
                "unknown"
            } else {
                item_confidence
            };
            add_item(
                &mut items,
                path,
                "execution-stage",
                if owner.contains('/') {
                    "task"
                } else {
                    "workspace"
                },
                &owner,
                &execution_ids,
                state,
                active,
                item_protection,
                Some(legacy_reason.into()),
                item_confidence,
            );
        }
    }

    // Recorded logs, backups, manifests and outputs are walked as individual
    // targets.  Final output is always hard protected, including outputs
    // outside C:\Package, because the manifest proves UDF ownership.
    let mut seen_paths = BTreeSet::new();
    for reference in &refs {
        let owner = reference
            .task_ref
            .clone()
            .unwrap_or_else(|| reference.workspace.clone());
        let owner_kind = if reference.task_ref.is_some() {
            "task"
        } else {
            "workspace"
        };
        for log in &reference.logs {
            if let Some(parent) = log.parent() {
                let backup = parent.join("delivery-backup");
                if backup.is_dir() {
                    let backup_confidence = if reference.target_kind.is_empty() {
                        if legacy_target_verified(reference, &backup) {
                            "legacy-matched"
                        } else {
                            "unknown"
                        }
                    } else {
                        "verified"
                    };
                    add_item(
                        &mut items,
                        backup,
                        "delivery-backup",
                        owner_kind,
                        &owner,
                        std::slice::from_ref(&reference.id),
                        if reference.state == "running" {
                            "building"
                        } else {
                            "delivered"
                        },
                        false,
                        if reference.state == "delivering" {
                            Some("交付事务正在进行".into())
                        } else {
                            None
                        },
                        if reference.state == "delivering" {
                            None
                        } else {
                            Some("delivered backup 可在摘要核对后回收".into())
                        },
                        backup_confidence,
                    );
                }
            }
        }
        for path in reference
            .paths
            .iter()
            .chain(reference.outputs.iter())
            .chain(reference.logs.iter())
        {
            if !path.exists() {
                continue;
            }
            // A few legacy engine records stored a directory in `logs` even
            // though it was the engine installation itself.  Only count a
            // log directory when it is under an explicit UnrealDevFlow root;
            // never walk arbitrary engine/project directories from metadata.
            if reference.logs.iter().any(|log| log == path) && path.is_dir() {
                let lower = path.to_string_lossy().to_ascii_lowercase();
                if !lower.contains("unrealdevflow") {
                    continue;
                }
            }
            let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.clone());
            if !seen_paths.insert(canonical.clone()) {
                continue;
            }
            let is_output = reference.outputs.iter().any(|output| {
                dunce::canonicalize(output).unwrap_or_else(|_| output.clone()) == canonical
            });
            let category = if is_output && protected_output(&canonical) {
                "final-output"
            } else if is_output {
                "unknown-output"
            } else if canonical.file_name().and_then(|name| name.to_str())
                == Some("delivery-backup")
            {
                "delivery-backup"
            } else if canonical.extension().and_then(|ext| ext.to_str()) == Some("log")
                || canonical.to_string_lossy().contains(".udf-logs")
            {
                "log"
            } else {
                classify_path(&canonical, &cache_root, &temp_root, &legacy_root)
            };
            let final_output = category == "final-output";
            let unknown_output = category == "unknown-output";
            let delivered_backup = category == "delivery-backup";
            let active = reference.state == "running"
                && (package_cache::active_lease(&canonical).is_some()
                    || process_uses_path(
                        &live_commands,
                        &canonical,
                        std::slice::from_ref(&reference.id),
                    ));
            let state = if active {
                "building"
            } else if reference.state == "running" && older_than(&canonical, 24 * 60 * 60) {
                "stale-running"
            } else if reference.state == "failed" && !older_than(&canonical, 24 * 60 * 60) {
                "failed-recent"
            } else if category == "log" && older_than(&canonical, 14 * 24 * 60 * 60) {
                "stale"
            } else {
                reference.state.as_str()
            };
            let item_confidence = if reference.target_kind.is_empty() {
                if legacy_target_verified(reference, &canonical) {
                    "legacy-matched"
                } else {
                    "unknown"
                }
            } else {
                "verified"
            };
            add_item(
                &mut items,
                canonical,
                category,
                owner_kind,
                &owner,
                std::slice::from_ref(&reference.id),
                state,
                active,
                if final_output {
                    Some("最终输出和 manifest 受保护".into())
                } else if unknown_output {
                    Some("输出缺少 manifest，保持保护并等待人工确认".into())
                } else if state == "failed-recent" {
                    Some("失败 execution 保留 24 小时供诊断".into())
                } else if active {
                    Some("活动 execution".into())
                } else if delivered_backup {
                    Some("交付事务证据，确认后可回收".into())
                } else {
                    None
                },
                if final_output || unknown_output {
                    None
                } else if delivered_backup {
                    Some("delivered 备份可在摘要核对后回收".into())
                } else {
                    Some("execution 记录中的受管路径".into())
                },
                item_confidence,
            );
        }
    }

    let mut collected = items
        .into_values()
        .filter(|item| filter.matches(item))
        .collect::<Vec<_>>();
    // A recorded execution can mention both a directory and files below it
    // (for example a log directory plus step-01.log).  Keep the owning
    // directory as the accounting unit so bytes are never double-counted.
    collected.sort_by(|left, right| {
        left.path
            .components()
            .count()
            .cmp(&right.path.components().count())
            .then_with(|| left.path.cmp(&right.path))
    });
    let mut kept = Vec::new();
    for item in collected {
        if kept.iter().any(|parent: &PackageInventoryItem| {
            parent.path != item.path && item.path.starts_with(&parent.path)
        }) {
            continue;
        }
        kept.push(item);
    }
    let mut output = PackageInventoryReport {
        items: kept,
        total_bytes: 0,
        reclaimable_bytes: 0,
        protected_bytes: 0,
        unknown_bytes: 0,
        diagnostics: Vec::new(),
    };
    for item in &output.items {
        output.total_bytes = output.total_bytes.saturating_add(item.bytes);
        if item.active || item.protection.is_some() {
            output.protected_bytes = output.protected_bytes.saturating_add(item.bytes);
        } else if item.reclaim_reason.is_some() && item.confidence != "unknown" {
            output.reclaimable_bytes = output.reclaimable_bytes.saturating_add(item.bytes);
        } else {
            output.unknown_bytes = output.unknown_bytes.saturating_add(item.bytes);
        }
    }
    if output.items.is_empty() {
        output
            .diagnostics
            .push("未发现可盘点的 UDF package 数据".into());
    }
    output
}

pub fn fixed_roots(config_dir: &Path, temp_dir: &Path) -> Vec<PathBuf> {
    vec![
        config_dir.join("package").join("cook-cache"),
        config_dir.join("package").join("profiles"),
        config_dir.join("executions").join("package"),
        temp_dir.join("UDF"),
        temp_dir.join("UnrealDevFlow").join("package-stage"),
    ]
}

pub fn is_fixed_root_path(path: &Path, config_dir: &Path, temp_dir: &Path) -> bool {
    fixed_roots(config_dir, temp_dir)
        .iter()
        .any(|root| is_under(path, root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_record_requires_three_matching_ownership_signals() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("UDF/stage");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("payload"), b"payload").unwrap();
        let config = root.path().join("config");
        fs::create_dir_all(config.join("executions/package")).unwrap();
        fs::write(
            config.join("executions/package/package-project-1.json"),
            serde_json::json!({
                "executionId":"package-project-1",
                "workspace":"test",
                "state":"succeeded",
                "cleanupTargets":[target],
                "outputs":[],
                "logs":[],
                "commands":[["RunUAT.bat", "-project=UDF/stage/Project.uproject", "package-project-1"]]
            })
            .to_string(),
        )
        .unwrap();
        let report = scan(&config, root.path(), &InventoryFilter::default());
        let item = report
            .items
            .iter()
            .find(|item| item.path == dunce::canonicalize(&target).unwrap())
            .unwrap();
        assert_eq!(item.confidence, "legacy-matched");
        assert!(
            item.execution_ids
                .contains(&"package-project-1".to_string())
        );
    }

    #[test]
    fn final_output_is_never_an_automatic_cleanup_target() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        let output = root.path().join("Package/Final");
        fs::create_dir_all(&output).unwrap();
        fs::write(output.join(".udf-manifest.json"), "{\"files\":[]}").unwrap();
        fs::create_dir_all(config.join("executions/package")).unwrap();
        fs::write(
            config.join("executions/package/package-project-1.json"),
            serde_json::json!({
                "executionId":"package-project-1","workspace":"test","state":"succeeded",
                "cleanupTargets":[],"outputs":[output],"logs":[]
            })
            .to_string(),
        )
        .unwrap();
        let report = scan(&config, root.path(), &InventoryFilter::default());
        let item = report
            .items
            .iter()
            .find(|item| item.category == "final-output")
            .unwrap();
        assert!(item.protection.is_some());
        assert!(item.reclaim_reason.is_none());
    }
}
