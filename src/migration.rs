//! Schema migrations for `.udf-meta.json`.
//!
//! v1 metadata has no `schema_version` and a single `branch` + `based_on` pair.
//! v2 introduces explicit `schema_version` and `primary_plugins` /
//! `dependency_plugins` arrays. Migration synthesizes a single primary plugin
//! entry from the v1 fields, assuming the legacy hard-coded `AesWorld`
//! worktree path.

use crate::config::Config;
use crate::error::Result;
use crate::host::{PrimaryPlugin, TaskMeta, CURRENT_SCHEMA_VERSION};
use crate::output;
use std::fs;
use std::path::{Path, PathBuf};

/// Mutate `meta` in place if it appears to be v1.
pub fn migrate_in_place(meta: &mut TaskMeta) {
    if meta.schema_version >= CURRENT_SCHEMA_VERSION {
        return;
    }
    if meta.primary_plugins.is_empty() {
        let plugin_name = legacy_plugin_name();
        meta.primary_plugins.push(PrimaryPlugin {
            name: plugin_name.clone(),
            source_repo: PathBuf::new(),
            worktree: PathBuf::from("Plugins").join(&plugin_name),
            branch: meta.branch.clone(),
            based_on: meta.based_on.clone(),
        });
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
    let probe: serde_json::Value = serde_json::from_str(&original).unwrap_or(serde_json::Value::Null);
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

    let mut meta: TaskMeta = serde_json::from_str(&original).map_err(|e| {
        crate::error::HostError::InvalidMeta(format!("JSON parse error: {}", e))
    })?;
    migrate_in_place(&mut meta);
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

fn legacy_plugin_name() -> String {
    "AesWorld".to_string()
}
