//! Stable package Cook cache identity and process-aware leases.
//!
//! A cache directory is a reusable slot, not a snapshot of every profile
//! revision.  Fields which affect isolation belong in [`CacheIdentity`].
//! Human-facing profile fields (name, output and revision) are deliberately
//! excluded so editing a profile does not create another multi-gigabyte tree.

use crate::error::{Result, UdfError};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{Pid, ProcessesToUpdate, System};

pub const CACHE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CacheIdentity {
    pub schema_version: u32,
    pub task_uid: String,
    pub project: PathBuf,
    pub engine: PathBuf,
    pub platform: String,
    pub configuration: String,
    pub container: String,
}

impl CacheIdentity {
    pub fn new(
        task_uid: impl Into<String>,
        project: &Path,
        engine: &Path,
        platform: impl Into<String>,
        configuration: impl Into<String>,
        container: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: CACHE_SCHEMA_VERSION,
            task_uid: task_uid.into(),
            project: canonical_or_original(project),
            engine: canonical_or_original(engine),
            platform: platform.into(),
            configuration: configuration.into(),
            container: container.into(),
        }
    }

    /// The stable slot id.  Do not add profile revision, package name,
    /// output path or change reason here: those are audit metadata only.
    pub fn cache_id(&self) -> String {
        let value = format!(
            "schema={}|task={}|project={}|engine={}|platform={}|configuration={}|container={}",
            self.schema_version,
            self.task_uid,
            self.project.display(),
            self.engine.display(),
            self.platform,
            self.configuration,
            self.container
        );
        format!("{:x}", Md5::digest(value.as_bytes()))
    }

    pub fn cache_root(&self, config_dir: &Path) -> PathBuf {
        config_dir
            .join("package")
            .join("cook-cache")
            .join(self.cache_id())
    }
}

fn canonical_or_original(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CacheLease {
    pub pid: u32,
    pub process_start_time: u64,
    pub host: String,
    pub execution_id: String,
    pub acquired_at: u64,
    pub heartbeat_at: u64,
}

pub struct CacheLeaseGuard {
    lock_path: PathBuf,
    lease_path: PathBuf,
}

impl Drop for CacheLeaseGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lease_path);
        let _ = fs::remove_dir(&self.lock_path);
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn current_process_start_time() -> u64 {
    let Ok(pid) = sysinfo::get_current_pid() else {
        return 0;
    };
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system
        .process(pid)
        .map(|process| process.start_time())
        .unwrap_or(0)
}

pub fn lease_path(root: &Path) -> PathBuf {
    root.join(".udf-cook-cache-lease.json")
}

pub fn lock_path(root: &Path) -> PathBuf {
    root.join(".udf-cook-cache.lock")
}

pub fn read_lease(root: &Path) -> Option<CacheLease> {
    fs::read(lease_path(root))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

/// A lease is live only when both PID and process start time match.  A stale
/// JSON file from a crashed process therefore cannot block cleanup forever.
pub fn lease_is_active(lease: &CacheLease) -> bool {
    let pid = Pid::from_u32(lease.pid);
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    system
        .process(pid)
        .is_some_and(|process| process.start_time() == lease.process_start_time)
}

pub fn active_lease(root: &Path) -> Option<CacheLease> {
    read_lease(root).filter(lease_is_active)
}

pub fn acquire_lease(root: &Path, execution_id: impl Into<String>) -> Result<CacheLeaseGuard> {
    fs::create_dir_all(root)?;
    let lock_path = lock_path(root);
    match fs::create_dir(&lock_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if let Some(lease) = active_lease(root) {
                return Err(UdfError::Other(format!(
                    "Cook cache 正在使用：{} (PID {})",
                    root.display(),
                    lease.pid
                )));
            }
            // A lock without a live owner is recoverable only after a crash
            // grace period.  Immediately deleting a freshly-created lock
            // would race the owner between mkdir and lease-file creation.
            let stale_lock = fs::metadata(&lock_path)
                .and_then(|metadata| metadata.modified())
                .and_then(|modified| modified.elapsed().map_err(std::io::Error::other))
                .map(|elapsed| elapsed.as_secs() >= 24 * 60 * 60)
                .unwrap_or(false);
            if !stale_lock {
                return Err(UdfError::Other(format!(
                    "Cook cache 锁正在初始化或由未记录的进程持有：{}",
                    root.display()
                )));
            }
            // A lock without a live owner is recoverable after a crash.
            let _ = fs::remove_dir(&lock_path);
            let _ = fs::remove_file(lease_path(root));
            fs::create_dir(&lock_path)?;
        }
        Err(error) => return Err(error.into()),
    }
    let pid = sysinfo::get_current_pid()
        .map(|pid| pid.as_u32())
        .unwrap_or_default();
    let now = unix_seconds();
    let lease = CacheLease {
        pid,
        process_start_time: current_process_start_time(),
        host: std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".into()),
        execution_id: execution_id.into(),
        acquired_at: now,
        heartbeat_at: now,
    };
    let lease_path = lease_path(root);
    let write_result = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&lease_path)
        .and_then(|mut file| {
            serde_json::to_writer_pretty(&mut file, &lease).map_err(std::io::Error::other)
        });
    if let Err(error) = write_result {
        let _ = fs::remove_dir(&lock_path);
        return Err(error.into());
    }
    Ok(CacheLeaseGuard {
        lock_path,
        lease_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_ignores_profile_revision_name_output_and_reason() {
        let identity = CacheIdentity::new(
            "workspace/test",
            Path::new("C:/Project"),
            Path::new("C:/UE"),
            "Win64",
            "Shipping",
            "pak",
        );
        // Profile-only changes are intentionally not represented in the
        // identity.  Reconstructing the same identity must yield the same id.
        let same = CacheIdentity::new(
            "workspace/test",
            Path::new("C:/Project"),
            Path::new("C:/UE"),
            "Win64",
            "Shipping",
            "pak",
        );
        assert_eq!(identity.cache_id(), same.cache_id());
    }

    #[test]
    fn cache_key_separates_isolation_fields() {
        let base = CacheIdentity::new(
            "workspace/test",
            Path::new("C:/Project"),
            Path::new("C:/UE"),
            "Win64",
            "Shipping",
            "pak",
        );
        for changed in [
            CacheIdentity::new(
                "task/other",
                Path::new("C:/Project"),
                Path::new("C:/UE"),
                "Win64",
                "Shipping",
                "pak",
            ),
            CacheIdentity::new(
                "workspace/test",
                Path::new("C:/Other"),
                Path::new("C:/UE"),
                "Win64",
                "Shipping",
                "pak",
            ),
            CacheIdentity::new(
                "workspace/test",
                Path::new("C:/Project"),
                Path::new("C:/OtherUE"),
                "Win64",
                "Shipping",
                "pak",
            ),
            CacheIdentity::new(
                "workspace/test",
                Path::new("C:/Project"),
                Path::new("C:/UE"),
                "Linux",
                "Shipping",
                "pak",
            ),
            CacheIdentity::new(
                "workspace/test",
                Path::new("C:/Project"),
                Path::new("C:/UE"),
                "Win64",
                "Development",
                "pak",
            ),
            CacheIdentity::new(
                "workspace/test",
                Path::new("C:/Project"),
                Path::new("C:/UE"),
                "Win64",
                "Shipping",
                "iostore",
            ),
        ] {
            assert_ne!(base.cache_id(), changed.cache_id());
        }
    }

    #[test]
    fn active_cache_lease_is_never_a_cleanup_candidate() {
        let root = tempfile::tempdir().unwrap();
        let guard = acquire_lease(root.path(), "package-test").unwrap();
        let lease = read_lease(root.path()).unwrap();
        assert!(lease_is_active(&lease));
        assert!(active_lease(root.path()).is_some());
        drop(guard);
        assert!(active_lease(root.path()).is_none());
    }
}
