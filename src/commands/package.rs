use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::output;
use crate::package_cache::{self, CacheIdentity};
use crate::package_inventory::{self, InventoryFilter, PackageInventoryItem};
use crate::package_profile;
use crate::package_storage::StorageReport;
use crate::project_packaging::{self, PackageContainer as ProjectContainer};
use crate::ue_commands::{
    Configuration, EngineSourceBuildOptions, InstalledBuildOptions, NativePackageSettings,
    PackageContainer, ProjectPackageOptions, UbtMutexMode, UeCommand, UePlatform,
    engine_source_build_commands, installed_build_commands, project_package_commands,
};
use chrono::Utc;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::UNIX_EPOCH;

fn plugin_stage_root(execution_id: &str) -> PathBuf {
    let mut hasher = Md5::new();
    hasher.update(execution_id.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    std::env::temp_dir().join("UDF").join(&digest[..12])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CookCacheState {
    schema_version: u32,
    #[serde(default)]
    cache_id: String,
    #[serde(default)]
    task_uid: String,
    project: PathBuf,
    engine: PathBuf,
    #[serde(default = "default_platform")]
    platform: String,
    configuration: String,
    container: String,
    project_settings_digest: String,
    source_fingerprint: String,
    #[serde(default)]
    overlay_fingerprint: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    last_used_at: String,
    #[serde(default)]
    last_success_at: String,
    #[serde(default)]
    execution_id: String,
    #[serde(default)]
    producer_version: String,
}

fn default_platform() -> String {
    "Win64".into()
}

fn cook_cache_root(profile: &package_profile::PackageProfile) -> Result<PathBuf> {
    let identity = CacheIdentity::new(
        profile.task_uid.clone(),
        &profile.project,
        &profile.engine,
        "Win64",
        profile.configuration.clone(),
        container_name(match profile.container {
            package_profile::Container::Loose => PackageContainer::Loose,
            package_profile::Container::Pak => PackageContainer::Pak,
            package_profile::Container::Iostore => PackageContainer::Iostore,
        }),
    );
    Ok(identity.cache_root(&Config::config_dir()?))
}

fn legacy_cook_cache_root(profile: &package_profile::PackageProfile) -> Result<PathBuf> {
    let value = format!(
        "{}|{}|{}|{}|{:?}|{}|{}",
        profile.task_uid,
        profile.project.display(),
        profile.engine.display(),
        profile.configuration,
        profile.container,
        profile.name,
        profile.revision,
    );
    let digest = format!("{:x}", Md5::digest(value.as_bytes()));
    Ok(Config::config_dir()?
        .join("package")
        .join("cook-cache")
        .join(digest))
}

fn migrate_legacy_cache(
    profile: &package_profile::PackageProfile,
    stable_root: &Path,
) -> Result<()> {
    if stable_root.exists() {
        return Ok(());
    }
    let legacy_root = legacy_cook_cache_root(profile)?;
    if legacy_root == stable_root || !legacy_root.is_dir() {
        return Ok(());
    }
    let Some(state) = read_cache_state(&legacy_root) else {
        return Ok(());
    };
    if state.project != profile.project
        || state.engine != profile.engine
        || state.configuration != profile.configuration
        || state.container
            != container_name(match profile.container {
                package_profile::Container::Loose => PackageContainer::Loose,
                package_profile::Container::Pak => PackageContainer::Pak,
                package_profile::Container::Iostore => PackageContainer::Iostore,
            })
    {
        return Ok(());
    }
    if let Some(parent) = stable_root.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&legacy_root, stable_root)?;
    Ok(())
}

fn update_source_fingerprint(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    digest: &mut Md5,
) -> Result<()> {
    // `junction::exists` resolves the target on every call. On a large UE
    // project that turns a metadata walk into millions of reparse-point
    // probes. Read the directory entry once instead, and resolve only actual
    // Junction/symlink entries. This still handles dangling links through
    // `symlink_metadata`, which deliberately does not follow the target.
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        let target = crate::junction::get_target(path)?;
        let canonical = dunce::canonicalize(&target).unwrap_or(target.clone());
        if !visited.insert(canonical.clone()) {
            return Ok(());
        }
        digest.update(b"junction");
        digest.update(canonical.to_string_lossy().as_bytes());
        return update_source_fingerprint(&canonical, visited, digest);
    }
    digest.update(path.to_string_lossy().as_bytes());
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)?
            .flatten()
            .map(|entry| entry.path())
            .filter(|entry| {
                let name = entry
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default();
                !excluded_entry(&name) && !excluded_file(&name)
            })
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            update_source_fingerprint(&entry, visited, digest)?;
        }
    } else if metadata.is_file() {
        digest.update(metadata.len().to_le_bytes());
        if let Ok(modified) = metadata.modified()
            && let Ok(duration) = modified.duration_since(UNIX_EPOCH)
        {
            digest.update(duration.as_secs().to_le_bytes());
            digest.update(duration.subsec_nanos().to_le_bytes());
        }
    }
    Ok(())
}

fn source_fingerprint(project_root: &Path) -> Result<String> {
    let mut digest = Md5::new();
    let mut visited = HashSet::new();
    for name in [
        "Config", "Content", "Plugins", "Source", "Build", "Shaders", "Binaries",
    ] {
        let path = project_root.join(name);
        if path.exists() {
            update_source_fingerprint(&path, &mut visited, &mut digest)?;
        }
    }
    for project in fs::read_dir(project_root)?
        .flatten()
        .map(|entry| entry.path())
    {
        if project
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("uproject"))
        {
            update_source_fingerprint(&project, &mut visited, &mut digest)?;
        }
    }
    Ok(format!("md5:{:x}", digest.finalize()))
}

fn overlay_fingerprint(
    overlays: &[(String, PathBuf)],
    disabled_plugins: &[String],
) -> Result<String> {
    let mut digest = Md5::new();
    digest.update(b"overlay-schema=1");
    let mut disabled = disabled_plugins.to_vec();
    disabled.sort();
    for plugin in disabled {
        digest.update(b"disabled:");
        digest.update(plugin.as_bytes());
    }
    let mut entries = overlays.to_vec();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    for (name, path) in entries {
        digest.update(b"plugin:");
        digest.update(name.as_bytes());
        digest.update(path.to_string_lossy().as_bytes());
        if path.exists() {
            let mut visited = HashSet::new();
            update_source_fingerprint(&path, &mut visited, &mut digest)?;
        }
    }
    Ok(format!("md5:{:x}", digest.finalize()))
}

fn cache_state_path(root: &Path) -> PathBuf {
    root.join(".udf-cook-cache.json")
}

fn read_cache_state(root: &Path) -> Option<CookCacheState> {
    let path = cache_state_path(root);
    fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn write_cache_state(root: &Path, state: &CookCacheState) -> Result<()> {
    fs::write(cache_state_path(root), serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

fn container_name(container: PackageContainer) -> &'static str {
    match container {
        PackageContainer::Loose => "loose",
        PackageContainer::Pak => "pak",
        PackageContainer::Iostore => "iostore",
    }
}

fn is_managed_cleanup_target(path: &Path) -> bool {
    let config_dir = Config::config_dir().ok();
    let temp_dir = package_temp_dir();
    if let Some(config_dir) = config_dir
        && package_inventory::is_fixed_root_path(path, &config_dir, &temp_dir)
    {
        return true;
    }
    // Execution logs historically lived below a project Saved/UnrealDevFlow
    // directory.  The caller must have obtained this path from an execution
    // record; the suffix check only preserves compatibility for that exact
    // recorded root and no longer accepts arbitrary similarly named folders.
    let lower = path.to_string_lossy().to_ascii_lowercase();
    if !(lower.contains("\\saved\\unrealdevflow\\") || lower.contains("/saved/unrealdevflow/")) {
        return false;
    }
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with("package-")
    })
}

fn warning_diagnostics(logs: &[PathBuf]) -> Vec<String> {
    let mut total = 0usize;
    let mut categories = BTreeMap::<&'static str, usize>::new();
    for log in logs {
        let Ok(text) = fs::read_to_string(log) else {
            continue;
        };
        for line in text.lines() {
            let lower = line.to_ascii_lowercase();
            let is_warning = lower.contains("warning:")
                || lower.contains("warning c")
                || lower.contains("warning ")
                || lower.contains("deprecated");
            if !is_warning {
                continue;
            }
            total += 1;
            let category = if lower.contains("unable to find package for cooking") {
                "cook_missing_package"
            } else if lower.contains("missing file")
                || lower.contains("failed to find")
                || lower.contains("could not find")
            {
                "missing_file_or_dependency"
            } else if lower.contains("material")
                || lower.contains("shader")
                || lower.contains("niagara")
            {
                "material_or_shader"
            } else if lower.contains("deprecated") || lower.contains("warning c") {
                "deprecated_or_compiler"
            } else {
                "other"
            };
            *categories.entry(category).or_default() += 1;
        }
    }
    if total == 0 {
        return Vec::new();
    }
    let summary = categories
        .into_iter()
        .map(|(category, count)| format!("{category}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    vec![format!(
        "UE 日志包含 {total} 条警告（{summary}）；详见 logs 中的原始 UAT/Cook 日志"
    )]
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageMode {
    Run,
    Plan,
    Check,
}

impl PackageMode {
    fn executes(self) -> bool {
        self == Self::Run
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageCheckReport {
    domain: &'static str,
    action: String,
    source: String,
    readiness: String,
    checks: Vec<String>,
    diagnostics: Vec<String>,
    next_command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<StorageReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackagePlanStep {
    name: String,
    executable: String,
    argv: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackagePlanReport {
    domain: &'static str,
    action: String,
    source: String,
    plan_digest: String,
    steps: Vec<PackagePlanStep>,
    outputs: Vec<PathBuf>,
    diagnostics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_settings: Option<crate::project_packaging::ProjectPackagingSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<StorageReport>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageResult {
    execution_id: String,
    action: String,
    workspace: String,
    source: String,
    state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    commands: Vec<Vec<String>>,
    outputs: Vec<PathBuf>,
    #[serde(default)]
    artifacts: Vec<PathBuf>,
    logs: Vec<PathBuf>,
    #[serde(default)]
    manifests: Vec<PathBuf>,
    #[serde(default)]
    cleanup_targets: Vec<PathBuf>,
    #[serde(default)]
    diagnostics: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project_settings: Option<crate::project_packaging::ProjectPackagingSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cook_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cook_reused: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cook_reuse_reason: Option<String>,
    #[serde(flatten)]
    metadata: PackageMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageMetadata {
    #[serde(default)]
    target_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    profile_revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lineage_parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lineage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    task_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    storage: Option<StorageReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cleanup_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cleanup_result: Option<String>,
}

impl PackageMetadata {
    fn target(target_kind: &str) -> Self {
        Self {
            target_kind: target_kind.to_string(),
            ..Self::default()
        }
    }
}

#[derive(Clone, Default)]
struct PackageEvidence {
    project_settings: Option<crate::project_packaging::ProjectPackagingSnapshot>,
    cook_mode: Option<String>,
    cook_reused: Option<bool>,
    cook_reuse_reason: Option<String>,
    metadata: PackageMetadata,
}

fn check_package_commands(
    commands: &[UeCommand],
    mutex_project: Option<(&Path, &Path)>,
) -> (String, Vec<String>) {
    let mut missing = commands
        .iter()
        .map(|command| PathBuf::from(&command.executable))
        .filter(|path| !path.is_file())
        .collect::<Vec<_>>();
    missing.extend(
        commands
            .iter()
            .flat_map(|command| &command.arguments)
            .filter(|argument| {
                argument
                    .to_ascii_lowercase()
                    .ends_with("unrealbuildtool.dll")
            })
            .map(PathBuf::from)
            .filter(|path| !path.is_file()),
    );
    missing.extend(
        commands
            .iter()
            .flat_map(|command| {
                let argument = command
                    .arguments
                    .iter()
                    .find(|argument| argument.to_ascii_lowercase().starts_with("-script="))?;
                let script = PathBuf::from(argument.trim_start_matches("-script="));
                if script.is_absolute() {
                    Some(script)
                } else {
                    PathBuf::from(&command.executable)
                        .parent()
                        .and_then(Path::parent)
                        .and_then(Path::parent)
                        .and_then(Path::parent)
                        .map(|engine_root| engine_root.join(script))
                }
            })
            .filter(|path: &PathBuf| !path.is_file()),
    );
    missing.sort();
    missing.dedup();
    if !missing.is_empty() {
        return (
            "blocked".to_string(),
            missing
                .iter()
                .map(|path| format!("缺少可执行文件：{}", path.display()))
                .collect(),
        );
    }
    if let Some((project, engine_root)) = mutex_project {
        let report =
            crate::build_policy::resolve_build_policy(&crate::build_policy::BuildPolicyRequest {
                main_project: Some(project.to_string_lossy().to_string()),
                engine_root: Some(engine_root.to_string_lossy().to_string()),
                ..crate::build_policy::BuildPolicyRequest::default()
            });
        return (
            report.status.as_str().to_string(),
            std::iter::once(report.reason)
                .chain(report.diagnostics)
                .collect(),
        );
    }
    ("ready".to_string(), Vec::new())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    path: PathBuf,
    bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    digest: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeliveryEntry {
    relative: PathBuf,
    existed: bool,
    backup: Option<PathBuf>,
    old_digest: Option<String>,
    new_digest: String,
    #[serde(default)]
    removed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeliveryJournal {
    schema_version: u32,
    execution_id: String,
    state: String,
    output: PathBuf,
    backup_root: PathBuf,
    entries: Vec<DeliveryEntry>,
}

struct DeliveryLock(PathBuf);

impl Drop for DeliveryLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.0);
    }
}

fn acquire_delivery_lock(output: &Path) -> Result<DeliveryLock> {
    let lock_root = Config::config_dir()?.join("locks").join("package-output");
    fs::create_dir_all(&lock_root)?;
    let digest = format!("{:x}", Md5::digest(output.to_string_lossy().as_bytes()));
    let lock = lock_root.join(digest);
    fs::create_dir(&lock).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            UdfError::Other(format!(
                "固定输出目录正在被另一个 package 交付占用：{}",
                output.display()
            ))
        } else {
            error.into()
        }
    })?;
    Ok(DeliveryLock(lock))
}

fn file_digest(path: &Path) -> Result<String> {
    Ok(format!("md5:{:x}", Md5::digest(fs::read(path)?)))
}

fn safe_relative(path: &Path) -> Result<()> {
    if path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(UdfError::Other(format!(
            "交付清单包含越界相对路径：{}",
            path.display()
        )));
    }
    Ok(())
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if crate::junction::exists(&path).unwrap_or(false) {
            return Err(UdfError::Other(format!(
                "交付源包含 Junction，拒绝递归接管：{}",
                path.display()
            )));
        }
        if entry.file_type()?.is_dir() {
            collect_files(root, &path, files)?;
        } else if entry.file_type()?.is_file() {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

fn write_journal(path: &Path, journal: &DeliveryJournal) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(journal)?)?;
    Ok(())
}

fn publish_directory(
    source: &Path,
    output: &Path,
    execution_id: &str,
    log_dir: &Path,
) -> Result<()> {
    let _lock = acquire_delivery_lock(output)?;
    if crate::junction::exists(output).unwrap_or(false) {
        return Err(UdfError::Other(format!(
            "拒绝向 Junction 交付：{}",
            output.display()
        )));
    }
    if !source.is_dir() {
        return Err(UdfError::Other(format!(
            "交付源目录不存在：{}",
            source.display()
        )));
    }
    let backup_root = log_dir.join("delivery-backup");
    let journal_path = log_dir.join(".udf-delivery-journal.json");
    fs::create_dir_all(output)?;
    fs::create_dir_all(&backup_root)?;
    let previous_manifest = read_manifest_entries(output);
    let mut relative_files = Vec::new();
    collect_files(source, source, &mut relative_files)?;
    relative_files.sort();
    let mut entries = Vec::new();
    for relative in &relative_files {
        safe_relative(relative)?;
        let from = source.join(relative);
        let to = output.join(relative);
        if crate::junction::exists(&to).unwrap_or(false) {
            return Err(UdfError::Other(format!(
                "交付目标是受保护 Junction：{}",
                to.display()
            )));
        }
        let (existed, backup, old_digest) = if to.is_file() {
            if !previous_manifest.contains_key(relative) {
                return Err(UdfError::Other(format!(
                    "交付目标存在未由 UDF manifest 拥有的文件，拒绝覆盖：{}",
                    to.display()
                )));
            }
            let backup = backup_root.join(relative);
            if let Some(parent) = backup.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&to, &backup)?;
            (true, Some(backup), Some(file_digest(&to)?))
        } else if to.exists() {
            return Err(UdfError::Other(format!(
                "交付目标不是普通文件，拒绝覆盖：{}",
                to.display()
            )));
        } else {
            (false, None, None)
        };
        entries.push(DeliveryEntry {
            relative: relative.clone(),
            existed,
            backup,
            old_digest,
            new_digest: file_digest(&from)?,
            removed: false,
        });
    }
    for (relative, old_digest) in &previous_manifest {
        if relative == Path::new(".udf-manifest.json")
            || relative_files_contains(&relative_files, relative)
        {
            continue;
        }
        let target = output.join(relative);
        if !target.is_file() || file_digest(&target)? != *old_digest {
            continue;
        }
        let backup = backup_root.join(relative);
        if let Some(parent) = backup.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&target, &backup)?;
        entries.push(DeliveryEntry {
            relative: relative.clone(),
            existed: true,
            backup: Some(backup),
            old_digest: Some(old_digest.clone()),
            new_digest: String::new(),
            removed: true,
        });
    }
    let mut journal = DeliveryJournal {
        schema_version: 1,
        execution_id: execution_id.to_string(),
        state: "delivering".into(),
        output: output.to_path_buf(),
        backup_root,
        entries,
    };
    write_journal(&journal_path, &journal)?;
    for entry in &journal.entries {
        if entry.removed {
            continue;
        }
        let from = source.join(&entry.relative);
        let to = output.join(&entry.relative);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
    }
    for entry in &journal.entries {
        if entry.removed {
            let target = output.join(&entry.relative);
            if target.is_file() {
                fs::remove_file(target)?;
            }
        }
    }
    // A successful copy is not enough: verify every resulting file before
    // declaring the transaction delivered.  This closes the window where a
    // partial copy could be marked delivered and its only recovery backup
    // discarded.
    for entry in &journal.entries {
        let target = output.join(&entry.relative);
        if entry.removed {
            if target.exists() {
                return Err(UdfError::Other(format!(
                    "交付校验发现应删除文件仍存在：{}",
                    target.display()
                )));
            }
        } else if !target.is_file() || file_digest(&target)? != entry.new_digest {
            return Err(UdfError::Other(format!(
                "交付校验摘要不匹配：{}",
                target.display()
            )));
        }
    }
    journal.state = "delivered".into();
    write_journal(&journal_path, &journal)?;
    // Once the target digest is verified, the backup is no longer needed.
    // Keep the journal itself for audit/recover diagnostics; recover only
    // accepts `delivering`, so a delivered transaction cannot be rolled back
    // against a deliberately removed backup.
    remove_owned_tree(&journal.backup_root)?;
    Ok(())
}

fn relative_files_contains(files: &[PathBuf], path: &Path) -> bool {
    files.iter().any(|candidate| candidate == path)
}

fn read_manifest_entries(root: &Path) -> BTreeMap<PathBuf, String> {
    let path = root.join(".udf-manifest.json");
    let Ok(bytes) = fs::read(path) else {
        return BTreeMap::new();
    };
    let Ok(manifest) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return BTreeMap::new();
    };
    manifest
        .get("files")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let path = entry.get("path")?.as_str().map(PathBuf::from)?;
            if safe_relative(&path).is_err() {
                return None;
            }
            let digest = entry.get("digest")?.as_str()?.to_string();
            Some((path, digest))
        })
        .collect()
}

fn collect_manifest_entries(
    root: &Path,
    current: &Path,
    entries: &mut Vec<ManifestEntry>,
) -> Result<()> {
    if !current.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if crate::junction::exists(&path).unwrap_or(false) {
            // Manifests describe files owned by this directory only.
            continue;
        }
        if entry.file_type()?.is_dir() {
            collect_manifest_entries(root, &path, entries)?;
        } else if entry.file_type()?.is_file() {
            entries.push(ManifestEntry {
                path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                bytes: entry.metadata()?.len(),
                digest: Some(file_digest(&path)?),
            });
        }
    }
    Ok(())
}

fn write_manifest(root: &Path, reported_root: &Path) -> Result<PathBuf> {
    let mut files = Vec::new();
    collect_manifest_entries(root, root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let path = root.join(".udf-manifest.json");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": 1,
            "generatedAt": Utc::now().to_rfc3339(),
            "root": reported_root,
            "files": files,
        }))?,
    )?;
    Ok(path)
}

fn execution_id(action: &str) -> String {
    let now = Utc::now();
    format!(
        "package-{action}-{}-{:03}",
        now.format("%Y%m%dT%H%M%SZ"),
        now.timestamp_subsec_millis()
    )
}

fn log_dir_for_execution(project_root: &Path, execution_id: &str) -> PathBuf {
    project_root
        .join("Saved")
        .join("UnrealDevFlow")
        .join(execution_id)
}

fn project_file(project_dir: &Path) -> Result<PathBuf> {
    let mut projects = fs::read_dir(project_dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "uproject"))
        .collect::<Vec<_>>();
    projects.sort();
    match projects.as_slice() {
        [project] => Ok(project.clone()),
        [] => Err(UdfError::Other(format!(
            "项目目录中找不到 .uproject：{}",
            project_dir.display()
        ))),
        _ => Err(UdfError::Other(format!(
            "项目目录中有多个 .uproject，请先修正 workspace：{}",
            project_dir.display()
        ))),
    }
}

fn commands_as_argv(commands: &[UeCommand]) -> Vec<Vec<String>> {
    commands.iter().map(UeCommand::argv).collect()
}

fn emit_check(
    action: &str,
    source: &str,
    commands: &[UeCommand],
    mutex_project: Option<(&Path, &Path)>,
    next_command: String,
    additional_diagnostics: Vec<String>,
    storage: Option<StorageReport>,
) -> Result<()> {
    let (readiness, mut diagnostics) = check_package_commands(commands, mutex_project);
    if let Some(storage) = &storage {
        diagnostics.extend(storage.diagnostics.clone());
    }
    diagnostics.extend(additional_diagnostics);
    let readiness = if storage.as_ref().is_some_and(StorageReport::blocked)
        || diagnostics
            .iter()
            .any(|diagnostic| diagnostic.starts_with("broken junction"))
    {
        "blocked".to_string()
    } else {
        readiness
    };
    let checks = if diagnostics.is_empty() {
        vec!["toolchainAvailable".to_string()]
    } else {
        diagnostics
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .split('：')
                    .next()
                    .unwrap_or(diagnostic)
                    .to_string()
            })
            .collect()
    };
    let report = PackageCheckReport {
        domain: "package",
        action: action.to_string(),
        source: source.to_string(),
        readiness,
        checks,
        diagnostics,
        next_command,
        storage,
    };
    output::emit("package check", report, |report| {
        format!("package {}: {}", report.action, report.readiness)
    });
    Ok(())
}

fn validate_project_links(root: &Path) -> Result<Vec<String>> {
    let mut broken = Vec::new();
    fn visit(current: &Path, broken: &mut Vec<String>) -> Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if excluded_entry(&name) {
                continue;
            }
            let path = entry.path();
            if crate::junction::exists(&path).unwrap_or(false) {
                let target = crate::junction::get_target(&path)?;
                if !target.exists() {
                    broken.push(format!(
                        "broken junction：{} -> {}",
                        path.display(),
                        target.display()
                    ));
                }
            }
        }
        Ok(())
    }
    // A full Content scan is prohibitively expensive on real projects. The
    // execution copier remains authoritative; preflight checks the project
    // root and the immediate children of the two usual external-data roots.
    visit(root, &mut broken)?;
    for child in ["Content", "Plugins"] {
        let path = root.join(child);
        if path.is_dir() && !crate::junction::exists(&path).unwrap_or(false) {
            visit(&path, &mut broken)?;
        }
    }
    Ok(broken)
}

fn emit_plan(
    action: &str,
    source: &str,
    commands: &[UeCommand],
    outputs: Vec<PathBuf>,
    diagnostics: Vec<String>,
    project_settings: Option<crate::project_packaging::ProjectPackagingSnapshot>,
    storage: Option<StorageReport>,
) -> Result<()> {
    let steps = commands
        .iter()
        .enumerate()
        .map(|(index, command)| PackagePlanStep {
            name: format!("step-{:02}", index + 1),
            executable: command.executable.clone(),
            argv: command.argv(),
        })
        .collect::<Vec<_>>();
    let normalized = serde_json::to_vec(&(&steps, &outputs, &diagnostics, &project_settings))?;
    let plan_digest = format!("md5:{:x}", Md5::digest(normalized));
    let report = PackagePlanReport {
        domain: "package",
        action: action.to_string(),
        source: source.to_string(),
        plan_digest,
        steps,
        outputs,
        diagnostics,
        project_settings,
        storage,
    };
    output::emit("package plan", report, |report| {
        format!("package {} plan: {}", report.action, report.plan_digest)
    });
    Ok(())
}

fn render(result: &PackageResult) -> String {
    let mut lines = vec![format!(
        "package {}: {} ({})",
        result.action, result.state, result.execution_id
    )];
    for command in &result.commands {
        lines.push(format!("  {}", command.join(" ")));
    }
    for output in &result.outputs {
        lines.push(format!("  输出：{}", output.display()));
    }
    if let Some(storage) = &result.metadata.storage {
        lines.push(format!(
            "  空间：{}（输出 {} bytes，临时 {} bytes，预计新增 {} bytes）",
            storage.decision,
            storage.current_output_bytes,
            storage.current_temp_bytes,
            storage.estimated_additional_bytes
        ));
    }
    lines.extend(
        result
            .diagnostics
            .iter()
            .map(|diagnostic| format!("  诊断：{diagnostic}")),
    );
    lines.join("\n")
}

fn execution_root() -> Result<PathBuf> {
    Ok(Config::config_dir()?.join("executions").join("package"))
}

fn package_temp_dir() -> PathBuf {
    std::env::var_os("UNREALDEVFLOW_TEMP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn save_result(result: &PackageResult) -> Result<()> {
    let root = execution_root()?;
    fs::create_dir_all(&root)?;
    fs::write(
        root.join(format!("{}.json", result.execution_id)),
        serde_json::to_vec_pretty(result)?,
    )?;
    fs::write(root.join("latest"), result.execution_id.as_bytes())?;
    Ok(())
}

fn finish_execution(command: &str, result: PackageResult) -> Result<()> {
    save_result(&result)?;
    output::emit(command, result, render);
    Ok(())
}

fn run_commands(commands: &[UeCommand], log_dir: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(log_dir)?;
    let mut logs = Vec::new();
    for (index, command) in commands.iter().enumerate() {
        let log = log_dir.join(format!("step-{:02}.log", index + 1));
        let stdout = fs::File::create(&log)?;
        let stderr = stdout.try_clone()?;
        let status = Command::new(&command.executable)
            .args(&command.arguments)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .status()
            .map_err(|error| {
                UdfError::Other(format!("无法启动 {}：{}", command.executable, error))
            })?;
        logs.push(log.clone());
        if !status.success() {
            return Err(UdfError::Other(format!(
                "package 第 {} 步失败，退出码 {:?}。日志：{}",
                index + 1,
                status.code(),
                log.display()
            )));
        }
    }
    Ok(logs)
}

#[allow(clippy::too_many_arguments)]
fn run_package_commands(
    commands: &[UeCommand],
    log_dir: &Path,
    execution_id: &str,
    action: &str,
    workspace: &str,
    source: &str,
    outputs: Vec<PathBuf>,
    cleanup_targets: Vec<PathBuf>,
    evidence: PackageEvidence,
) -> Result<Vec<PathBuf>> {
    save_result(&PackageResult {
        execution_id: execution_id.to_string(),
        action: action.to_string(),
        workspace: workspace.to_string(),
        source: source.to_string(),
        state: "running".to_string(),
        exit_code: None,
        commands: commands_as_argv(commands),
        outputs: outputs.clone(),
        artifacts: outputs.clone(),
        logs: Vec::new(),
        manifests: Vec::new(),
        cleanup_targets: cleanup_targets.clone(),
        diagnostics: Vec::new(),
        project_settings: evidence.project_settings.clone(),
        cook_mode: evidence.cook_mode.clone(),
        cook_reused: evidence.cook_reused,
        cook_reuse_reason: evidence.cook_reuse_reason.clone(),
        metadata: evidence.metadata.clone(),
    })?;
    match run_commands(commands, log_dir) {
        Ok(logs) => Ok(logs),
        Err(error) => {
            let mut logs = fs::read_dir(log_dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "log"))
                .collect::<Vec<_>>();
            logs.sort();
            let mut diagnostics = vec![error.to_string()];
            diagnostics.extend(warning_diagnostics(&logs));
            save_result(&PackageResult {
                execution_id: execution_id.to_string(),
                action: action.to_string(),
                workspace: workspace.to_string(),
                source: source.to_string(),
                state: "failed".to_string(),
                exit_code: extract_exit_code(&error.to_string()),
                commands: commands_as_argv(commands),
                artifacts: outputs.clone(),
                outputs,
                logs,
                manifests: Vec::new(),
                cleanup_targets,
                diagnostics,
                project_settings: evidence.project_settings,
                cook_mode: evidence.cook_mode,
                cook_reused: evidence.cook_reused,
                cook_reuse_reason: evidence.cook_reuse_reason,
                metadata: evidence.metadata,
            })?;
            Err(UdfError::Other(format!(
                "{}（execution ID: {}）",
                error, execution_id
            )))
        }
    }
}

fn engine_failure_cleanup_targets(
    output_dir: &Path,
    log_dir: &Path,
    output_preexisted: bool,
) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    if !output_preexisted {
        targets.push(output_dir.to_path_buf());
    }
    targets.push(log_dir.to_path_buf());
    targets
}

fn extract_exit_code(message: &str) -> Option<i32> {
    let start = message.find("Some(")? + "Some(".len();
    let end = message[start..].find(')')? + start;
    message[start..end].parse().ok()
}

fn collect_plugin_descriptors(
    root: &Path,
    relative: &Path,
    descriptors: &mut BTreeMap<String, Vec<PathBuf>>,
) -> Result<()> {
    let directory = root.join(relative);
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let entry_name = entry.file_name().to_string_lossy().to_string();
        if matches!(
            entry_name.to_ascii_lowercase().as_str(),
            ".git" | "binaries" | "content" | "config" | "intermediate" | "saved" | "source"
        ) {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() && crate::junction::exists(&path).unwrap_or(false) {
            // Discovery must never recurse through project or task links.
            // The selected source is staged explicitly after discovery.
            continue;
        }
        if file_type.is_dir() {
            let direct_descriptor = path.join(format!("{entry_name}.uplugin"));
            if direct_descriptor.is_file() {
                descriptors
                    .entry(entry_name)
                    .or_default()
                    .push(relative.join(entry.file_name()));
            } else {
                collect_plugin_descriptors(root, &relative.join(entry.file_name()), descriptors)?;
            }
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "uplugin")
        {
            let name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| UdfError::Other(format!("无效插件描述文件：{}", path.display())))?
                .to_string();
            let plugin_dir = path
                .parent()
                .and_then(|parent| parent.strip_prefix(root).ok())
                .ok_or_else(|| {
                    UdfError::Other(format!("插件不在插件根目录内：{}", path.display()))
                })?
                .to_path_buf();
            descriptors.entry(name).or_default().push(plugin_dir);
        }
    }
    Ok(())
}

fn plugin_index(plugins_root: &Path) -> Result<BTreeMap<String, Vec<PathBuf>>> {
    let mut descriptors = BTreeMap::new();
    collect_plugin_descriptors(plugins_root, Path::new(""), &mut descriptors)?;
    Ok(descriptors)
}

fn resolve_plugin_seeds(
    plugins_root: &Path,
    requested: &[String],
    index: &BTreeMap<String, Vec<PathBuf>>,
) -> Result<(Vec<String>, BTreeMap<String, PathBuf>)> {
    let mut resolved = BTreeSet::new();
    let mut selected = BTreeMap::new();
    for request in requested {
        let direct = plugins_root
            .join(request)
            .join(format!("{request}.uplugin"));
        if direct.is_file() {
            resolved.insert(request.clone());
            selected.insert(request.clone(), PathBuf::from(request));
            continue;
        }
        let collection = plugins_root.join(request);
        let prefix = Path::new(request);
        let mut members = Vec::new();
        for (name, candidates) in index {
            let matches = candidates
                .iter()
                .filter(|relative| relative.starts_with(prefix))
                .collect::<Vec<_>>();
            if matches.len() > 1 {
                return Err(ambiguous_plugin_error(name, &matches));
            }
            if let Some(relative) = matches.first() {
                members.push(name.clone());
                selected.insert(name.clone(), (*relative).clone());
            }
        }
        if !collection.is_dir() || members.is_empty() {
            return Err(UdfError::Other(format!(
                "找不到插件或插件集合：{}",
                collection.display()
            )));
        }
        resolved.extend(members);
    }
    Ok((resolved.into_iter().collect(), selected))
}

fn ambiguous_plugin_error(name: &str, candidates: &[&PathBuf]) -> UdfError {
    let locations = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" 与 ");
    UdfError::Other(format!("插件集合中存在重名插件 '{}'：{}", name, locations))
}

fn plugin_closure(
    plugins_root: &Path,
    seeds: &[String],
    index: &BTreeMap<String, Vec<PathBuf>>,
    selected: &mut BTreeMap<String, PathBuf>,
) -> Result<Vec<String>> {
    let mut closure = BTreeSet::new();
    let mut pending = seeds.to_vec();
    while let Some(name) = pending.pop() {
        if !closure.insert(name.clone()) {
            continue;
        }
        let Some(relative_dir) = selected.get(&name) else {
            // Dependencies supplied by the engine are not staged.
            continue;
        };
        let plugin_dir = plugins_root.join(relative_dir);
        for dependency in crate::plugin::uplugin::read_dependencies(&plugin_dir)? {
            if let Some(candidates) = index.get(&dependency) {
                if candidates.len() > 1 {
                    return Err(ambiguous_plugin_error(
                        &dependency,
                        &candidates.iter().collect::<Vec<_>>(),
                    ));
                }
                if let Some(relative) = candidates.first() {
                    selected.insert(dependency.clone(), relative.clone());
                    pending.push(dependency);
                }
            }
        }
    }
    Ok(closure.into_iter().collect())
}

fn excluded_entry(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git"
            | "workflow"
            | "intermediate"
            | "saved"
            | "deriveddatacache"
            | ".claude"
            | ".codex"
            | ".omx"
            | ".sisyphus"
            | ".idea"
            | ".junie"
            | ".vs"
            | ".vscode"
            | "nul"
    )
}

fn copy_tree(source: &Path, destination: &Path, include_source: bool) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if excluded_entry(&name_text)
            || excluded_file(&name_text)
            || (!include_source && name_text.eq_ignore_ascii_case("Source"))
        {
            continue;
        }
        let source_path = entry.path();
        let destination_path = destination.join(&name);
        if crate::junction::exists(&source_path).unwrap_or(false) {
            let target = crate::junction::get_target(&source_path)?;
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            crate::junction::create(&target, &destination_path).map_err(|error| {
                UdfError::Other(format!(
                    "创建 staging Junction 失败：{} -> {}：{}",
                    destination_path.display(),
                    target.display(),
                    error
                ))
            })?;
            continue;
        }
        if entry.file_type()?.is_dir() {
            copy_tree(&source_path, &destination_path, include_source)?;
        } else if entry.file_type()?.is_file() {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &destination_path).map_err(|error| {
                UdfError::Other(format!(
                    "复制 staging 文件失败：{} -> {}：{}",
                    source_path.display(),
                    destination_path.display(),
                    error
                ))
            })?;
        }
    }
    Ok(())
}

fn copy_tree_entry(source: &Path, destination: &Path, include_source: bool) -> Result<()> {
    if crate::junction::exists(source).unwrap_or(false) {
        let target = crate::junction::get_target(source)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        crate::junction::create(&target, destination).map_err(|error| {
            UdfError::Other(format!(
                "创建 staging Junction 失败：{} -> {}：{}",
                destination.display(),
                target.display(),
                error
            ))
        })?;
        return Ok(());
    }
    if source.is_dir() {
        copy_tree(source, destination, include_source)
    } else if source.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, destination).map_err(|error| {
            UdfError::Other(format!(
                "复制 staging 文件失败：{} -> {}：{}",
                source.display(),
                destination.display(),
                error
            ))
        })?;
        Ok(())
    } else {
        Err(UdfError::Other(format!(
            "staging 输入不存在：{}",
            source.display()
        )))
    }
}

fn remove_owned_tree(path: &Path) -> Result<()> {
    if crate::junction::exists(path).unwrap_or(false) {
        return crate::junction::delete(path);
    }
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            remove_owned_tree(&entry?.path())?;
        }
        fs::remove_dir(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn excluded_file(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".gitattributes" | ".gitignore" | ".gitmodules" | ".p4config" | ".p4ignore"
    )
}

fn copy_project_inputs(
    source_root: &Path,
    destination: &Path,
    preserve_plugins_root: bool,
) -> Result<()> {
    fs::create_dir_all(destination)?;
    for name in [
        "Config", "Content", "Plugins", "Source", "Build", "Shaders", "Binaries",
    ] {
        let source = source_root.join(name);
        if source.exists() {
            if name == "Plugins" && preserve_plugins_root {
                crate::junction::create(&source, &destination.join(name)).map_err(|error| {
                    UdfError::Other(format!(
                        "创建 staging Plugins Junction 失败：{} -> {}：{}",
                        destination.join(name).display(),
                        source.display(),
                        error
                    ))
                })?;
                continue;
            }
            copy_tree_entry(&source, &destination.join(name), true)?;
        }
    }
    for entry in fs::read_dir(source_root)? {
        let entry = entry?;
        if entry
            .path()
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("uproject"))
        {
            fs::copy(entry.path(), destination.join(entry.file_name())).map_err(|error| {
                UdfError::Other(format!(
                    "复制 staging 项目文件失败：{} -> {}：{}",
                    entry.path().display(),
                    destination.join(entry.file_name()).display(),
                    error
                ))
            })?;
        }
    }
    Ok(())
}

fn prepare_project_stage(
    project: &Path,
    stage_root: &Path,
    disabled_plugins: &[String],
    plugin_overlays: &[(String, PathBuf)],
) -> Result<PathBuf> {
    if crate::junction::exists(stage_root).unwrap_or(false) {
        return Err(UdfError::Other(format!(
            "拒绝覆盖受管副本路径上的 Junction：{}",
            stage_root.display()
        )));
    }
    if stage_root.exists() {
        // The cache lease lives beside the staged project.  Preserve the
        // lock/lease files while replacing the disposable Cook inputs so a
        // concurrent cleaner cannot observe an unlocked half-built slot.
        for entry in fs::read_dir(stage_root)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name == ".udf-cook-cache.lock" || name == ".udf-cook-cache-lease.json" {
                continue;
            }
            remove_owned_tree(&entry.path())?;
        }
    }
    let source_root = project
        .parent()
        .ok_or_else(|| UdfError::Other(format!("项目路径没有父目录：{}", project.display())))?;
    copy_project_inputs(source_root, stage_root, plugin_overlays.is_empty())?;
    let staged_project = stage_root.join(
        project
            .file_name()
            .ok_or_else(|| UdfError::Other("项目文件名为空".into()))?,
    );
    let mut descriptor: serde_json::Value = serde_json::from_slice(&fs::read(&staged_project)?)?;
    if let Some(plugins) = descriptor
        .get_mut("Plugins")
        .and_then(serde_json::Value::as_array_mut)
    {
        for plugin in plugins {
            let Some(name) = plugin.get("Name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if disabled_plugins
                .iter()
                .any(|disabled| disabled.eq_ignore_ascii_case(name))
            {
                plugin["Enabled"] = serde_json::Value::Bool(false);
            }
        }
    }
    fs::write(&staged_project, serde_json::to_vec_pretty(&descriptor)?)?;
    for (name, source) in plugin_overlays {
        let destination = stage_root.join("Plugins").join(name);
        if crate::junction::exists(&destination).unwrap_or(false) {
            crate::junction::delete(&destination)?;
        } else if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        copy_tree(source, &destination, true)?;
    }
    Ok(staged_project)
}

fn prepare_plugin_stage(
    plugins_root: &Path,
    stage_root: &Path,
    closure: &[String],
    index: &BTreeMap<String, PathBuf>,
) -> Result<()> {
    if stage_root.exists() {
        remove_owned_tree(stage_root)?;
    }
    fs::create_dir_all(stage_root.join("Plugins"))?;
    for plugin in closure {
        if let Some(relative_dir) = index.get(plugin) {
            let source = plugins_root.join(relative_dir);
            prepare_private_plugin_root(&source, &stage_root.join("Plugins").join(relative_dir))?;
        }
    }
    let enabled = closure
        .iter()
        .map(|name| serde_json::json!({"Name": name, "Enabled": true}))
        .collect::<Vec<_>>();
    fs::write(
        stage_root.join("HostProject.uproject"),
        serde_json::to_vec_pretty(&serde_json::json!({"FileVersion": 3, "Plugins": enabled}))?,
    )?;
    Ok(())
}

fn is_private_plugin_stage_directory(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "intermediate" | "binaries" | "saved"
    )
}

fn stage_read_only_directory(source: &Path, destination: &Path) -> Result<()> {
    let target = if crate::junction::exists(source).unwrap_or(false) {
        crate::junction::get_target(source)?
    } else {
        source.to_path_buf()
    };
    crate::junction::create(&target, destination).map_err(|error| {
        UdfError::Other(format!(
            "创建插件 staging 只读 Junction 失败：{} -> {}：{}",
            destination.display(),
            target.display(),
            error
        ))
    })
}

fn copy_private_plugin_tree(
    source: &Path,
    destination: &Path,
    visiting: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    let resolved = if crate::junction::exists(source).unwrap_or(false) {
        crate::junction::get_target(source)?
    } else {
        source.to_path_buf()
    };
    let canonical = dunce::canonicalize(&resolved).unwrap_or(resolved.clone());
    if !visiting.insert(canonical.clone()) {
        return Err(UdfError::Other(format!(
            "插件 Binaries 包含循环 Junction：{}",
            resolved.display()
        )));
    }
    let result = if resolved.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(&resolved)? {
            let entry = entry?;
            copy_private_plugin_tree(
                &entry.path(),
                &destination.join(entry.file_name()),
                visiting,
            )?;
        }
        Ok(())
    } else if resolved.is_file() {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&resolved, destination).map_err(|error| {
            UdfError::Other(format!(
                "复制私有插件 Binaries 文件失败：{} -> {}：{}",
                resolved.display(),
                destination.display(),
                error
            ))
        })?;
        Ok(())
    } else {
        Err(UdfError::Other(format!(
            "插件 Binaries 输入不存在：{}",
            resolved.display()
        )))
    };
    visiting.remove(&canonical);
    result
}

/// Build a plugin root that gives UBT private generated directories without
/// copying Source, Content, or third-party data for every package execution.
fn prepare_private_plugin_root(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        let source_path = entry.path();
        let destination_path = destination.join(&name);
        if is_private_plugin_stage_directory(&name_text) {
            if name_text.eq_ignore_ascii_case("Binaries") && source_path.is_dir() {
                // Precompiled third-party DLLs can live here. Keep a private
                // copy so UBT may write its outputs without touching source.
                copy_private_plugin_tree(&source_path, &destination_path, &mut BTreeSet::new())?;
            } else {
                fs::create_dir_all(&destination_path)?;
            }
            continue;
        }
        if excluded_entry(&name_text) || excluded_file(&name_text) {
            continue;
        }
        if entry.file_type()?.is_dir() {
            stage_read_only_directory(&source_path, &destination_path)?;
        } else if entry.file_type()?.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                UdfError::Other(format!(
                    "复制插件 staging 根文件失败：{} -> {}：{}",
                    source_path.display(),
                    destination_path.display(),
                    error
                ))
            })?;
        }
    }
    for name in ["Intermediate", "Binaries", "Saved"] {
        fs::create_dir_all(destination.join(name))?;
    }
    Ok(())
}

fn cleanup_plugin_stage_after_success(
    stage_root: &Path,
    cleanup_targets: &mut Vec<PathBuf>,
    diagnostics: &mut Vec<String>,
) -> String {
    match remove_owned_tree(stage_root) {
        Ok(()) => {
            cleanup_targets.retain(|target| target != stage_root);
            "succeeded".into()
        }
        Err(error) => {
            diagnostics.push(format!("插件交付成功，但 staging 自动清理失败：{error}"));
            "warning".into()
        }
    }
}

fn direct_plugin_commands(
    engine_root: &Path,
    stage_root: &Path,
    seeds: &[String],
    index: &BTreeMap<String, PathBuf>,
) -> Vec<UeCommand> {
    let dotnet = engine_root.join("Engine/Binaries/ThirdParty/DotNet/8.0.300/win-x64/dotnet.exe");
    let ubt = engine_root.join("Engine/Binaries/DotNET/UnrealBuildTool/UnrealBuildTool.dll");
    let project = stage_root.join("HostProject.uproject");
    let mut commands = Vec::new();
    for seed in seeds {
        let relative_dir = index.get(seed).expect("resolved plugin seed");
        let plugin = stage_root
            .join("Plugins")
            .join(relative_dir)
            .join(format!("{seed}.uplugin"));
        for (target, configuration) in [
            ("UnrealEditor", "Development"),
            ("UnrealGame", "Development"),
            ("UnrealGame", "Shipping"),
        ] {
            commands.push(UeCommand::new(
                dotnet.to_string_lossy(),
                vec![
                    ubt.to_string_lossy().to_string(),
                    target.to_string(),
                    "Win64".to_string(),
                    configuration.to_string(),
                    format!("-Project={}", project.display()),
                    format!("-plugin={}", plugin.display()),
                    "-nohotreload".to_string(),
                    // Plugin packaging compiles from a private staging project whose
                    // project/plugin intermediates are isolated per execution. UEB's
                    // dependency-aware path therefore does not take the engine-global
                    // UBT mutex and must not block normal Host builds.
                    "-NoMutex".to_string(),
                    format!(
                        "-log={}",
                        stage_root
                            .join("Saved/Logs")
                            .join(format!("UBT-{seed}-{target}-{configuration}.log"))
                            .display()
                    ),
                ],
            ));
        }
    }
    commands
}

pub fn project(workspace: Option<String>, task: Option<String>, mode: PackageMode) -> Result<()> {
    let config = Config::load()?;
    let requested_task = task.clone();
    let (name, project_root, engine_root, source) = if let Some(task_ref) = task {
        let (host_dir, _, context) = crate::host::resolve_task(&config, &task_ref)?;
        (context.workspace, host_dir, context.engine_path, "task")
    } else {
        let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
        (
            name,
            workspace_config.default_project,
            workspace_config.engine_path,
            "workspace",
        )
    };
    let saved_profile = package_profile::load_for_project(
        &config,
        workspace.as_deref(),
        requested_task.as_deref(),
    )?;
    let project = project_file(
        saved_profile
            .as_ref()
            .map(|p| &p.project)
            .unwrap_or(&project_root),
    )?;
    let source_project = project.clone();
    let project_settings =
        project_packaging::load_snapshot(project.parent().ok_or_else(|| {
            UdfError::Other(format!("项目路径没有父目录：{}", project.display()))
        })?)?;
    let archive_dir = saved_profile
        .as_ref()
        .map(|p| p.output.clone())
        .unwrap_or_else(|| project_root.join("Saved/UnrealDevFlow/Packages/Win64"));
    let engine_root = saved_profile
        .as_ref()
        .map(|p| p.engine.clone())
        .unwrap_or(engine_root);
    let configuration = saved_profile
        .as_ref()
        .map(|p| Configuration::parse(&p.configuration))
        .or_else(|| {
            project_settings
                .packaging
                .configuration
                .as_deref()
                .map(Configuration::parse)
        })
        .unwrap_or(Configuration::Development);
    let container = saved_profile
        .as_ref()
        .map(|p| match p.container {
            package_profile::Container::Loose => PackageContainer::Loose,
            package_profile::Container::Pak => PackageContainer::Pak,
            package_profile::Container::Iostore => PackageContainer::Iostore,
        })
        .or(Some(match project_settings.packaging.container {
            ProjectContainer::Loose => PackageContainer::Loose,
            ProjectContainer::Pak => PackageContainer::Pak,
            ProjectContainer::Iostore => PackageContainer::Iostore,
        }))
        .unwrap_or(PackageContainer::Pak);
    let configuration_name = configuration.as_unreal_value().to_string();
    let execution_id = execution_id("project");
    let profile_output = saved_profile.is_some();
    let requested_iterate = saved_profile
        .as_ref()
        .is_some_and(|profile| profile.cook_mode == package_profile::CookMode::Iterate);
    let persistent_stage_root = if requested_iterate {
        Some(cook_cache_root(
            saved_profile.as_ref().expect("iterate requires profile"),
        )?)
    } else {
        None
    };
    if mode.executes() && requested_iterate {
        migrate_legacy_cache(
            saved_profile.as_ref().expect("iterate requires profile"),
            persistent_stage_root.as_ref().expect("iterate cache root"),
        )?;
    }
    let overlays = if let Some(task_ref) = requested_task.as_deref() {
        let (host_dir, meta, _) = crate::host::resolve_task(&config, task_ref)?;
        meta.primary_plugins
            .iter()
            .map(|plugin| (plugin.name.clone(), host_dir.join(&plugin.worktree)))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let current_source_fingerprint = if requested_iterate && mode != PackageMode::Check {
        Some(source_fingerprint(project.parent().ok_or_else(|| {
            UdfError::Other(format!("项目路径没有父目录：{}", project.display()))
        })?)?)
    } else {
        None
    };
    let current_overlay_fingerprint = if requested_iterate && mode != PackageMode::Check {
        Some(overlay_fingerprint(
            &overlays,
            &saved_profile
                .as_ref()
                .map(|profile| profile.disabled_plugins.clone())
                .unwrap_or_default(),
        )?)
    } else {
        None
    };
    let _cache_lease = if mode.executes() && requested_iterate {
        Some(package_cache::acquire_lease(
            persistent_stage_root.as_ref().expect("iterate cache root"),
            execution_id.clone(),
        )?)
    } else {
        None
    };
    let can_iterate = requested_iterate
        && persistent_stage_root
            .as_ref()
            .and_then(|root| read_cache_state(root).map(|state| (root, state)))
            .is_some_and(|(root, state)| {
                root.is_dir()
                    && current_source_fingerprint.as_deref()
                        == Some(state.source_fingerprint.as_str())
                    && state.project_settings_digest == project_settings.digest
                    && state.project == project
                    && state.engine == engine_root
                    && state.configuration == configuration_name.as_str()
                    && state.container == container_name(container)
                    && current_overlay_fingerprint.as_deref()
                        == Some(state.overlay_fingerprint.as_str())
            });
    let execution_stage_root = persistent_stage_root
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("UDF").join(&execution_id));
    if mode.executes() {
        let input_diagnostics = project
            .parent()
            .map(validate_project_links)
            .transpose()?
            .unwrap_or_default();
        if !input_diagnostics.is_empty() {
            return Err(UdfError::Other(input_diagnostics.join("；")));
        }
    }
    let staged_project = if mode.executes() {
        if let Some(profile) = saved_profile.as_ref() {
            if can_iterate {
                let archive = execution_stage_root.join("Archive");
                if archive.exists() {
                    remove_owned_tree(&archive)?;
                }
                Some(
                    execution_stage_root.join(
                        project
                            .file_name()
                            .ok_or_else(|| UdfError::Other("项目文件名为空".into()))?,
                    ),
                )
            } else {
                Some(prepare_project_stage(
                    &project,
                    &execution_stage_root,
                    &profile.disabled_plugins,
                    &overlays,
                )?)
            }
        } else {
            None
        }
    } else {
        None
    };
    let project = staged_project.unwrap_or(project);
    let execution_archive_dir = if profile_output {
        execution_stage_root.join("Archive")
    } else {
        archive_dir.clone()
    };
    let commands = project_package_commands(&ProjectPackageOptions {
        engine_root: engine_root.clone(),
        project: project.clone(),
        archive_dir: if mode == PackageMode::Plan {
            archive_dir.clone()
        } else {
            execution_archive_dir.clone()
        },
        platform: UePlatform::Windows,
        configuration,
        mutex: UbtMutexMode::Wait,
        package_args: None,
        container,
        clean: false,
        native_settings: Some(NativePackageSettings {
            build: project_settings.packaging.build.clone(),
            full_rebuild: project_settings.packaging.full_rebuild,
            include_debug_files: project_settings.packaging.include_debug_files,
            cook_all: project_settings.packaging.cook_all,
            cook_maps_only: project_settings.packaging.cook_maps_only,
            skip_editor_content: project_settings.packaging.skip_editor_content,
            compressed: project_settings.packaging.compressed,
            include_prerequisites: project_settings.packaging.include_prerequisites,
            use_zen_store: project_settings.packaging.use_zen_store,
            maps_to_cook: project_settings.maps.maps_to_cook.clone(),
        }),
        iterate: can_iterate,
    });
    let log_dir = log_dir_for_execution(&project_root, &execution_id);
    let mut cleanup_targets = if profile_output {
        if requested_iterate {
            // Archive is a disposable delivery staging area, not Cook state.
            // Keep it in the execution ledger only until digest verification
            // and publish complete.
            vec![log_dir.clone(), execution_archive_dir.clone()]
        } else {
            vec![log_dir.clone(), execution_stage_root.clone()]
        }
    } else {
        // The final package is never a clean target. It is user-facing output,
        // even when it was produced without a saved profile.
        vec![log_dir.clone()]
    };
    let storage = crate::package_storage::assess_global(
        &archive_dir,
        &[execution_stage_root.clone(), log_dir.clone()],
        Some(crate::package_storage::source_input_estimate(
            source_project
                .parent()
                .ok_or_else(|| UdfError::Other("项目文件没有父目录".into()))?,
        )),
    );
    if mode.executes() && storage.blocked() {
        return Err(UdfError::Other(format!(
            "package project 空间预检阻止执行：{}",
            storage.diagnostics.join("；")
        )));
    }
    if mode == PackageMode::Check {
        let input_diagnostics = project
            .parent()
            .map(validate_project_links)
            .transpose()?
            .unwrap_or_default();
        let next = requested_task
            .map(|task| format!("udf package project --task {task}"))
            .unwrap_or_else(|| format!("udf package project --workspace {name}"));
        return emit_check(
            "project",
            source,
            &commands,
            Some((&project, &engine_root)),
            next,
            input_diagnostics,
            Some(storage.clone()),
        );
    }
    if mode == PackageMode::Plan {
        let (_, mut diagnostics) = check_package_commands(&commands, None);
        diagnostics.extend(
            project
                .parent()
                .map(validate_project_links)
                .transpose()?
                .unwrap_or_default(),
        );
        diagnostics.extend(storage.diagnostics.clone());
        return emit_plan(
            "project",
            source,
            &commands,
            vec![archive_dir],
            diagnostics,
            Some(project_settings),
            Some(storage.clone()),
        );
    }
    let id = execution_id;
    let cook_mode_name = if requested_iterate { "iterate" } else { "full" };
    let cook_reuse_reason = if requested_iterate && can_iterate {
        "持久化 Cook 状态与项目来源摘要匹配".to_string()
    } else if requested_iterate {
        "没有匹配的持久化 Cook 状态，已回退完整 Cook".to_string()
    } else {
        "默认遵循 UE Package Project 的完整 By-the-book Cook".to_string()
    };
    let logs = if mode.executes() {
        run_package_commands(
            &commands,
            &log_dir,
            &id,
            "project",
            &name,
            source,
            vec![archive_dir.clone()],
            cleanup_targets.clone(),
            PackageEvidence {
                project_settings: Some(project_settings.clone()),
                cook_mode: Some(cook_mode_name.to_string()),
                cook_reused: Some(can_iterate),
                cook_reuse_reason: Some(cook_reuse_reason.clone()),
                metadata: {
                    let mut metadata = PackageMetadata::target("project");
                    metadata.profile_revision =
                        saved_profile.as_ref().map(|profile| profile.revision);
                    metadata.source_digest = current_source_fingerprint.clone();
                    metadata.lineage_label =
                        saved_profile.as_ref().map(|profile| profile.name.clone());
                    metadata.task_ref = requested_task.clone();
                    metadata.storage = Some(storage.clone());
                    metadata.cleanup_policy = Some(if requested_iterate {
                        "keep-persistent-iterate-cache".into()
                    } else {
                        "remove-execution-stage-on-success".into()
                    });
                    metadata
                },
            },
        )?
    } else {
        Vec::new()
    };
    let mut package_diagnostics = warning_diagnostics(&logs);
    let manifests = if mode.executes() {
        if profile_output {
            write_manifest(&execution_archive_dir, &archive_dir)?;
            publish_directory(&execution_archive_dir, &archive_dir, &id, &log_dir).map_err(
                |error| {
                    let mut diagnostics = vec![format!("交付失败：{error}")];
                    diagnostics.extend(warning_diagnostics(&logs));
                    let mut failed = PackageResult {
                        execution_id: id.clone(),
                        action: "project".to_string(),
                        workspace: name.clone(),
                        source: source.to_string(),
                        state: "failed".to_string(),
                        exit_code: Some(0),
                        commands: commands_as_argv(&commands),
                        artifacts: vec![execution_archive_dir.clone()],
                        outputs: vec![archive_dir.clone()],
                        logs: logs.clone(),
                        manifests: Vec::new(),
                        cleanup_targets: cleanup_targets.clone(),
                        diagnostics,
                        project_settings: Some(project_settings.clone()),
                        cook_mode: Some(cook_mode_name.to_string()),
                        cook_reused: Some(can_iterate),
                        cook_reuse_reason: Some(cook_reuse_reason.clone()),
                        metadata: {
                            let mut metadata = PackageMetadata::target("project");
                            metadata.profile_revision =
                                saved_profile.as_ref().map(|profile| profile.revision);
                            metadata.source_digest = current_source_fingerprint.clone();
                            metadata.lineage_label =
                                saved_profile.as_ref().map(|profile| profile.name.clone());
                            metadata.task_ref = requested_task.clone();
                            metadata.storage = Some(storage.clone());
                            metadata
                        },
                    };
                    let _ = save_result(&failed);
                    failed.diagnostics.push(format!("execution ID: {id}"));
                    UdfError::Other(failed.diagnostics.join("；"))
                },
            )?;
        }
        if profile_output {
            vec![archive_dir.join(".udf-manifest.json")]
        } else {
            vec![write_manifest(&archive_dir, &archive_dir)?]
        }
    } else {
        Vec::new()
    };
    if mode.executes() && profile_output && requested_iterate {
        match remove_owned_tree(&execution_archive_dir) {
            Ok(()) => cleanup_targets.retain(|target| target != &execution_archive_dir),
            Err(error) => {
                package_diagnostics.push(format!("交付成功，但 Archive 自动清理失败：{error}"))
            }
        }
    }
    if mode.executes()
        && requested_iterate
        && let Some(source_fingerprint) = current_source_fingerprint.as_ref()
    {
        let now = Utc::now().to_rfc3339();
        write_cache_state(
            &execution_stage_root,
            &CookCacheState {
                schema_version: package_cache::CACHE_SCHEMA_VERSION,
                cache_id: execution_stage_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_string(),
                task_uid: saved_profile
                    .as_ref()
                    .map(|profile| profile.task_uid.clone())
                    .unwrap_or_default(),
                project: source_project,
                engine: engine_root.clone(),
                platform: "Win64".into(),
                configuration: configuration_name.clone(),
                container: container_name(container).to_string(),
                project_settings_digest: project_settings.digest.clone(),
                source_fingerprint: source_fingerprint.clone(),
                overlay_fingerprint: current_overlay_fingerprint.clone().unwrap_or_default(),
                state: "ready".into(),
                created_at: now.clone(),
                last_used_at: now.clone(),
                last_success_at: now,
                execution_id: id.clone(),
                producer_version: env!("CARGO_PKG_VERSION").into(),
            },
        )?;
    }
    let mut cleanup_result = "not-applicable".to_string();
    if mode.executes() && profile_output && !requested_iterate {
        match remove_owned_tree(&execution_stage_root) {
            Ok(()) => {
                cleanup_targets.retain(|target| target != &execution_stage_root);
                cleanup_result = "succeeded".into();
            }
            Err(error) => {
                cleanup_result = "warning".into();
                package_diagnostics.push(format!("成功交付，但自动清理 staging 失败：{error}"));
            }
        }
    } else if mode.executes() && requested_iterate {
        cleanup_result = "persistent-cache-kept".into();
    }
    finish_execution(
        "package project",
        PackageResult {
            execution_id: id,
            action: "project".to_string(),
            workspace: name,
            source: source.to_string(),
            state: "succeeded".to_string(),
            exit_code: Some(0),
            commands: commands_as_argv(&commands),
            artifacts: vec![if profile_output {
                execution_archive_dir
            } else {
                archive_dir.clone()
            }],
            outputs: vec![archive_dir.clone()],
            logs,
            manifests,
            cleanup_targets,
            diagnostics: package_diagnostics,
            project_settings: Some(project_settings),
            cook_mode: Some(cook_mode_name.to_string()),
            cook_reused: Some(can_iterate),
            cook_reuse_reason: Some(cook_reuse_reason),
            metadata: {
                let mut metadata = PackageMetadata::target("project");
                metadata.profile_revision = saved_profile.as_ref().map(|profile| profile.revision);
                metadata.source_digest = current_source_fingerprint;
                metadata.lineage_label = saved_profile.map(|profile| profile.name);
                metadata.task_ref = requested_task;
                metadata.storage = Some(storage);
                metadata.cleanup_result = Some(cleanup_result.clone());
                metadata.cleanup_policy = Some(if requested_iterate {
                    "keep-persistent-iterate-cache".into()
                } else {
                    "remove-execution-stage-on-success".into()
                });
                metadata
            },
        },
    )
}

/// 保留旧命令的解析兼容，但不再让模糊的顶层目标启动重量级工具链。
/// 用户必须把意图写进 `package advanced <target>`，这样 AI 和脚本不会因旧提示
/// 或补全结果误打插件/引擎。
pub fn legacy_target(target: &str) -> Result<()> {
    Err(UdfError::Other(format!(
        "package {target} 已降为兼容入口，未启动任何 UBT/BuildGraph；请明确使用 `udf package advanced {target} ...`。普通任务请使用 `udf package project ...`"
    )))
}

pub fn plugin(
    plugins: Vec<String>,
    task: Option<String>,
    workspace: Option<String>,
    requested_output: Option<PathBuf>,
    mode: PackageMode,
) -> Result<()> {
    if plugins.is_empty() {
        return Err(UdfError::Other("至少指定一个插件名".to_string()));
    }
    let config = Config::load()?;
    let requested_task = task.clone();
    let (name, plugins_root, engine_root, output_dir, source) = if let Some(task_ref) = task {
        let (host_dir, _, context) = crate::host::resolve_task(&config, &task_ref)?;
        (
            context.workspace,
            host_dir.join("Plugins"),
            context.engine_path,
            requested_output
                .clone()
                .unwrap_or_else(|| host_dir.join("Artifacts/UnrealDevFlow/Plugins")),
            "task",
        )
    } else {
        let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
        let plugins_root = workspace_config
            .effective_plugins_root()
            .ok_or_else(|| UdfError::Other(format!("workspace '{}' 没有 plugins_root", name)))?;
        let output_dir = requested_output.clone().unwrap_or_else(|| {
            plugins_root
                .parent()
                .unwrap_or(&plugins_root)
                .join("Artifacts/UnrealDevFlow/Plugins")
        });
        (
            name,
            plugins_root,
            workspace_config.engine_path,
            output_dir,
            "workspace",
        )
    };
    let index = plugin_index(&plugins_root)?;
    let (seed_plugins, mut selected_index) = resolve_plugin_seeds(&plugins_root, &plugins, &index)?;
    let id = if mode.executes() {
        execution_id("plugin")
    } else {
        "package-plugin-query".to_string()
    };
    // UBT still rejects action paths longer than 260 characters on Windows.
    // Host/task artifact paths are often already deep, so keep the disposable
    // build stage in the system temp directory and copy only final artifacts
    // back into the managed output directory.
    let stage_root = plugin_stage_root(&id);
    let closure = plugin_closure(&plugins_root, &seed_plugins, &index, &mut selected_index)?;
    let commands =
        direct_plugin_commands(&engine_root, &stage_root, &seed_plugins, &selected_index);
    let log_dir = execution_root()?.join(&id);
    let mut cleanup_targets = if mode.executes() {
        vec![stage_root.clone(), log_dir.clone()]
    } else {
        Vec::new()
    };
    let package_dirs = plugins
        .iter()
        .map(|plugin| output_dir.join(plugin))
        .collect::<Vec<_>>();
    let storage = crate::package_storage::assess_global(
        &output_dir,
        &[stage_root.clone(), log_dir.clone()],
        None,
    );
    if mode.executes() && storage.blocked() {
        return Err(UdfError::Other(format!(
            "package advanced plugin 空间预检阻止执行：{}",
            storage.diagnostics.join("；")
        )));
    }
    if mode == PackageMode::Check {
        let selector = requested_task
            .map(|task| format!("--task {task}"))
            .unwrap_or_else(|| format!("--workspace {name}"));
        return emit_check(
            "plugin",
            source,
            &commands,
            None,
            format!("udf package plugin {} {selector}", plugins.join(" ")),
            Vec::new(),
            Some(storage.clone()),
        );
    }
    if mode == PackageMode::Plan {
        let (_, diagnostics) = check_package_commands(&commands, None);
        return emit_plan(
            "plugin",
            source,
            &commands,
            package_dirs,
            diagnostics,
            None,
            Some(storage.clone()),
        );
    }
    let mut package_diagnostics = Vec::new();
    let (logs, cleanup_result) = if mode.executes() {
        prepare_plugin_stage(&plugins_root, &stage_root, &closure, &selected_index)?;
        let package_dirs = plugins
            .iter()
            .map(|plugin| output_dir.join(plugin))
            .collect::<Vec<_>>();
        let logs = run_package_commands(
            &commands,
            &log_dir,
            &id,
            "plugin",
            &name,
            source,
            package_dirs,
            cleanup_targets.clone(),
            PackageEvidence {
                metadata: PackageMetadata::target("plugin"),
                ..PackageEvidence::default()
            },
        )?;
        let mut delivery_logs = Vec::new();
        for plugin in &plugins {
            let package_dir = output_dir.join(plugin);
            let source_dir = stage_root.join("Plugins").join(plugin);
            write_manifest(&source_dir, &package_dir)?;
            let plugin_log_dir = log_dir.join(plugin);
            publish_directory(&source_dir, &package_dir, &id, &plugin_log_dir)?;
            delivery_logs.push(plugin_log_dir.join(".udf-delivery-journal.json"));
        }
        // Keep the per-plugin transaction journals discoverable by recover.
        let logs = logs.into_iter().chain(delivery_logs).collect();
        let cleanup_result = cleanup_plugin_stage_after_success(
            &stage_root,
            &mut cleanup_targets,
            &mut package_diagnostics,
        );
        (logs, cleanup_result)
    } else {
        (Vec::new(), "not-applicable".to_string())
    };
    let manifests = if mode.executes() {
        plugins
            .iter()
            .map(|plugin| output_dir.join(plugin).join(".udf-manifest.json"))
            .collect()
    } else {
        Vec::new()
    };
    let package_dirs = plugins
        .iter()
        .map(|plugin| output_dir.join(plugin))
        .collect::<Vec<_>>();
    finish_execution(
        "package plugin",
        PackageResult {
            execution_id: id,
            action: "plugin".to_string(),
            workspace: name,
            source: source.to_string(),
            state: "succeeded".to_string(),
            exit_code: Some(0),
            commands: commands_as_argv(&commands),
            artifacts: package_dirs.clone(),
            outputs: package_dirs,
            logs,
            manifests,
            cleanup_targets,
            diagnostics: package_diagnostics,
            project_settings: None,
            cook_mode: None,
            cook_reused: None,
            cook_reuse_reason: None,
            metadata: {
                let mut metadata = PackageMetadata::target("plugin");
                metadata.task_ref = requested_task.clone();
                metadata.storage = Some(storage);
                metadata.cleanup_result = Some(cleanup_result);
                metadata.cleanup_policy = Some("remove-private-plugin-stage-on-success".into());
                metadata
            },
        },
    )
}

pub fn engine(
    workspace: Option<String>,
    requested_output: Option<PathBuf>,
    requested_name: Option<String>,
    mode: PackageMode,
) -> Result<()> {
    let config = Config::load()?;
    let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
    let default_output = workspace_config
        .default_project
        .join("Saved")
        .join("UnrealDevFlow")
        .join("InstalledBuild");
    let output_dir = if let Some(root) = requested_output {
        let name = requested_name.unwrap_or_else(|| "InstalledBuild-Win64".into());
        root.join(name)
    } else {
        default_output
    };
    let commands = installed_build_commands(&InstalledBuildOptions {
        engine_root: workspace_config.engine_path,
        output_dir: output_dir.clone(),
        platform: UePlatform::Windows,
    });
    let storage = crate::package_storage::assess_global(&output_dir, &[execution_root()?], None);
    if mode.executes() && storage.blocked() {
        return Err(UdfError::Other(format!(
            "package advanced engine 空间预检阻止执行：{}",
            storage.diagnostics.join("；")
        )));
    }
    if mode == PackageMode::Check {
        return emit_check(
            "engine",
            "workspace",
            &commands,
            None,
            format!("udf package engine --workspace {name}"),
            Vec::new(),
            Some(storage.clone()),
        );
    }
    if mode == PackageMode::Plan {
        let (_, diagnostics) = check_package_commands(&commands, None);
        return emit_plan(
            "engine",
            "workspace",
            &commands,
            vec![output_dir],
            diagnostics,
            None,
            Some(storage.clone()),
        );
    }
    let id = execution_id("engine");
    let log_dir = execution_root()?.join(&id);
    let output_preexisted = output_dir.exists();
    let logs = if mode.executes() {
        run_package_commands(
            &commands,
            &log_dir,
            &id,
            "engine",
            &name,
            "workspace",
            vec![output_dir.clone()],
            engine_failure_cleanup_targets(&output_dir, &log_dir, output_preexisted),
            PackageEvidence {
                metadata: PackageMetadata::target("engine"),
                ..PackageEvidence::default()
            },
        )?
    } else {
        Vec::new()
    };
    let manifests = if mode.executes() {
        vec![write_manifest(&output_dir, &output_dir)?]
    } else {
        Vec::new()
    };
    finish_execution(
        "package engine",
        PackageResult {
            execution_id: id,
            action: "engine".to_string(),
            workspace: name,
            source: "workspace".to_string(),
            state: "succeeded".to_string(),
            exit_code: Some(0),
            commands: commands_as_argv(&commands),
            artifacts: vec![output_dir.clone()],
            outputs: vec![output_dir.clone()],
            logs,
            manifests,
            cleanup_targets: Vec::new(),
            diagnostics: Vec::new(),
            project_settings: None,
            cook_mode: None,
            cook_reused: None,
            cook_reuse_reason: None,
            metadata: {
                let mut metadata = PackageMetadata::target("engine");
                metadata.storage = Some(storage);
                metadata.cleanup_policy = Some("keep-final-installed-build".into());
                metadata
            },
        },
    )
}

pub fn build_engine(workspace: Option<String>, plan_only: bool) -> Result<()> {
    let config = Config::load()?;
    let (name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
    let commands = engine_source_build_commands(&EngineSourceBuildOptions {
        engine_root: workspace_config.engine_path.clone(),
        platform: UePlatform::Windows,
        configuration: Configuration::Development,
        mutex: UbtMutexMode::Wait,
        gitdeps_threads: 16,
        gitdeps_cache: None,
        editor_target: "UnrealEditor".to_string(),
        extra_targets: Vec::new(),
    });
    let id = format!("build-engine-{}", Utc::now().format("%Y%m%dT%H%M%SZ"));
    let log_dir = workspace_config
        .default_project
        .join("Saved/UnrealDevFlow")
        .join(&id);
    if !plan_only && !Path::new(&commands[0].executable).is_file() {
        return Err(UdfError::Other(format!(
            "workspace '{}' 使用的引擎不是源码引擎，缺少 {}。可先运行 `udf build plan --workspace {}` 查看完整计划",
            name, commands[0].executable, name
        )));
    }
    let logs = if plan_only {
        Vec::new()
    } else {
        run_commands(&commands, &log_dir)?
    };
    let result = PackageResult {
        execution_id: id,
        action: "engine".to_string(),
        workspace: name,
        source: "workspace".to_string(),
        state: if plan_only { "planned" } else { "succeeded" }.to_string(),
        exit_code: if plan_only { None } else { Some(0) },
        commands: commands_as_argv(&commands),
        artifacts: vec![workspace_config.engine_path.clone()],
        outputs: vec![workspace_config.engine_path],
        logs,
        manifests: Vec::new(),
        cleanup_targets: Vec::new(),
        diagnostics: Vec::new(),
        project_settings: None,
        cook_mode: None,
        cook_reused: None,
        cook_reuse_reason: None,
        metadata: PackageMetadata::target("engine"),
    };
    output::emit(
        if plan_only {
            "build plan"
        } else {
            "build engine"
        },
        result,
        render,
    );
    Ok(())
}

fn read_result(root: &Path, id: &str) -> Result<PackageResult> {
    let path = root.join(format!("{id}.json"));
    let bytes = fs::read(&path).map_err(|error| {
        UdfError::Other(format!(
            "找不到 package 执行记录 {}：{}",
            path.display(),
            error
        ))
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn is_real_execution_state(state: &str) -> bool {
    !matches!(state, "planned" | "ready" | "blocked" | "deferred")
}

fn load_result(execution_id: Option<&str>) -> Result<PackageResult> {
    let root = execution_root()?;
    if let Some(id) = execution_id {
        return read_result(&root, id);
    }

    if let Ok(latest_id) = fs::read_to_string(root.join("latest"))
        && let Ok(result) = read_result(&root, latest_id.trim())
        && is_real_execution_state(&result.state)
    {
        return Ok(result);
    }

    let mut candidates = fs::read_dir(&root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<PackageResult>(&bytes).ok())
        .filter(|result| is_real_execution_state(&result.state))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.execution_id.cmp(&right.execution_id));
    candidates.pop().ok_or_else(|| {
        UdfError::Other("找不到真实的 package 执行记录；check 和 plan 不属于执行状态".to_string())
    })
}

pub fn status(execution_id: Option<String>) -> Result<()> {
    let result = load_result(execution_id.as_deref())?;
    output::emit("package status", result, render);
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanReport {
    dry_run: bool,
    scope: String,
    total_bytes: u64,
    reclaimable_bytes: u64,
    protected_bytes: u64,
    unknown_bytes: u64,
    targets: Vec<PathBuf>,
    items: Vec<PackageInventoryItem>,
    diagnostics: Vec<String>,
}

fn package_records() -> Result<Vec<PackageResult>> {
    let root = execution_root()?;
    Ok(fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<PackageResult>(&bytes).ok())
        .collect())
}

fn update_cleaned_records(
    deleted_execution_ids: &HashSet<String>,
    deleted_paths: &BTreeSet<PathBuf>,
) -> Result<usize> {
    let root = execution_root()?;
    let mut updated = 0;
    for entry in fs::read_dir(&root).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let execution_id = value
            .get("executionId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let path_match = value
            .get("cleanupTargets")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(PathBuf::from)
            .any(|recorded| {
                let recorded = dunce::canonicalize(&recorded).unwrap_or(recorded);
                deleted_paths.iter().any(|removed| {
                    recorded
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&removed.to_string_lossy())
                })
            });
        if !deleted_execution_ids.contains(execution_id) && !path_match {
            continue;
        }
        value["state"] = serde_json::Value::String("cleaned".into());
        value["cleanupResult"] = serde_json::Value::String("manual-clean".into());
        fs::write(path, serde_json::to_vec_pretty(&value)?)?;
        updated += 1;
    }
    Ok(updated)
}

#[allow(clippy::too_many_arguments)]
pub fn clean(
    execution_id: Option<String>,
    task_ref: Option<String>,
    workspace: Option<String>,
    cache_id: Option<String>,
    stale: bool,
    legacy: bool,
    yes: bool,
    force: bool,
    dry_run: bool,
) -> Result<()> {
    let explicit_execution = execution_id.is_some();
    let scoped = explicit_execution
        || cache_id.is_some()
        || task_ref.is_some()
        || workspace.is_some()
        || stale
        || legacy;
    let inventory_only = !scoped;
    let filter = InventoryFilter {
        task_ref: task_ref.clone(),
        workspace: workspace.clone(),
    };
    let inventory = package_inventory::scan(&Config::config_dir()?, &package_temp_dir(), &filter);
    let scope = if let Some(id) = execution_id.as_deref() {
        format!("execution {id}")
    } else if let Some(id) = cache_id.as_deref() {
        format!("cache {id}")
    } else if let Some(task) = task_ref.as_deref() {
        format!("task {task}")
    } else if let Some(name) = workspace.as_deref() {
        format!("workspace {name}")
    } else if stale {
        "stale package data".into()
    } else if legacy {
        "legacy package data".into()
    } else {
        "all package data".into()
    };
    let mut diagnostics = inventory.diagnostics.clone();
    let mut selected = Vec::new();
    for item in &inventory.items {
        let matches = if let Some(id) = cache_id.as_deref() {
            item.category == "cook-cache"
                && item.path.file_name().and_then(|name| name.to_str()) == Some(id)
        } else if let Some(id) = execution_id.as_deref() {
            item.execution_ids.iter().any(|execution| execution == id)
        } else {
            let scope_candidate =
                (task_ref.is_some() || workspace.is_some()) && item.reclaim_reason.is_some();
            let stale_candidate = stale
                && matches!(
                    item.state.as_str(),
                    "stale" | "stale-running" | "failed" | "orphan"
                )
                && item.protection.is_none()
                && !item.active;
            // `%TEMP%\\UDF` predates the package lifecycle ledger.  Some
            // historical executions were later enriched with `targetKind`,
            // which makes their confidence `verified` rather than
            // `legacy-matched`.  That must not strand a completed stage:
            // `--legacy --yes` explicitly authorizes cleanup of this fixed,
            // managed legacy root, while the active/recent-failure guards
            // below still protect an in-flight or diagnostic stage.
            let legacy_candidate = legacy
                && item.category == "execution-stage"
                && item.reclaim_reason.as_deref()
                    == Some("旧 execution 记录可追溯，已过渡到 legacy 清理")
                && item.protection.is_none()
                && !item.active;
            (stale_candidate || legacy_candidate || scope_candidate)
                && item.protection.is_none()
                && !item.active
                && item.category != "final-output"
                && item.category != "profile"
                && item.category != "record"
        };
        if matches {
            selected.push(item.clone());
        }
    }
    selected.sort_by(|left, right| left.path.cmp(&right.path));
    selected.dedup_by(|left, right| left.path == right.path);

    if let Some(id) = execution_id.as_deref() {
        let result = load_result(Some(id))?;
        if result.metadata.target_kind.is_empty() && !legacy {
            return Err(UdfError::Other(format!(
                "执行记录 '{}' 缺少可信 targetKind，拒绝猜测性清理",
                id
            )));
        }
        for target in &result.cleanup_targets {
            let resolved = dunce::canonicalize(target).unwrap_or_else(|_| target.clone());
            if !is_managed_cleanup_target(&resolved)
                || selected
                    .iter()
                    .any(|item| item.category == "final-output" && item.path == resolved)
            {
                return Err(UdfError::Other(format!(
                    "拒绝清理未位于固定 UnrealDevFlow 制品根内的路径：{}",
                    target.display()
                )));
            }
            if !selected.iter().any(|item| item.path == resolved) && target.exists() {
                selected.push(PackageInventoryItem {
                    category: "execution-stage".into(),
                    path: resolved.clone(),
                    bytes: crate::package_storage::tree_bytes(target),
                    last_modified: None,
                    owner_kind: "execution".into(),
                    owner_id: id.into(),
                    execution_ids: vec![id.into()],
                    state: result.state.clone(),
                    active: false,
                    protection: None,
                    reclaim_reason: Some("execution cleanup target".into()),
                    confidence: if result.metadata.target_kind.is_empty() {
                        "legacy-matched"
                    } else {
                        "verified"
                    }
                    .into(),
                    outcome: None,
                });
            }
        }
    }

    let can_delete =
        !dry_run && !inventory_only && (yes || explicit_execution || cache_id.is_some());
    if !dry_run && !inventory_only && !can_delete {
        diagnostics.push("这是范围清理；未提供 --yes，已退回 dry-run，未删除任何文件".into());
    }
    let mut deleted = BTreeSet::new();
    if can_delete {
        let config_dir = Config::config_dir()?;
        let temp_dir = package_temp_dir();
        for item in &selected {
            if item.active || (item.protection.is_some() && !force) {
                diagnostics.push(format!("保护项未删除：{}", item.path.display()));
                continue;
            }
            if !is_managed_cleanup_target(&item.path)
                && !package_inventory::is_fixed_root_path(&item.path, &config_dir, &temp_dir)
            {
                diagnostics.push(format!("路径不在固定受管根，跳过：{}", item.path.display()));
                continue;
            }
            if let Err(error) = remove_owned_tree(&item.path) {
                diagnostics.push(format!("删除失败 {}：{}", item.path.display(), error));
            } else {
                deleted.insert(item.path.clone());
            }
        }
        let deleted_execution_ids = selected
            .iter()
            .filter(|item| deleted.contains(&item.path))
            .flat_map(|item| item.execution_ids.iter().cloned())
            .collect::<HashSet<_>>();
        diagnostics.push(format!(
            "删除关联 execution 数：{}",
            deleted_execution_ids.len()
        ));
        // Update every execution which referenced a removed target.  The old
        // implementation updated only records.first_mut(), which left the
        // remaining records claiming ownership of deleted bytes.
        let updated_records = update_cleaned_records(&deleted_execution_ids, &deleted)?;
        diagnostics.push(format!("更新 execution 记录数：{updated_records}"));
    }
    if inventory_only {
        diagnostics
            .push("未指定 execution ID 或 scope；仅报告 package inventory，未删除任何文件".into());
    }
    let selected_paths = selected
        .iter()
        .map(|item| item.path.clone())
        .collect::<BTreeSet<_>>();
    let mut report_items = inventory.items.clone();
    for item in &mut report_items {
        if deleted.contains(&item.path) {
            item.outcome = Some("deleted".into());
        } else if selected_paths.contains(&item.path) {
            item.outcome = Some(if item.active || item.protection.is_some() {
                "protected".into()
            } else if can_delete {
                "skipped".into()
            } else {
                "would-delete".into()
            });
        } else if item.active || item.protection.is_some() {
            item.outcome = Some("protected".into());
        }
    }
    let report = CleanReport {
        dry_run: inventory_only || dry_run || !can_delete,
        scope,
        total_bytes: inventory.total_bytes,
        reclaimable_bytes: inventory.reclaimable_bytes,
        protected_bytes: inventory.protected_bytes,
        unknown_bytes: inventory.unknown_bytes,
        targets: selected.iter().map(|item| item.path.clone()).collect(),
        items: report_items,
        diagnostics,
    };
    output::emit("package clean", report, |report| {
        format!(
            "package clean: {} bytes 可回收（{}，保护 {} bytes）",
            report.reclaimable_bytes, report.scope, report.protected_bytes
        )
    });
    Ok(())
}

pub fn recover(execution_id: String) -> Result<()> {
    let result = load_result(Some(&execution_id))?;
    let journal_path = result
        .logs
        .iter()
        .filter_map(|log| log.parent())
        .map(|dir| dir.join(".udf-delivery-journal.json"))
        .find(|path| path.is_file())
        .ok_or_else(|| {
            UdfError::Other(format!(
                "执行 '{}' 没有可验证的交付事务日志，拒绝猜测并修改输出目录",
                result.execution_id
            ))
        })?;
    let journal_text = fs::read_to_string(&journal_path)?;
    let mut journal: DeliveryJournal = serde_json::from_str(&journal_text)?;
    if journal.execution_id != result.execution_id || journal.state != "delivering" {
        return Err(UdfError::Other(format!(
            "交付事务日志不匹配或已不是 delivering：{}",
            journal_path.display()
        )));
    }
    if crate::junction::exists(&journal.output).unwrap_or(false) {
        return Err(UdfError::Other(format!(
            "恢复目标已变成 Junction，拒绝修改：{}",
            journal.output.display()
        )));
    }
    for entry in &journal.entries {
        safe_relative(&entry.relative)?;
        let target = journal.output.join(&entry.relative);
        if crate::junction::exists(&target).unwrap_or(false) {
            return Err(UdfError::Other(format!(
                "恢复目标包含 Junction，拒绝修改：{}",
                target.display()
            )));
        }
        if entry.removed {
            if target.exists()
                && (!target.is_file()
                    || entry
                        .old_digest
                        .as_deref()
                        .is_none_or(|digest| file_digest(&target).ok().as_deref() != Some(digest)))
            {
                return Err(UdfError::Other(format!(
                    "恢复前发现外部修改，未回退：{}",
                    target.display()
                )));
            }
            continue;
        }
        if target.exists() && (!target.is_file() || file_digest(&target)? != entry.new_digest) {
            return Err(UdfError::Other(format!(
                "恢复前发现外部修改，未回退：{}",
                target.display()
            )));
        }
        if entry.existed {
            let backup = entry.backup.as_ref().ok_or_else(|| {
                UdfError::Other(format!(
                    "恢复清单缺少旧文件备份：{}",
                    entry.relative.display()
                ))
            })?;
            if !backup.is_file() {
                return Err(UdfError::Other(format!(
                    "恢复清单中的备份不存在：{}",
                    backup.display()
                )));
            }
        }
    }
    for entry in &journal.entries {
        let target = journal.output.join(&entry.relative);
        if entry.existed {
            fs::copy(entry.backup.as_ref().expect("validated backup"), &target)?;
        } else if target.is_file() {
            fs::remove_file(target)?;
        }
    }
    journal.state = "rolled_back".into();
    write_journal(&journal_path, &journal)?;
    let mut recovered = result;
    recovered.state = "delivery_rolled_back".into();
    recovered
        .diagnostics
        .push("未完成交付已按事务清单回退".into());
    save_result(&recovered)?;
    output::emit("package recover", recovered, render);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_plugin_matrix_never_takes_engine_global_mutex() {
        let commands = direct_plugin_commands(
            Path::new("C:/UE"),
            Path::new("C:/stage/package-1"),
            &["AesWorld".to_string()],
            &BTreeMap::from([("AesWorld".to_string(), PathBuf::from("AesWorld"))]),
        );

        assert_eq!(commands.len(), 3);
        for command in commands {
            let argv = command.argv();
            assert!(argv.contains(&"-NoMutex".to_string()));
            assert!(!argv.contains(&"-WaitMutex".to_string()));
        }
    }

    #[test]
    fn plugin_stage_uses_short_managed_temp_root() {
        let stage = plugin_stage_root("package-plugin-1");

        assert_eq!(
            stage.parent(),
            Some(std::env::temp_dir().join("UDF").as_path())
        );
        assert_eq!(stage.file_name().unwrap().to_string_lossy().len(), 12);
        assert!(!stage.to_string_lossy().contains("Artifacts"));
        assert!(is_managed_cleanup_target(&stage));
        assert!(!is_managed_cleanup_target(&std::env::temp_dir()));
    }

    #[test]
    fn plugin_stage_uses_read_only_junctions_and_private_generated_dirs() {
        let root = tempfile::tempdir().unwrap();
        let plugins = root.path().join("Plugins");
        let source = plugins.join("AesWorld");
        let stage = root.path().join("Stage");
        fs::create_dir_all(source.join("Source/Runtime")).unwrap();
        fs::create_dir_all(source.join("Content/Maps")).unwrap();
        fs::create_dir_all(source.join("Config")).unwrap();
        fs::create_dir_all(source.join("Resources")).unwrap();
        fs::create_dir_all(source.join("Shaders")).unwrap();
        fs::create_dir_all(source.join("ThirdParty/SDK")).unwrap();
        fs::create_dir_all(source.join("Intermediate/Old")).unwrap();
        fs::create_dir_all(source.join("Binaries/Old")).unwrap();
        fs::create_dir_all(source.join("Saved/Old")).unwrap();
        let external_binary = root.path().join("ExternalBinary");
        fs::create_dir_all(&external_binary).unwrap();
        fs::write(source.join("AesWorld.uplugin"), "{}").unwrap();
        fs::write(source.join("Source/Runtime/A.cpp"), "// source").unwrap();
        fs::write(source.join("Content/Maps/Test.umap"), "content").unwrap();
        fs::write(source.join("Config/Default.ini"), "config").unwrap();
        fs::write(source.join("Intermediate/Old/stale.obj"), "old").unwrap();
        fs::write(source.join("Binaries/Required.dll"), "required").unwrap();
        fs::write(source.join("Saved/Old/stale.log"), "old").unwrap();
        fs::write(external_binary.join("Shared.dll"), "shared").unwrap();
        crate::junction::create(&external_binary, &source.join("Binaries/Shared")).unwrap();

        prepare_plugin_stage(
            &plugins,
            &stage,
            &["AesWorld".to_string()],
            &BTreeMap::from([("AesWorld".to_string(), PathBuf::from("AesWorld"))]),
        )
        .unwrap();

        let staged = stage.join("Plugins/AesWorld");
        assert!(!crate::junction::exists(&staged).unwrap_or(false));
        for name in [
            "Source",
            "Content",
            "Config",
            "Resources",
            "Shaders",
            "ThirdParty",
        ] {
            assert!(
                crate::junction::exists(&staged.join(name)).unwrap(),
                "{name}"
            );
        }
        for name in ["Intermediate", "Binaries", "Saved"] {
            assert!(staged.join(name).is_dir(), "{name}");
            assert!(
                !crate::junction::exists(&staged.join(name)).unwrap_or(false),
                "{name}"
            );
        }
        assert!(!staged.join("Intermediate/Old/stale.obj").exists());
        assert_eq!(
            fs::read_to_string(staged.join("Binaries/Required.dll")).unwrap(),
            "required"
        );
        assert!(!crate::junction::exists(&staged.join("Binaries/Shared")).unwrap_or(false));
        assert_eq!(
            fs::read_to_string(staged.join("Binaries/Shared/Shared.dll")).unwrap(),
            "shared"
        );
        assert!(!staged.join("Saved/Old/stale.log").exists());
        assert_eq!(
            fs::read_to_string(staged.join("AesWorld.uplugin")).unwrap(),
            "{}"
        );
    }

    #[test]
    fn successful_plugin_delivery_removes_private_stage() {
        let root = tempfile::tempdir().unwrap();
        let stage = root.path().join("plugin-stage");
        let logs = root.path().join("logs");
        fs::create_dir_all(&stage).unwrap();
        fs::write(stage.join("output.bin"), "output").unwrap();
        let mut cleanup_targets = vec![stage.clone(), logs.clone()];
        let mut diagnostics = Vec::new();

        let result =
            cleanup_plugin_stage_after_success(&stage, &mut cleanup_targets, &mut diagnostics);

        assert_eq!(result, "succeeded");
        assert!(!stage.exists());
        assert!(!cleanup_targets.contains(&stage));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn project_stage_disables_plugins_without_mutating_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("SourceProject");
        let stage = root.path().join("Stage");
        fs::create_dir_all(source.join("Plugins/AesWorld")).unwrap();
        fs::create_dir_all(source.join(".vscode")).unwrap();
        fs::create_dir_all(source.join("ContentBackups")).unwrap();
        fs::write(source.join("Plugins/AesWorld/.gitignore"), b"Binaries").unwrap();
        fs::write(
            source.join("ContentBackups/should-not-stage.txt"),
            b"backup",
        )
        .unwrap();
        fs::write(
            source.join("Game.uproject"),
            br#"{"FileVersion":3,"Plugins":[{"Name":"ModelContextProtocol","Enabled":true},{"Name":"AesWorld","Enabled":true}]}"#,
        ).unwrap();
        fs::write(source.join("Plugins/AesWorld/AesWorld.uplugin"), b"{} ").unwrap();
        let original = fs::read(source.join("Game.uproject")).unwrap();
        let staged = prepare_project_stage(
            &source.join("Game.uproject"),
            &stage,
            &["ModelContextProtocol".to_string()],
            &[],
        )
        .unwrap();
        let descriptor: serde_json::Value =
            serde_json::from_slice(&fs::read(staged).unwrap()).unwrap();
        assert_eq!(descriptor["Plugins"][0]["Enabled"], false);
        assert_eq!(descriptor["Plugins"][1]["Enabled"], true);
        assert_eq!(fs::read(source.join("Game.uproject")).unwrap(), original);
        assert!(!stage.join(".vscode").exists());
        assert!(!stage.join("ContentBackups").exists());
        assert!(
            crate::junction::exists(&stage.join("Plugins")).unwrap(),
            "without task overlays the complete project plugin root is read-only"
        );
        assert!(stage.join("Plugins/AesWorld/.gitignore").exists());
    }

    #[test]
    fn project_stage_preserves_top_level_junction_inputs() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("SourceProject");
        let external_content = root.path().join("ExternalContent");
        let stage = root.path().join("Stage");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&external_content).unwrap();
        fs::write(external_content.join("asset.uasset"), b"asset").unwrap();
        crate::junction::create(&external_content, &source.join("Content")).unwrap();
        fs::write(
            source.join("Game.uproject"),
            br#"{"FileVersion":3,"Plugins":[]}"#,
        )
        .unwrap();

        prepare_project_stage(&source.join("Game.uproject"), &stage, &[], &[]).unwrap();

        assert!(crate::junction::exists(&stage.join("Content")).unwrap());
        assert_eq!(
            crate::junction::get_target(&stage.join("Content")).unwrap(),
            crate::junction::get_target(&source.join("Content")).unwrap()
        );
        assert_eq!(
            fs::read(stage.join("Content/asset.uasset")).unwrap(),
            b"asset"
        );
    }

    #[test]
    fn source_fingerprint_detects_input_changes_without_resolving_every_file_as_a_junction() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("Content")).unwrap();
        fs::write(root.path().join("Game.uproject"), br#"{"FileVersion":3}"#).unwrap();
        fs::write(root.path().join("Content/asset.uasset"), b"before").unwrap();

        let before = source_fingerprint(root.path()).unwrap();
        fs::write(
            root.path().join("Content/asset.uasset"),
            b"after and changed",
        )
        .unwrap();
        let after = source_fingerprint(root.path()).unwrap();

        assert_ne!(before, after);
    }

    #[test]
    fn warning_diagnostics_group_unreal_log_warnings() {
        let root = tempfile::tempdir().unwrap();
        let log = root.path().join("step.log");
        fs::write(
            &log,
            "Warning: Unable to find package for cooking Foo\nWarning: Failed to find Bar\nwarning C4996: deprecated API\n",
        )
        .unwrap();

        let diagnostics = warning_diagnostics(&[log]);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("3 条警告"));
        assert!(diagnostics[0].contains("cook_missing_package=1"));
        assert!(diagnostics[0].contains("missing_file_or_dependency=1"));
        assert!(diagnostics[0].contains("deprecated_or_compiler=1"));
    }

    #[test]
    fn delivery_journal_preserves_existing_files_and_records_new_files() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("archive");
        let output = root.path().join("output");
        let logs = root.path().join("logs");
        fs::create_dir_all(source.join("Binaries")).unwrap();
        fs::create_dir_all(output.join("Binaries")).unwrap();
        fs::write(source.join("Binaries/Game.exe"), b"new").unwrap();
        fs::write(output.join("Binaries/Game.exe"), b"old").unwrap();
        fs::write(output.join("user.sav"), b"user data").unwrap();
        fs::write(
            output.join(".udf-manifest.json"),
            serde_json::json!({
                "schemaVersion": 1,
                "files": [{
                    "path": "Binaries/Game.exe",
                    "bytes": 3,
                    "digest": file_digest(&output.join("Binaries/Game.exe")).unwrap(),
                }]
            })
            .to_string(),
        )
        .unwrap();

        publish_directory(&source, &output, "package-test", &logs).unwrap();

        assert_eq!(fs::read(output.join("Binaries/Game.exe")).unwrap(), b"new");
        assert_eq!(fs::read(output.join("user.sav")).unwrap(), b"user data");
        let journal: DeliveryJournal =
            serde_json::from_slice(&fs::read(logs.join(".udf-delivery-journal.json")).unwrap())
                .unwrap();
        assert_eq!(journal.state, "delivered");
        assert_eq!(journal.entries.len(), 1);
        assert!(journal.entries[0].backup.is_some());
        assert!(!logs.join("delivery-backup").exists());
    }

    #[test]
    fn delivery_reconciles_stale_udf_files_but_keeps_unowned_files() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("archive");
        let output = root.path().join("output");
        let logs = root.path().join("logs");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&output).unwrap();
        fs::write(source.join("new.pak"), b"new").unwrap();
        fs::write(output.join("old.pak"), b"old").unwrap();
        fs::write(output.join("user.txt"), b"keep").unwrap();
        fs::write(
            output.join(".udf-manifest.json"),
            serde_json::json!({
                "schemaVersion": 1,
                "files": [{
                    "path": "old.pak",
                    "bytes": 3,
                    "digest": file_digest(&output.join("old.pak")).unwrap(),
                }]
            })
            .to_string(),
        )
        .unwrap();

        publish_directory(&source, &output, "package-reconcile", &logs).unwrap();

        assert!(!output.join("old.pak").exists());
        assert_eq!(fs::read(output.join("user.txt")).unwrap(), b"keep");
        assert!(output.join("new.pak").is_file());
    }

    #[test]
    fn engine_failure_cleanup_only_removes_new_output_and_own_logs() {
        let targets = engine_failure_cleanup_targets(
            Path::new("C:/Package/InstalledBuild-Win64"),
            Path::new("C:/udf/executions/package/package-engine-1"),
            false,
        );

        assert_eq!(
            targets,
            vec![
                PathBuf::from("C:/Package/InstalledBuild-Win64"),
                PathBuf::from("C:/udf/executions/package/package-engine-1"),
            ]
        );

        let existing_output_targets = engine_failure_cleanup_targets(
            Path::new("C:/Package/InstalledBuild-Win64"),
            Path::new("C:/udf/executions/package/package-engine-2"),
            true,
        );
        assert_eq!(
            existing_output_targets,
            vec![PathBuf::from("C:/udf/executions/package/package-engine-2")]
        );
    }

    #[test]
    fn cleanup_guard_accepts_user_package_execution_logs_but_not_external_output() {
        let execution_log = Config::config_dir()
            .unwrap()
            .join("executions/package/package-engine-1");
        assert!(is_managed_cleanup_target(&execution_log));
        assert!(!is_managed_cleanup_target(Path::new(
            "C:/Package/UE55-InstalledBuild-Win64"
        )));
    }
}
