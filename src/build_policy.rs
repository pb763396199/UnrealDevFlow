//! Controlled Unreal build policy.
//!
//! Resolves the `.uproject`, the engine root behind its `EngineAssociation`,
//! the editor target and the UnrealBuildTool mutex, then reports whether a
//! build may start right now. Raw `Build.bat` / `RunUBT.bat` invocations are
//! refused so that concurrent builds queue on the UBT mutex instead of
//! corrupting each other's intermediates.
//!
//! Ported from UnrealWorkflow's `uwf-devflow` module without its `uwf_core`
//! dependency: the request is a local struct carrying only the fields this
//! module reads, and JSON is produced by `serde` instead of hand-written
//! string formatting.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ipc_lock::{Error as LockError, Lock};
use md5::{Digest, Md5};
use serde::{Serialize, Serializer};

use crate::build_profile::BuildProfile;

/// Overrides the engine root that would otherwise come from `EngineAssociation`.
pub const ENGINE_ROOT_ENV: &str = "UNREALDEVFLOW_UE_ENGINE_ROOT";

/// Everything the build policy needs to know about a requested build.
///
/// Callers fill in what they know; every field is optional and the resolver
/// falls back to the current directory and the project's own metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuildPolicyRequest {
    /// Host project directory of a task, preferred over `project`.
    pub main_project: Option<String>,
    /// Project directory or `.uproject` path.
    pub project: Option<String>,
    /// Workspace root, used only when nothing more specific is known.
    pub workspace: Option<String>,
    /// Explicit engine root, overriding `EngineAssociation`.
    pub engine_root: Option<String>,
    /// Explicit editor target, overriding target discovery.
    pub build_target: Option<String>,
    /// Build profile name: `light`, `medium` or `heavy`.
    pub build_profile: Option<String>,
    /// A build command a caller proposes to run, checked before it runs.
    pub build_command: Option<String>,
    /// Requested mutex handling. `nomutex` is always refused.
    pub mutex_mode: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildPolicyStatus {
    Ready,
    NeedsUserInput,
    Blocked,
    Deferred,
}

impl BuildPolicyStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NeedsUserInput => "needsUserInput",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
        }
    }
}

impl Serialize for BuildPolicyStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Serialized as the constant `"disabled"`: this tool never runs raw UE builds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RawUnrealBuildDisabled;

impl Serialize for RawUnrealBuildDisabled {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("disabled")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPolicyReport {
    pub status: BuildPolicyStatus,
    pub reason: String,
    #[serde(serialize_with = "serialize_optional_path")]
    pub uproject_path: Option<PathBuf>,
    pub engine_association: Option<String>,
    #[serde(serialize_with = "serialize_optional_path")]
    pub engine_root: Option<PathBuf>,
    #[serde(serialize_with = "serialize_optional_path")]
    pub build_bat: Option<PathBuf>,
    #[serde(serialize_with = "serialize_optional_path")]
    pub ubt_dll: Option<PathBuf>,
    pub target: Option<String>,
    pub build_profile: String,
    pub mutex_name: Option<String>,
    pub mutex_status: String,
    pub ide_command: Option<String>,
    pub validation_command: Option<String>,
    pub raw_unreal_build: RawUnrealBuildDisabled,
    pub diagnostics: Vec<String>,
}

impl BuildPolicyReport {
    pub fn to_json(&self) -> String {
        to_json(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildGateReport {
    pub allowed: bool,
    pub status: BuildPolicyStatus,
    pub reason: String,
    pub matched_trigger: Option<String>,
    pub recommendation: String,
}

impl BuildGateReport {
    pub fn to_json(&self) -> String {
        to_json(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildExecution {
    #[serde(rename = "policy")]
    pub report: BuildPolicyReport,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl BuildExecution {
    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }

    pub fn to_json(&self) -> String {
        to_json(self)
    }
}

pub fn resolve_build_policy(request: &BuildPolicyRequest) -> BuildPolicyReport {
    resolve_build_policy_with(request, probe_mutex_status)
}

fn resolve_build_policy_with<F>(request: &BuildPolicyRequest, mutex_probe: F) -> BuildPolicyReport
where
    F: Fn(&str) -> String,
{
    let profile = match project_build_profile(request.build_profile.as_deref()) {
        Ok(profile) => profile,
        Err(reason) => {
            return BuildPolicyReport {
                status: BuildPolicyStatus::Blocked,
                reason,
                uproject_path: None,
                engine_association: None,
                engine_root: None,
                build_bat: None,
                ubt_dll: None,
                target: None,
                build_profile: request
                    .build_profile
                    .clone()
                    .unwrap_or_else(|| "light".to_string()),
                mutex_name: None,
                mutex_status: "notChecked".to_string(),
                ide_command: None,
                validation_command: None,
                raw_unreal_build: RawUnrealBuildDisabled,
                diagnostics: Vec::new(),
            };
        }
    };
    let mut report = BuildPolicyReport {
        status: BuildPolicyStatus::NeedsUserInput,
        reason: "uninitialized".to_string(),
        uproject_path: None,
        engine_association: None,
        engine_root: None,
        build_bat: None,
        ubt_dll: None,
        target: None,
        build_profile: profile.label().to_string(),
        mutex_name: None,
        mutex_status: "notChecked".to_string(),
        ide_command: None,
        validation_command: None,
        raw_unreal_build: RawUnrealBuildDisabled,
        diagnostics: Vec::new(),
    };

    if request
        .mutex_mode
        .as_deref()
        .is_some_and(|mode| mode.eq_ignore_ascii_case("nomutex"))
    {
        report.status = BuildPolicyStatus::Blocked;
        report.reason = "no_mutex_forbidden".to_string();
        return report;
    }

    let start = request
        .main_project
        .as_deref()
        .or(request.project.as_deref())
        .or(request.workspace.as_deref())
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok());
    let Some(start) = start else {
        report.reason = "build_start_path_missing".to_string();
        return report;
    };
    let project = match resolve_uproject(&start) {
        Ok(path) => path,
        Err(reason) => {
            report.reason = reason;
            return report;
        }
    };
    report.uproject_path = Some(project.clone());

    let association = read_engine_association(&project);
    report.engine_association = association.clone();
    let engine_root =
        match resolve_engine_root(request.engine_root.as_deref(), association.as_deref()) {
            Ok(path) => path,
            Err(reason) => {
                report.reason = reason;
                return report;
            }
        };
    let build_bat = engine_root
        .join("Engine")
        .join("Build")
        .join("BatchFiles")
        .join("Build.bat");
    if !build_bat.is_file() {
        report.reason = "build_bat_not_found".to_string();
        report.engine_root = Some(engine_root);
        return report;
    }
    report.engine_root = Some(engine_root.clone());
    report.build_bat = Some(build_bat.clone());

    let target = match resolve_editor_target(&project, request.build_target.as_deref()) {
        Ok(target) => target,
        Err(reason) => {
            report.reason = reason;
            return report;
        }
    };
    report.target = Some(target.clone());

    let ubt_dll = engine_root
        .join("Engine")
        .join("Binaries")
        .join("DotNET")
        .join("UnrealBuildTool")
        .join("UnrealBuildTool.dll");
    if ubt_dll.is_file() {
        let mutex_name = ubt_mutex_name(&ubt_dll);
        report.mutex_status = mutex_probe(&mutex_name);
        report.mutex_name = Some(format!("Global\\{mutex_name}"));
        report.ubt_dll = Some(ubt_dll);
    } else {
        report.mutex_status = "unknown".to_string();
        report
            .diagnostics
            .push("UnrealBuildTool.dll was not found; mutex status is unknown".to_string());
    }

    let project_arg = format!("-Project=\"{}\"", project.display());
    let base = format!(
        "\"{}\" {} Win64 {} {} -NoHotReloadFromIDE",
        build_bat.display(),
        target,
        "Development",
        project_arg
    );
    let strict_flags = profile.flags().join(" ");
    report.ide_command = Some(format!("{base} {strict_flags} -WaitMutex"));
    report.validation_command = Some(format!("{base} {strict_flags}"));

    if let Some(explicit) = request.build_command.as_deref() {
        if contains_flag(explicit, "-nomutex") {
            report.status = BuildPolicyStatus::Blocked;
            report.reason = "no_mutex_forbidden".to_string();
            return report;
        }
        if !command_contains_path(explicit, &project) {
            report.status = BuildPolicyStatus::Blocked;
            report.reason = "explicit_project_mismatch".to_string();
            return report;
        }
        if !command_contains_path(explicit, &build_bat) {
            report.status = BuildPolicyStatus::Blocked;
            report.reason = "explicit_build_bat_mismatch".to_string();
            return report;
        }
        if contains_flag(explicit, "-waitmutex") && report.mutex_status == "busy" {
            report.status = BuildPolicyStatus::Deferred;
            report.reason = "wait_mutex_would_block".to_string();
            return report;
        }
    }

    if report.mutex_status == "busy" {
        report.status = BuildPolicyStatus::Deferred;
        report.reason = "ubt_mutex_busy".to_string();
    } else {
        report.status = BuildPolicyStatus::Ready;
        report.reason = "build_policy_resolved".to_string();
    }
    report
}

pub fn inspect_provider_command(command: Option<&str>) -> BuildGateReport {
    let Some(command) = command.map(str::trim).filter(|value| !value.is_empty()) else {
        return BuildGateReport {
            allowed: false,
            status: BuildPolicyStatus::NeedsUserInput,
            reason: "build_command_missing".to_string(),
            matched_trigger: None,
            recommendation: "Pass the proposed tool command as --build-command data.".to_string(),
        };
    };
    let lower = command.to_ascii_lowercase();
    if contains_flag(command, "-nomutex") {
        return blocked_gate("no_mutex_forbidden", "-NoMutex");
    }
    for trigger in [
        "build.bat",
        "runubt.bat",
        "runuat.bat",
        "unrealbuildtool.dll",
        "build_run.bat",
    ] {
        if lower.contains(trigger) {
            return blocked_gate("raw_ue_build_forbidden", trigger);
        }
    }
    if contains_shell_chain(command) {
        return blocked_gate("shell_chaining_forbidden", "shell-chain");
    }
    if is_direct_controlled_command(command) {
        return BuildGateReport {
            allowed: true,
            status: BuildPolicyStatus::Ready,
            reason: "controlled_build_command".to_string(),
            matched_trigger: Some("unrealdevflow build".to_string()),
            recommendation:
                "Execute the single declared unrealdevflow command and preserve its JSON result."
                    .to_string(),
        };
    }
    BuildGateReport {
        allowed: false,
        status: BuildPolicyStatus::Blocked,
        reason: "undeclared_provider_command".to_string(),
        matched_trigger: None,
        recommendation: "Use one direct unrealdevflow build command; arbitrary shell wrappers and undeclared commands are blocked.".to_string(),
    }
}

/// Build actions that go through this tool and therefore respect the UBT mutex.
const CONTROLLED_BUILD_ACTIONS: &[&str] = &[
    "build-check",
    "build-gate",
    "build-project",
    "build-status",
    "build",
];

fn is_direct_controlled_command(command: &str) -> bool {
    let command = command.trim();
    let command = command
        .strip_prefix('&')
        .map(str::trim_start)
        .unwrap_or(command);
    let Some((program, arguments)) = split_program(command) else {
        return false;
    };
    let program = program.trim_matches(['\'', '"']).replace('\\', "/");
    let executable = program.rsplit('/').next().unwrap_or(program.as_str());
    let executable = executable.strip_prefix('$').unwrap_or(executable);
    if !executable.eq_ignore_ascii_case("unrealdevflow")
        && !executable.eq_ignore_ascii_case("unrealdevflow.exe")
        && !executable.eq_ignore_ascii_case("udf")
        && !executable.eq_ignore_ascii_case("udf.exe")
    {
        return false;
    }
    let arguments = arguments.trim_start().to_ascii_lowercase();
    CONTROLLED_BUILD_ACTIONS
        .iter()
        .any(|action| arguments == *action || arguments.starts_with(&format!("{action} ")))
}

fn split_program(command: &str) -> Option<(&str, &str)> {
    let first = command.chars().next()?;
    if matches!(first, '\'' | '"') {
        let end = command[1..].find(first)? + 2;
        return Some((&command[..end], &command[end..]));
    }
    let end = command.find(char::is_whitespace).unwrap_or(command.len());
    Some((&command[..end], &command[end..]))
}

fn contains_shell_chain(command: &str) -> bool {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut first_non_whitespace_seen = false;
    for ch in command.chars() {
        if ch == '\'' && !double_quoted {
            single_quoted = !single_quoted;
            continue;
        }
        if ch == '"' && !single_quoted {
            double_quoted = !double_quoted;
            continue;
        }
        if single_quoted || double_quoted {
            continue;
        }
        if ch.is_whitespace() {
            if matches!(ch, '\r' | '\n') {
                return true;
            }
            continue;
        }
        if !first_non_whitespace_seen {
            first_non_whitespace_seen = true;
            if ch == '&' {
                continue;
            }
        }
        if matches!(ch, ';' | '|' | '&') {
            return true;
        }
    }
    false
}

pub fn execute_project_build(request: &BuildPolicyRequest) -> Result<BuildExecution, String> {
    let report = resolve_build_policy(request);
    if report.status != BuildPolicyStatus::Ready {
        return Err(format!(
            "build policy is not ready: {} ({})",
            report.reason,
            report.status.as_str()
        ));
    }
    let build_bat = report
        .build_bat
        .as_ref()
        .ok_or_else(|| "resolved build is missing Build.bat".to_string())?;
    let project = report
        .uproject_path
        .as_ref()
        .ok_or_else(|| "resolved build is missing .uproject".to_string())?;
    let target = report
        .target
        .as_ref()
        .ok_or_else(|| "resolved build is missing Editor target".to_string())?;
    let project_arg = format!("-Project={}", project.display());
    let output = Command::new(build_bat)
        .arg(target)
        .arg("Win64")
        .arg("Development")
        .arg(project_arg)
        .arg("-NoHotReloadFromIDE")
        .args(project_build_profile(Some(&report.build_profile))?.flags())
        .current_dir(project.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|error| format!("controlled UE build failed to start: {error}"))?;
    Ok(BuildExecution {
        report,
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn blocked_gate(reason: &str, trigger: &str) -> BuildGateReport {
    BuildGateReport {
        allowed: false,
        status: BuildPolicyStatus::Blocked,
        reason: reason.to_string(),
        matched_trigger: Some(trigger.to_string()),
        recommendation: "Use `unrealdevflow build-check --format json`, then run one declared `unrealdevflow build` or `unrealdevflow build-project`; do not run raw UE build commands.".to_string(),
    }
}

fn resolve_uproject(start: &Path) -> Result<PathBuf, String> {
    if start.is_file()
        && start
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("uproject"))
    {
        return dunce::canonicalize(start)
            .map_err(|error| format!("uproject_path_invalid:{error}"));
    }
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent().unwrap_or(start).to_path_buf()
    };
    loop {
        let projects = fs::read_dir(&current)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.is_file()
                            && path
                                .extension()
                                .is_some_and(|extension| extension.eq_ignore_ascii_case("uproject"))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        match projects.as_slice() {
            [project] => {
                return dunce::canonicalize(project)
                    .map_err(|error| format!("uproject_path_invalid:{error}"));
            }
            [] => {}
            _ => return Err("multiple_uprojects_same_directory".to_string()),
        }
        if !current.pop() {
            break;
        }
    }
    Err("uproject_not_found".to_string())
}

fn project_build_profile(value: Option<&str>) -> Result<BuildProfile, String> {
    match value.unwrap_or("light") {
        "light" => Ok(BuildProfile::Light),
        "medium" => Ok(BuildProfile::Medium),
        "heavy" => Ok(BuildProfile::Heavy),
        value => Err(format!("unknown_build_profile:{value}")),
    }
}

fn read_engine_association(project: &Path) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(project).ok()?).ok()?;
    value
        .get("EngineAssociation")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn resolve_engine_root(
    explicit: Option<&str>,
    association: Option<&str>,
) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(path) = explicit {
        candidates.push(PathBuf::from(path));
    }
    if let Ok(path) = env::var(ENGINE_ROOT_ENV) {
        candidates.push(PathBuf::from(path));
    }
    if let Some(value) = association {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            candidates.push(path);
        }
        if let Some(path) = launcher_engine_root(value) {
            candidates.push(path);
        }
        if let Ok(program_files) = env::var("ProgramFiles") {
            candidates.push(
                PathBuf::from(program_files)
                    .join("Epic Games")
                    .join(format!("UE_{value}")),
            );
        }
    }
    candidates
        .into_iter()
        .find(|path| path.join("Engine/Build/BatchFiles/Build.bat").is_file())
        .and_then(|path| path.canonicalize().ok())
        .ok_or_else(|| {
            if association.is_none() && explicit.is_none() {
                "engine_association_missing".to_string()
            } else {
                "engine_root_unresolved".to_string()
            }
        })
}

fn launcher_engine_root(association: &str) -> Option<PathBuf> {
    let program_data = env::var("PROGRAMDATA").ok()?;
    let manifest = PathBuf::from(program_data)
        .join("Epic")
        .join("UnrealEngineLauncher")
        .join("LauncherInstalled.dat");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(manifest).ok()?).ok()?;
    let installations = value.get("InstallationList")?.as_array()?;
    let expected = format!("UE_{association}");
    installations.iter().find_map(|entry| {
        if entry.get("AppName")?.as_str()? != expected {
            return None;
        }
        entry
            .get("InstallLocation")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
    })
}

fn resolve_editor_target(project: &Path, explicit: Option<&str>) -> Result<String, String> {
    if let Some(target) = explicit.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(target.to_string());
    }
    let project_name = project
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let source = project.parent().unwrap_or(Path::new(".")).join("Source");
    let targets = fs::read_dir(source)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter_map(|path| path.file_name()?.to_str().map(str::to_string))
                .filter(|name| name.ends_with("Editor.Target.cs"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let expected = format!("{project_name}Editor.Target.cs");
    if targets.iter().any(|target| target == &expected) {
        return Ok(format!("{project_name}Editor"));
    }
    match targets.as_slice() {
        [target] => Ok(target.trim_end_matches(".Target.cs").to_string()),
        [] => Ok(format!("{project_name}Editor")),
        _ => Err("multiple_editor_targets".to_string()),
    }
}

fn ubt_mutex_name(ubt_dll: &Path) -> String {
    let mut bytes = Vec::new();
    for unit in ubt_dll.to_string_lossy().to_uppercase().encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let digest = Md5::digest(bytes);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("UnrealBuildTool_Mutex_{hex}")
}

fn probe_mutex_status(name: &str) -> String {
    let lock = match Lock::new(name) {
        Ok(lock) => lock,
        Err(error) => return format!("unknown:{error}"),
    };
    match lock.try_lock() {
        Ok(_guard) => "available".to_string(),
        Err(LockError::WouldBlock) => "busy".to_string(),
        Err(error) => format!("unknown:{error}"),
    }
}

fn contains_flag(command: &str, flag: &str) -> bool {
    command
        .split_whitespace()
        .any(|token| token.trim_matches(['\'', '"']).eq_ignore_ascii_case(flag))
}

fn command_contains_path(command: &str, path: &Path) -> bool {
    normalize(command).contains(&normalize(&path.to_string_lossy()))
}

fn normalize(value: &str) -> String {
    value.replace('/', "\\").to_ascii_lowercase()
}

fn serialize_optional_path<S: Serializer>(
    path: &Option<PathBuf>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match path {
        Some(path) => serializer.serialize_str(&path.to_string_lossy().replace('\\', "/")),
        None => serializer.serialize_none(),
    }
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|error| {
        serde_json::json!({ "error": format!("report_serialization_failed:{error}") }).to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_project_engine_target_and_no_wait_validation_command() {
        let fixture = BuildFixture::new("resolve");
        let request = fixture.request();
        let report = resolve_build_policy(&request);
        assert_eq!(report.status, BuildPolicyStatus::Ready);
        assert_eq!(report.target.as_deref(), Some("DEVEditor"));
        assert!(
            report
                .ide_command
                .as_deref()
                .unwrap()
                .contains("-WaitMutex")
        );
        assert!(
            !report
                .validation_command
                .as_deref()
                .unwrap()
                .contains("-WaitMutex")
        );
        assert!(
            report
                .validation_command
                .as_deref()
                .unwrap()
                .contains("DEVEditor Win64 Development")
        );
        assert!(
            report
                .validation_command
                .as_deref()
                .unwrap()
                .contains("-FailIfGeneratedCodeChanges")
        );
        assert!(
            !report
                .uproject_path
                .as_deref()
                .unwrap()
                .to_string_lossy()
                .starts_with(r"\\?\")
        );
        assert!(
            report
                .mutex_name
                .as_deref()
                .unwrap()
                .starts_with("Global\\UnrealBuildTool_Mutex_")
        );
    }

    #[test]
    fn no_mutex_and_raw_build_commands_are_blocked() {
        let fixture = BuildFixture::new("blocked");
        let mut request = fixture.request();
        request.mutex_mode = Some("nomutex".to_string());
        assert_eq!(resolve_build_policy(&request).reason, "no_mutex_forbidden");
        let gate = inspect_provider_command(Some("Build.bat DEVEditor Win64 Development"));
        assert!(!gate.allowed);
        assert_eq!(gate.reason, "raw_ue_build_forbidden");

        let smuggled = inspect_provider_command(Some(
            "& $udf build my-workspace/my-task --format json; Build.bat DEVEditor Win64 Development",
        ));
        assert!(!smuggled.allowed);
        assert_eq!(smuggled.reason, "raw_ue_build_forbidden");

        let chained = inspect_provider_command(Some(
            "& $udf build my-workspace/my-task --format json; Remove-Item file",
        ));
        assert!(!chained.allowed);
        assert_eq!(chained.reason, "shell_chaining_forbidden");

        for command in [
            "Remove-Item file",
            "cmd /c echo unsafe",
            "powershell -NoProfile -Command \"Remove-Item file\"",
            "pwsh -Command 'unrealdevflow build-check --format json'",
        ] {
            let undeclared = inspect_provider_command(Some(command));
            assert!(!undeclared.allowed, "must block {command}");
            assert_eq!(undeclared.reason, "undeclared_provider_command");
        }

        for command in [
            "unrealdevflow build-check --format json",
            "& $udf build my-workspace/my-task",
            "\"C:\\Tools\\unrealdevflow.exe\" build-project --format json",
        ] {
            let controlled = inspect_provider_command(Some(command));
            assert!(controlled.allowed, "must allow {command}");
            assert_eq!(controlled.reason, "controlled_build_command");
        }
    }

    #[test]
    fn explicit_project_mismatch_is_blocked() {
        let fixture = BuildFixture::new("mismatch");
        let mut request = fixture.request();
        request.build_command = Some(format!(
            "\"{}\" OtherEditor Win64 Development -Project=\"C:\\Other\\Other.uproject\"",
            fixture.build_bat.display()
        ));
        assert_eq!(
            resolve_build_policy(&request).reason,
            "explicit_project_mismatch"
        );
    }

    #[test]
    fn busy_mutex_is_deferred_without_waiting() {
        let fixture = BuildFixture::new("busy");
        let report = resolve_build_policy_with(&fixture.request(), |_| "busy".to_string());
        assert_eq!(report.status, BuildPolicyStatus::Deferred);
        assert_eq!(report.reason, "ubt_mutex_busy");
    }

    #[test]
    fn report_json_uses_forward_slashes_and_declares_raw_build_disabled() {
        let fixture = BuildFixture::new("json");
        let report = resolve_build_policy(&fixture.request());
        let value: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
        assert_eq!(value["status"], "ready");
        assert_eq!(value["rawUnrealBuild"], "disabled");
        assert_eq!(value["target"], "DEVEditor");
        let uproject = value["uprojectPath"].as_str().unwrap();
        assert!(uproject.ends_with("DEV/DEV.uproject"), "{uproject}");
        assert!(!uproject.contains('\\'), "{uproject}");
    }

    struct BuildFixture {
        root: PathBuf,
        project: PathBuf,
        engine: PathBuf,
        build_bat: PathBuf,
    }

    impl BuildFixture {
        fn new(label: &str) -> Self {
            let root = env::temp_dir().join(format!(
                "udf-build-policy-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let project = root.join("DEV");
            let engine = root.join("UE_5.5");
            fs::create_dir_all(project.join("Source")).unwrap();
            fs::write(
                project.join("DEV.uproject"),
                r#"{"EngineAssociation":"5.5"}"#,
            )
            .unwrap();
            fs::write(project.join("Source/DEVEditor.Target.cs"), "// fixture").unwrap();
            let build_bat = engine.join("Engine/Build/BatchFiles/Build.bat");
            fs::create_dir_all(build_bat.parent().unwrap()).unwrap();
            fs::write(&build_bat, "@echo off").unwrap();
            let ubt = engine.join("Engine/Binaries/DotNET/UnrealBuildTool/UnrealBuildTool.dll");
            fs::create_dir_all(ubt.parent().unwrap()).unwrap();
            fs::write(ubt, "fixture").unwrap();
            Self {
                root,
                project,
                engine,
                build_bat,
            }
        }

        fn request(&self) -> BuildPolicyRequest {
            BuildPolicyRequest {
                main_project: Some(self.project.to_string_lossy().to_string()),
                engine_root: Some(self.engine.to_string_lossy().to_string()),
                ..BuildPolicyRequest::default()
            }
        }
    }

    impl Drop for BuildFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
