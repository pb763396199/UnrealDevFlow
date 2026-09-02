//! Package storage and lineage preflight.
//!
//! This module deliberately measures only ordinary files. Junctions are
//! boundaries owned by the project or another tool and must not inflate UDF's
//! accounting or become deletion targets.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::Disks;

const MIN_FREE_RESERVE: u64 = 512 * 1024 * 1024;
const DEFAULT_ESTIMATE: u64 = 1024 * 1024 * 1024;
const LARGE_TEMP_WARNING: u64 = 100 * 1024 * 1024 * 1024;

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
    pub decision: String,
    pub diagnostics: Vec<String>,
}

impl StorageReport {
    pub fn blocked(&self) -> bool {
        self.decision == "blocked"
    }
}

fn measure_tree(path: &Path) -> u64 {
    if !path.exists() || crate::junction::exists(path).unwrap_or(false) {
        return 0;
    }
    if path.is_file() {
        return fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
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

pub fn assess(output: &Path, temp_roots: &[PathBuf], estimated_hint: Option<u64>) -> StorageReport {
    let current_output_bytes = measure_tree(output);
    let current_temp_bytes = temp_roots.iter().map(|path| measure_tree(path)).sum();
    let estimated_additional_bytes =
        estimated_hint.unwrap_or_else(|| current_output_bytes.max(DEFAULT_ESTIMATE));
    let output_free_bytes = disk_free(output);
    let temp_free_bytes = temp_roots.iter().find_map(|path| disk_free(path));
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
    let required = estimated_additional_bytes.saturating_add(MIN_FREE_RESERVE);
    let blocked = output_free_bytes.is_some_and(|free| free < estimated_additional_bytes)
        || temp_free_bytes.is_some_and(|free| free < estimated_additional_bytes);
    let warning = output_free_bytes.is_none()
        || temp_free_bytes.is_none()
        || output_free_bytes.is_some_and(|free| free < required)
        || temp_free_bytes.is_some_and(|free| free < required)
        || current_temp_bytes >= LARGE_TEMP_WARNING
        || sibling_version_count >= 3;
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
        decision: decision.into(),
        diagnostics,
    }
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
}
