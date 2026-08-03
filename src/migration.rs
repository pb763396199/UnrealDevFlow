//! Schema migrations for `.udf-meta.json`.
//!
//! v1 metadata has no `schema_version` and a single `branch` + `based_on` pair.
//! v2 introduces explicit `schema_version` and `primary_plugins` /
//! `dependency_plugins` arrays. A v1 file does not contain enough evidence to
//! identify the primary plugin, so migration must not invent one.

use crate::config::Config;
use crate::error::Result;
use crate::host::{CURRENT_SCHEMA_VERSION, TaskMeta};
use crate::output;
use std::fs;
use std::path::Path;

/// Mutate `meta` in place if it appears to be v1.
pub fn migrate_in_place(meta: &mut TaskMeta) {
    if meta.schema_version >= CURRENT_SCHEMA_VERSION {
        return;
    }
    if meta.primary_plugins.is_empty() {
        // Keep the legacy schema visible until an evidence-based repair can
        // recover plugin identity from the Host/worktree/uproject. Marking it
        // current here would turn an old AesWorld assumption into false fact.
        return;
    }
    meta.schema_version = CURRENT_SCHEMA_VERSION;
}

/// Persist migrated metadata, backing up the original `.udf-meta.json` to
/// `.udf-meta.json.v1.bak` exactly once.
pub fn persist_migration_if_needed(host_dir: &Path) -> Result<bool> {
    let meta_path = host_dir.join(".udf-meta.json");
    if !meta_path.exists() {
        return Ok(false);
    }
    let original = fs::read_to_string(&meta_path)?;
    let probe: serde_json::Value =
        serde_json::from_str(&original).unwrap_or(serde_json::Value::Null);
    let needs_migration = probe
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .map(|v| (v as u32) < CURRENT_SCHEMA_VERSION)
        .unwrap_or(true);
    if !needs_migration {
        return Ok(false);
    }

    let backup_path = host_dir.join(".udf-meta.json.v1.bak");
    if !backup_path.exists() {
        fs::write(&backup_path, &original)?;
        output::print_info(&format!("Backed up v1 metadata to {:?}", backup_path));
    }

    let mut meta: TaskMeta = serde_json::from_str(&original)
        .map_err(|e| crate::error::HostError::InvalidMeta(format!("JSON parse error: {}", e)))?;
    migrate_in_place(&mut meta);
    if meta.schema_version < CURRENT_SCHEMA_VERSION {
        return Err(crate::error::UdfError::Other(
            "旧任务元数据缺少 primary plugin 身份，不能安全自动迁移；请从 Host/git/uproject 证据恢复后重试。"
                .to_string(),
        ));
    }
    crate::host::write_meta(host_dir, &meta)?;
    Ok(true)
}

/// Try to ensure a migrated v1 primary plugin has a valid `source_repo`, using
/// the configured plugins root or the legacy `plugin_path` from config.
pub fn backfill_source_repo(meta: &mut TaskMeta, config: &Config) {
    for plugin in meta.primary_plugins.iter_mut() {
        if plugin.source_repo.as_os_str().is_empty() {
            if let Some(root) = config.effective_plugins_root() {
                let candidate = root.join(&plugin.name);
                if candidate.exists() {
                    plugin.source_repo = candidate;
                    continue;
                }
            }
            if let Some(legacy) = &config.plugin_path {
                if legacy
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s == plugin.name)
                    .unwrap_or(false)
                {
                    plugin.source_repo = legacy.clone();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_meta_without_plugin_identity_is_not_assumed_to_be_aesworld() {
        let mut meta = TaskMeta {
            schema_version: 1,
            id: "legacy".to_string(),
            name: "legacy".to_string(),
            branch: "feature/legacy".to_string(),
            created: "2026-07-16T00:00:00Z".to_string(),
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
            workspace: None,
            task_uid: None,
            context: None,
        };

        migrate_in_place(&mut meta);

        assert_eq!(meta.schema_version, 1);
        assert!(meta.primary_plugins.is_empty());
    }
}
