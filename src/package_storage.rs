//! Package storage and lineage preflight.
//!
//! This module deliberately measures only ordinary files. Junctions are
//! boundaries owned by the project or another tool and must not inflate UDF's
//! accounting or become deletion targets.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::Disks;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

const MIN_FREE_RESERVE: u64 = 50 * 1024 * 1024 * 1024;
const DEFAULT_ESTIMATE: u64 = 1024 * 1024 * 1024;
const LARGE_TEMP_WARNING: u64 = 100 * 1024 * 1024 * 1024;
const CACHE_CAP: u64 = 250 * 1024 * 1024 * 1024;
const CACHE_CAP_RATIO_PERCENT: u64 = 30;
const FREE_RESERVE_RATIO_PERCENT: u64 = 10;

fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageReport {
    pub current_output_bytes: u64,
    pub current_temp_bytes: u64,
    pub estimated_additional_bytes: u64,
    pub output_free_bytes: Option<u64>,
    pub temp_free_bytes: Option<u64>,
    pub sibling_version_count: u64,
    pub sibling_version_bytes: u64,
    #[serde(default)]
    pub managed_package_bytes: u64,
    #[serde(default)]
    pub reclaimable_package_bytes: u64,
    #[serde(default)]
    pub protected_package_bytes: u64,
    #[serde(default)]
    pub free_reserve_bytes: u64,
    #[serde(default)]
    pub cache_cap_bytes: u64,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

impl StorageReport {
    pub fn blocked(&self) -> bool {
        self.decision == "blocked"
    }
}

fn measure_tree(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    // Reparse points are boundaries.  Avoid the relatively expensive
    // Junction probe for ordinary files; only directories need the extra
    // check because Windows represents Junctions as directory reparse points.
    if is_reparse_point(&metadata) {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| measure_tree(&entry.path()))
        .sum()
}

pub fn tree_bytes(path: &Path) -> u64 {
    measure_tree(path)
}

fn disk_free(path: &Path) -> Option<u64> {
    let probe = if path.exists() { path } else { path.parent()? };
    let canonical = dunce::canonicalize(probe).unwrap_or_else(|_| probe.to_path_buf());
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|disk| canonical.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(|disk| disk.available_space())
}

fn disk_total(path: &Path) -> Option<u64> {
    let probe = if path.exists() { path } else { path.parent()? };
    let canonical = dunce::canonicalize(probe).unwrap_or_else(|_| probe.to_path_buf());
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|disk| canonical.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())
        .map(|disk| disk.total_space())
}

fn sibling_inventory(output: &Path) -> (u64, u64) {
    let Some(parent) = output.parent() else {
        return (0, 0);
    };
    let Some(name) = output.file_name().and_then(|value| value.to_str()) else {
        return (0, 0);
    };
    let prefix = name
        .split_once("-Win64-")
        .map(|(prefix, _)| prefix)
        .unwrap_or(name);
    let mut count = 0;
    let mut bytes = 0;
    for entry in fs::read_dir(parent).into_iter().flatten().flatten() {
        let candidate = entry.path();
        if candidate == output
            || !candidate.is_dir()
            || !candidate
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.starts_with(prefix))
        {
            continue;
        }
        // Only count sibling packages that UDF can identify. Walking an
        // arbitrary user directory here would make every preflight as costly
        // as the package itself and could follow unrelated data trees.
        let manifest = candidate.join(".udf-manifest.json");
        let Ok(bytes_json) = fs::read(&manifest) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes_json) else {
            continue;
        };
        count += 1;
        bytes += value
            .get("files")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.get("bytes")?.as_u64())
            .sum::<u64>();
    }
    (count, bytes)
}

fn assess_impl(
    output: &Path,
    temp_roots: &[PathBuf],
    estimated_hint: Option<u64>,
    include_global_package_data: bool,
    config_dir_override: Option<&Path>,
) -> StorageReport {
    let current_output_bytes = measure_tree(output);
    let config_dir = if let Some(config_dir) = config_dir_override {
        config_dir.to_path_buf()
    } else if include_global_package_data {
        crate::config::Config::config_dir().unwrap_or_default()
    } else {
        PathBuf::new()
    };
    let cache_root = config_dir.join("package").join("cook-cache");
    let persistent_cache_bytes = measure_tree(&cache_root);
    let current_temp_bytes = temp_roots
        .iter()
        .map(|path| {
            let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.clone());
            let cache_canonical =
                dunce::canonicalize(&cache_root).unwrap_or_else(|_| cache_root.clone());
            if canonical.starts_with(&cache_canonical) {
                0
            } else {
                measure_tree(path)
            }
        })
        .sum::<u64>()
        .saturating_add(persistent_cache_bytes);
    let source_input_bytes = temp_roots
        .iter()
        .find(|path| path.is_dir() && path.join("Config").exists())
        .map(|path| {
            [
                "Config", "Content", "Plugins", "Source", "Build", "Shaders", "Binaries",
            ]
            .iter()
            .map(|name| measure_tree(&path.join(name)))
            .sum::<u64>()
        })
        .unwrap_or_default();
    let historical_estimate = persistent_cache_bytes.max(source_input_bytes.saturating_mul(5) / 2);
    let estimated_additional_bytes = if include_global_package_data {
        estimated_hint
            .unwrap_or_default()
            .max(current_output_bytes)
            .max(historical_estimate)
            .max(DEFAULT_ESTIMATE)
    } else {
        estimated_hint.unwrap_or_else(|| current_output_bytes.max(DEFAULT_ESTIMATE))
    };
    let output_free_bytes = disk_free(output);
    let temp_free_bytes = temp_roots
        .iter()
        .find_map(|path| disk_free(path))
        .or_else(|| {
            if include_global_package_data {
                disk_free(&config_dir)
            } else {
                None
            }
        });
    let (sibling_version_count, sibling_version_bytes) = sibling_inventory(output);
    let mut diagnostics = Vec::new();
    if sibling_version_count > 0 {
        diagnostics.push(format!(
            "同一任务已有 {} 个兄弟版本，占用约 {} bytes；本次预计新增 {} bytes",
            sibling_version_count, sibling_version_bytes, estimated_additional_bytes
        ));
    }
    if current_temp_bytes >= LARGE_TEMP_WARNING {
        diagnostics.push(format!(
            "UDF 临时/缓存目录当前占用 {} bytes，建议先执行 package clean --dry-run",
            current_temp_bytes
        ));
    }
    let disk_capacity = disk_total(output)
        .or_else(|| temp_roots.iter().find_map(|path| disk_total(path)))
        .or_else(|| disk_total(&config_dir))
        .or(output_free_bytes)
        .or(temp_free_bytes)
        .unwrap_or_default();
    let free_reserve_bytes = MIN_FREE_RESERVE.max(
        disk_capacity
            .saturating_mul(FREE_RESERVE_RATIO_PERCENT)
            .saturating_div(100),
    );
    let cache_cap_bytes = CACHE_CAP.min(
        disk_capacity
            .saturating_mul(CACHE_CAP_RATIO_PERCENT)
            .saturating_div(100),
    );
    let cache_over_cap = cache_cap_bytes > 0 && persistent_cache_bytes > cache_cap_bytes;
    if cache_over_cap {
        diagnostics.push(format!(
            "Cook cache 当前 {} bytes，超过 {} bytes 全局上限；请先执行 package clean --stale --yes",
            persistent_cache_bytes, cache_cap_bytes
        ));
    }
    let required = estimated_additional_bytes.saturating_add(free_reserve_bytes);
    let blocked = output_free_bytes.is_some_and(|free| free < estimated_additional_bytes)
        || temp_free_bytes.is_some_and(|free| free < estimated_additional_bytes);
    let warning = output_free_bytes.is_none()
        || temp_free_bytes.is_none()
        || output_free_bytes.is_some_and(|free| free < required)
        || temp_free_bytes.is_some_and(|free| free < required)
        || current_temp_bytes >= LARGE_TEMP_WARNING
        || sibling_version_count >= 3
        || cache_over_cap;
    let decision = if blocked {
        diagnostics.push("可用空间不足，已阻止启动 UAT/BuildGraph".into());
        "blocked"
    } else if warning {
        diagnostics.push(format!(
            "空间预检为 warning；建议至少保留 {} bytes 的额外空间",
            required
        ));
        "warning"
    } else {
        "ready"
    };
    StorageReport {
        current_output_bytes,
        current_temp_bytes,
        estimated_additional_bytes,
        output_free_bytes,
        temp_free_bytes,
        sibling_version_count,
        sibling_version_bytes,
        managed_package_bytes: current_temp_bytes,
        reclaimable_package_bytes: persistent_cache_bytes.saturating_sub(cache_cap_bytes),
        protected_package_bytes: current_temp_bytes
            .saturating_sub(persistent_cache_bytes.saturating_sub(cache_cap_bytes)),
        free_reserve_bytes,
        cache_cap_bytes,
        decision: decision.into(),
        diagnostics,
    }
}

/// Lightweight preflight used by isolated callers and unit tests.  It only
/// measures the explicitly supplied roots.
pub fn assess(output: &Path, temp_roots: &[PathBuf], estimated_hint: Option<u64>) -> StorageReport {
    assess_impl(output, temp_roots, estimated_hint, false, None)
}

/// Package commands use the global form so the preflight accounts for all
/// UDF Cook cache bytes, not merely the current execution's staging root.
pub fn assess_global(
    output: &Path,
    temp_roots: &[PathBuf],
    estimated_hint: Option<u64>,
) -> StorageReport {
    assess_impl(output, temp_roots, estimated_hint, true, None)
}

/// Estimate the disposable Cook inputs which will be copied for a project.
/// The caller may pass this to [`assess_global`] so a first-ever package is
/// not optimistically treated as a 1 GiB operation.
pub fn source_input_estimate(project_root: &Path) -> u64 {
    let input_bytes = [
        "Config", "Content", "Plugins", "Source", "Build", "Shaders", "Binaries",
    ]
    .iter()
    .map(|name| measure_tree(&project_root.join(name)))
    .sum::<u64>();
    input_bytes.saturating_mul(5) / 2
}

#[cfg(test)]
pub fn assess_global_for_config(
    output: &Path,
    temp_roots: &[PathBuf],
    estimated_hint: Option<u64>,
    config_dir: &Path,
) -> StorageReport {
    assess_impl(output, temp_roots, estimated_hint, true, Some(config_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_paths_are_safe_to_measure() {
        let root = tempfile::tempdir().unwrap();
        let report = assess(
            &root.path().join("missing-output"),
            &[root.path().join("missing-temp")],
            Some(1),
        );
        assert_eq!(report.current_output_bytes, 0);
        assert_eq!(report.current_temp_bytes, 0);
        assert_eq!(report.estimated_additional_bytes, 1);
    }

    #[test]
    fn sibling_versions_are_counted_without_following_junctions() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("Game-task-Win64-Development");
        let sibling = root.path().join("Game-task-Win64-Shipping");
        fs::create_dir_all(&sibling).unwrap();
        fs::write(sibling.join("Game.exe"), b"package").unwrap();
        fs::write(
            sibling.join(".udf-manifest.json"),
            r#"{"files":[{"path":"Game.exe","bytes":7}]}"#,
        )
        .unwrap();
        let report = assess(&output, &[], Some(1));
        assert_eq!(report.sibling_version_count, 1);
        assert_eq!(report.sibling_version_bytes, 7);
    }

    #[test]
    fn global_preflight_uses_historical_cache_size_and_exposes_cap() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        let cache = config.join("package/cook-cache/slot");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("payload.bin"), vec![0_u8; 1024 * 1024]).unwrap();
        let report = assess_global_for_config(
            &root.path().join("output"),
            &[root.path().join("temp")],
            None,
            &config,
        );
        assert!(report.managed_package_bytes >= 1024 * 1024);
        assert!(report.estimated_additional_bytes >= DEFAULT_ESTIMATE);
        assert!(report.cache_cap_bytes > 0);
    }
}
