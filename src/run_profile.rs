//! Strict, reusable run configuration. All storage paths are caller supplied.

use crate::error::{Result, UdfError};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeProject {
    Main,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunBackend {
    Editor,
    Commandlet,
    Gauntlet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditorMode {
    Editor,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Rhi {
    #[default]
    Default,
    Null,
    Dx11,
    Dx12,
    Vulkan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    Windowed,
    Fullscreen,
    Borderless,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunWindow {
    pub mode: WindowMode,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorOptions {
    pub mode: EditorMode,
    #[serde(default)]
    pub rhi: Rhi,
    pub window: Option<RunWindow>,
    #[serde(default)]
    pub native_args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandletOptions {
    pub name: String,
    #[serde(default)]
    pub native_args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GauntletOptions {
    pub test: String,
    pub build: String,
    pub platform: String,
    pub configuration: String,
    #[serde(default)]
    pub null_rhi: bool,
    #[serde(default)]
    pub unattended: bool,
    #[serde(default)]
    pub exec_cmds: Vec<String>,
    pub max_duration_seconds: Option<u32>,
    pub run_test: Option<String>,
    #[serde(default)]
    pub native_args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunProfile {
    pub schema_version: u32,
    pub description: String,
    pub use_when: String,
    pub project: RuntimeProject,
    pub backend: RunBackend,
    pub map: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor: Option<EditorOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commandlet: Option<CommandletOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gauntlet: Option<GauntletOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileIssue {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopeKind {
    Task,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileScope {
    pub kind: ScopeKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileValidation {
    pub revision: u64,
    pub digest: String,
    pub execution_id: String,
    pub state: String,
    pub engine: PathBuf,
    pub project: PathBuf,
    pub validated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredRunProfile {
    pub schema_version: u32,
    pub name: String,
    pub scope: ProfileScope,
    pub revision: u64,
    pub digest: String,
    pub profile: RunProfile,
    pub updated_at: String,
    pub last_reason: String,
    pub last_validation: Option<ProfileValidation>,
}

#[derive(Debug, Clone)]
pub struct SelectedProfile {
    pub stored: StoredRunProfile,
    pub path: PathBuf,
    pub inherited: bool,
}

fn invalid(message: impl Into<String>) -> UdfError {
    UdfError::InvalidConfig(message.into())
}

pub fn parse_candidate(text: &str) -> Result<RunProfile> {
    let profile: RunProfile = serde_json::from_str(text)
        .map_err(|error| invalid(format!("运行配置 JSON 无效：{error}")))?;
    profile.validate()?;
    Ok(profile)
}

/// Names are portable single path components, including on Windows.
pub fn validate_name(name: &str) -> Result<()> {
    let reserved = matches!(
        name.to_ascii_uppercase().as_str(),
        "CON" | "PRN" | "AUX" | "NUL"
    ) || (name.len() == 4
        && (name.as_bytes()[..3].eq_ignore_ascii_case(b"COM")
            || name.as_bytes()[..3].eq_ignore_ascii_case(b"LPT"))
        && matches!(name.as_bytes()[3], b'1'..=b'9'));
    if name.is_empty()
        || name.len() > 80
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || reserved
    {
        return Err(invalid(format!(
            "无效运行配置名 '{name}'：请使用小写字母、数字、连字符或下划线，并避开 Windows 保留名"
        )));
    }
    Ok(())
}

fn validate_batch_value(field: &str, value: &str, command_separators: bool) -> Result<()> {
    if value
        .chars()
        .any(|c| c.is_control() || matches!(c, '&' | '|' | '<' | '>' | '%' | '!' | '^' | '`' | '"'))
        || value.contains("$(")
        || (!command_separators && value.contains(';'))
    {
        return Err(invalid(format!(
            "{field} 包含 shell 操作符、变量展开或未支持的引号"
        )));
    }
    Ok(())
}

/// Native options cannot replace parameters that UDF already owns.
pub fn validate_native_args(args: &[String], backend: RunBackend) -> Result<()> {
    let common = [
        "project",
        "map",
        "game",
        "editor",
        "server",
        "run",
        "execcmds",
        "exec",
        "quit",
        "quit_editor",
        "exit",
        "testexit",
        "nullrhi",
        "rhi",
        "dx11",
        "dx12",
        "d3d11",
        "d3d12",
        "vulkan",
        "opengl",
        "opengl4",
        "windowed",
        "fullscreen",
        "borderless",
        "resx",
        "resy",
        "winx",
        "winy",
        "abslog",
        "log",
        "reportexportpath",
        "ini",
    ];
    let gauntlet = [
        "test",
        "tests",
        "build",
        "platform",
        "configuration",
        "runtest",
        "unattended",
        "attended",
        "maxduration",
        "notimeout",
        "testiterations",
        "scriptdir",
        "scriptsforproject",
        "tempdir",
        "logdir",
        "artifactpath",
        "args",
        "editorargs",
        "clientargs",
        "serverargs",
        "additionalargs",
        "editorexeccmds",
        "udfsynccmds",
        "clientexeccmds",
        "serverexeccmds",
        "nocompile",
        "compile",
        "ignorebuildrecords",
        "listargs",
        "listallargs",
    ];
    let mut seen = HashSet::new();
    for arg in args {
        validate_batch_value("nativeArgs", arg, false)?;
        if !arg.starts_with('-')
            || arg.trim() != arg
            || arg.to_ascii_lowercase().contains(".uproject")
        {
            return Err(invalid(format!(
                "nativeArgs 必须是单个原生开关，不能包含项目或地图：{arg}"
            )));
        }
        let key = arg
            .trim_start_matches('-')
            .split(['=', ':', ' ', '\t'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if key.is_empty()
            || common.contains(&key.as_str())
            || (backend == RunBackend::Gauntlet && gauntlet.contains(&key.as_str()))
            || arg
                .split_whitespace()
                .skip(1)
                .any(|part| part.starts_with('-'))
        {
            return Err(invalid(format!(
                "nativeArgs 与受控参数冲突：{arg}；请使用配置中的专用字段"
            )));
        }
        if !seen.insert(key) {
            return Err(invalid(format!("nativeArgs 重复指定参数：{arg}")));
        }
    }
    Ok(())
}

fn validate_identifier(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value.split('.').all(|part| {
            !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        })
    {
        return Err(invalid(format!("{field} 必须是明确的原生名称：{value}")));
    }
    Ok(())
}

impl RunProfile {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(invalid("只支持 schemaVersion=1"));
        }
        if self.description.trim().is_empty() || self.use_when.trim().is_empty() {
            return Err(invalid("description 和 useWhen 不能为空"));
        }
        if let Some(map) = &self.map {
            validate_batch_value("map", map, false)?;
            if map.trim().is_empty() || map.trim() != map || map.starts_with('-') {
                return Err(invalid("map 必须是明确的地图路径或查询名"));
            }
        }
        let present = usize::from(self.editor.is_some())
            + usize::from(self.commandlet.is_some())
            + usize::from(self.gauntlet.is_some());
        if present != 1 {
            return Err(invalid("运行配置必须且只能包含一个 backend 参数对象"));
        }
        match self.backend {
            RunBackend::Editor => {
                let options = self
                    .editor
                    .as_ref()
                    .ok_or_else(|| invalid("backend=editor 必须对应 editor 对象"))?;
                if let Some(window) = &options.window {
                    if window.width.is_some() != window.height.is_some()
                        || window.width == Some(0)
                        || window.height == Some(0)
                    {
                        return Err(invalid("窗口 width/height 必须成对出现且大于 0"));
                    }
                    if options.rhi == Rhi::Null {
                        return Err(invalid("null RHI 不能指定图形窗口"));
                    }
                }
                validate_native_args(&options.native_args, self.backend)?;
            }
            RunBackend::Commandlet => {
                let options = self
                    .commandlet
                    .as_ref()
                    .ok_or_else(|| invalid("backend=commandlet 必须对应 commandlet 对象"))?;
                validate_identifier("commandlet.name", &options.name)?;
                validate_native_args(&options.native_args, self.backend)?;
            }
            RunBackend::Gauntlet => {
                let options = self
                    .gauntlet
                    .as_ref()
                    .ok_or_else(|| invalid("backend=gauntlet 必须对应 gauntlet 对象"))?;
                validate_identifier("gauntlet.test", &options.test)?;
                validate_identifier("gauntlet.platform", &options.platform)?;
                validate_batch_value("gauntlet.build", &options.build, false)?;
                if options.build.trim().is_empty() || options.build.starts_with('-') {
                    return Err(invalid("gauntlet.build 不能为空或为开关"));
                }
                if !["Debug", "DebugGame", "Development", "Test", "Shipping"]
                    .contains(&options.configuration.as_str())
                {
                    return Err(invalid("gauntlet.configuration 不是受支持的 UE 构建配置"));
                }
                if options.max_duration_seconds == Some(0) {
                    return Err(invalid("maxDurationSeconds 必须大于 0"));
                }
                if let Some(filter) = &options.run_test {
                    validate_batch_value("gauntlet.runTest", filter, false)?;
                }
                for command in &options.exec_cmds {
                    validate_batch_value("gauntlet.execCmds", command, true)?;
                    for part in command.split([',', ';']) {
                        let verb = part.split_whitespace().next().unwrap_or("");
                        if verb.is_empty()
                            || verb.starts_with('-')
                            || matches!(
                                verb.to_ascii_lowercase().as_str(),
                                "quit" | "quit_editor" | "exit" | "exit_editor" | "quitgame"
                            )
                            || (verb.eq_ignore_ascii_case("automation")
                                && part.split_whitespace().nth(1).is_some_and(|subcommand| {
                                    matches!(
                                        subcommand.to_ascii_lowercase().as_str(),
                                        "quit" | "softquit"
                                    )
                                }))
                        {
                            return Err(invalid(format!(
                                "execCmds 不能重写原生参数或退出控制：{command}"
                            )));
                        }
                    }
                }
                validate_native_args(&options.native_args, self.backend)?;
            }
        }
        Ok(())
    }

    /// Missing scenario inputs are readiness issues, not a fabricated test result.
    pub fn required_inputs(&self) -> Vec<ProfileIssue> {
        let mut issues = Vec::new();
        if self
            .editor
            .as_ref()
            .is_some_and(|options| options.mode == EditorMode::Game)
            && self.map.is_none()
        {
            issues.push(ProfileIssue {
                code: "map_required".into(),
                message: "Game 启动需要明确地图".into(),
            });
        }
        if self.gauntlet.as_ref().is_some_and(|options| {
            options.test.eq_ignore_ascii_case("UE.EditorAutomation")
                && options
                    .run_test
                    .as_deref()
                    .is_none_or(|filter| filter.trim().is_empty())
        }) {
            issues.push(ProfileIssue {
                code: "automation_filter_required".into(),
                message: "UE.EditorAutomation 需要非空 runTest".into(),
            });
        }
        issues
    }

    pub fn digest(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("run profile consists of serializable fields");
        format!("md5:{:x}", Md5::digest(bytes))
    }
}

impl StoredRunProfile {
    pub fn validation_state(&self) -> &str {
        self.last_validation
            .as_ref()
            .filter(|v| v.revision == self.revision && v.digest == self.digest)
            .map(|v| v.state.as_str())
            .unwrap_or("pending")
    }
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn check_storage_path(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(invalid("运行配置存储根目录必须是绝对路径"));
    }
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir | Component::CurDir) {
            return Err(invalid("配置存储路径不能包含相对跳转"));
        }
        current.push(part.as_os_str());
        // A Windows drive prefix by itself is drive-relative, not a file path.
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if is_link(&metadata) => {
                return Err(invalid(format!(
                    "运行配置路径不能经过 Junction 或符号链接：{}",
                    current.display()
                )));
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn profile_path(root: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    let path = root.join(format!("{name}.json"));
    check_storage_path(&path)?;
    Ok(path)
}

pub fn load_profile(root: &Path, name: &str) -> Result<Option<StoredRunProfile>> {
    let path = profile_path(root, name)?;
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let stored: StoredRunProfile = serde_json::from_slice(&bytes)?;
    stored.profile.validate()?;
    if stored.schema_version != 1
        || stored.name != name
        || stored.revision == 0
        || stored.scope.value.trim().is_empty()
    {
        return Err(invalid(format!(
            "配置记录身份或版本无效：{}",
            path.display()
        )));
    }
    if stored.digest != stored.profile.digest() {
        return Err(invalid(format!(
            "配置内容已被直接修改：{}；请使用 run configure 登记候选",
            path.display()
        )));
    }
    Ok(Some(stored))
}

struct ProfileWriteLock {
    path: PathBuf,
}
impl ProfileWriteLock {
    fn acquire(root: &Path, name: &str) -> Result<Self> {
        let path = root.join(format!(".{name}.lock"));
        check_storage_path(&path)?;
        fs::create_dir_all(root)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    invalid(format!(
                        "配置 {name} 正在写入；如上次写入被中断，请核对锁文件 {}",
                        path.display()
                    ))
                } else {
                    error.into()
                }
            })?;
        drop(file);
        Ok(Self { path })
    }
}
impl Drop for ProfileWriteLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn write_stored(root: &Path, stored: &StoredRunProfile) -> Result<()> {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let destination = profile_path(root, &stored.name)?;
    let temporary = root.join(format!(
        ".{}.{}.{}.tmp",
        stored.name,
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let written = (|| -> Result<()> {
        let mut bytes = serde_json::to_vec_pretty(stored)?;
        bytes.push(b'\n');
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        check_storage_path(&destination)?;
        fs::rename(&temporary, &destination)?;
        Ok(())
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written
}

pub fn save_profile(
    root: &Path,
    name: &str,
    scope: &ProfileScope,
    profile: &RunProfile,
    reason: Option<&str>,
    expected_revision: Option<u64>,
) -> Result<StoredRunProfile> {
    profile_path(root, name)?;
    profile.validate()?;
    if scope.value.trim().is_empty() {
        return Err(invalid("配置来源不能为空"));
    }
    if scope.kind == ScopeKind::Workspace && profile.project == RuntimeProject::Host {
        return Err(invalid("workspace 配置不能指定 task Host；请使用 --task"));
    }
    let _lock = ProfileWriteLock::acquire(root, name)?;
    let existing = load_profile(root, name)?;
    let revision = existing.as_ref().map_or(0, |saved| saved.revision);
    if expected_revision.is_some_and(|expected| expected != revision) {
        return Err(invalid(format!(
            "配置 revision 已变化：期望 {:?}，当前 {revision}",
            expected_revision
        )));
    }
    if let Some(saved) = &existing {
        if &saved.scope != scope {
            return Err(invalid("已保存配置属于另一个来源，拒绝重绑定"));
        }
        if saved.profile == *profile {
            return Ok(saved.clone());
        }
        if reason.is_none_or(|text| text.trim().is_empty()) {
            return Err(invalid("修改已有运行配置必须提供 --reason"));
        }
    }
    let stored = StoredRunProfile {
        schema_version: 1,
        name: name.into(),
        scope: scope.clone(),
        revision: revision
            .checked_add(1)
            .ok_or_else(|| invalid("配置 revision 溢出"))?,
        digest: profile.digest(),
        profile: profile.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        last_reason: reason.unwrap_or("首次登记运行配置").into(),
        last_validation: existing.and_then(|saved| saved.last_validation),
    };
    write_stored(root, &stored)?;
    Ok(stored)
}

pub fn record_validation(
    root: &Path,
    name: &str,
    validation: ProfileValidation,
) -> Result<StoredRunProfile> {
    profile_path(root, name)?;
    let _lock = ProfileWriteLock::acquire(root, name)?;
    let mut stored =
        load_profile(root, name)?.ok_or_else(|| invalid("找不到要记录验证的运行配置"))?;
    if validation.digest != stored.digest || validation.revision != stored.revision {
        return Err(invalid("验证属于旧版配置，不能更新当前验证状态"));
    }
    if !["passed", "failed", "unknown"].contains(&validation.state.as_str())
        || validation.execution_id.trim().is_empty()
    {
        return Err(invalid(
            "验证结果必须包含真实执行 ID 与 passed/failed/unknown 状态",
        ));
    }
    stored.last_validation = Some(validation);
    write_stored(root, &stored)?;
    Ok(stored)
}

pub fn select_profile(
    task_root: Option<&Path>,
    workspace_root: &Path,
    name: &str,
) -> Result<Option<SelectedProfile>> {
    if let Some(root) = task_root
        && let Some(stored) = load_profile(root, name)?
    {
        return Ok(Some(SelectedProfile {
            stored,
            path: profile_path(root, name)?,
            inherited: false,
        }));
    }
    Ok(
        load_profile(workspace_root, name)?.map(|stored| SelectedProfile {
            stored,
            path: workspace_root.join(format!("{name}.json")),
            inherited: task_root.is_some(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::fs;
    use tempfile::TempDir;

    fn editor() -> Value {
        json!({"schemaVersion":1,"description":"打开编辑器","useWhen":"启动冒烟", "project":"main",
            "backend":"editor","editor":{"mode":"editor","rhi":"default","nativeArgs":[]}})
    }

    fn gauntlet(test: &str) -> Value {
        json!({"schemaVersion":1,"description":"退出验证","useWhen":"检查自然退出", "project":"host",
            "backend":"gauntlet","gauntlet":{"test":test,"build":"editor","platform":"Win64",
            "configuration":"Development","nullRhi":true,"unattended":true,"execCmds":["stat unit"],"maxDurationSeconds":120}})
    }

    fn parse(value: Value) -> RunProfile {
        parse_candidate(&value.to_string()).unwrap()
    }
    fn scope(kind: ScopeKind) -> ProfileScope {
        ProfileScope {
            kind,
            value: "neon-dev/example".into(),
        }
    }

    #[test]
    fn backend_and_nested_fields_are_closed() {
        for field in ["extra", "revision", "digest"] {
            let mut value = editor();
            value[field] = json!(1);
            assert!(parse_candidate(&value.to_string()).is_err(), "{field}");
        }
        let mut value = editor();
        value["editor"]["extra"] = json!(true);
        assert!(parse_candidate(&value.to_string()).is_err());
        let mut value = editor();
        value["commandlet"] = json!({"name":"ResavePackages"});
        assert!(parse_candidate(&value.to_string()).is_err());
        let mut value = editor();
        value["backend"] = json!("gauntlet");
        assert!(parse_candidate(&value.to_string()).is_err());
    }

    #[test]
    fn invalid_schema_and_zero_window_sizes_are_rejected() {
        let mut value = editor();
        value["schemaVersion"] = json!(2);
        assert!(parse_candidate(&value.to_string()).is_err());
        let mut value = editor();
        value["editor"]["window"] = json!({"mode":"windowed","width":0,"height":1024});
        assert!(parse_candidate(&value.to_string()).is_err());
        value["editor"]["window"] = json!({"mode":"windowed","width":1366});
        assert!(parse_candidate(&value.to_string()).is_err());
    }

    #[test]
    fn editor_automation_filter_is_a_required_input_not_a_fake_validation() {
        let mut value = gauntlet("UE.EditorAutomation");
        assert_eq!(
            parse(value.clone()).required_inputs()[0].code,
            "automation_filter_required"
        );
        value["gauntlet"]["runTest"] = json!(" ");
        assert_eq!(
            parse(value.clone()).required_inputs()[0].code,
            "automation_filter_required"
        );
        value["gauntlet"]["runTest"] = json!("Earth.Elevation.SpatialElementConstruction");
        assert!(parse(value).required_inputs().is_empty());
        assert!(
            parse(gauntlet("UE.EditorBootTest"))
                .required_inputs()
                .is_empty()
        );
    }

    #[test]
    fn game_map_is_required_but_editor_smoke_can_use_untitled() {
        let mut value = editor();
        assert!(parse(value.clone()).required_inputs().is_empty());
        value["editor"]["mode"] = json!("game");
        assert_eq!(
            parse(value.clone()).required_inputs()[0].code,
            "map_required"
        );
        value["map"] = json!("/Game/Maps/Sample");
        assert!(parse(value).required_inputs().is_empty());
    }

    #[test]
    fn native_args_cannot_override_controlled_fields() {
        for arg in [
            "-Project=Elsewhere",
            "--map=/Game/Other",
            "-GAME",
            "-server",
            "-run=Foo",
            "-ExecCmds=QUIT_EDITOR",
            "-nullrhi",
            "-dx12",
            "-ResX=20",
            "-windowed",
            "-unattended",
            "-abslog=C:/elsewhere.log",
            "-ReportExportPath=C:/report",
            "-Test=UE.Other",
            "-RunTest=Other",
            "-TempDir=C:/shared",
            "-ScriptDir=C:/other",
            "-MaxDuration=1",
            "Other.uproject",
            "/Game/Other",
        ] {
            let mut value = gauntlet("Udf.EditorExit");
            value["gauntlet"]["nativeArgs"] = json!([arg]);
            assert!(parse_candidate(&value.to_string()).is_err(), "{arg}");
        }
    }

    #[test]
    fn shell_operators_and_expansion_are_rejected_but_ue_exec_separator_is_preserved() {
        for arg in [
            "-Foo=a&whoami",
            "-Foo=a|b",
            "-Foo=%PATH%",
            "-Foo=!PATH!",
            "-Foo=^x",
            "-Foo=$(x)",
            "-Foo=`x`",
            "-Foo=a\nb",
            "-Foo=\"x\"",
            "-Foo=1;whoami",
            "-Foo=<x>",
        ] {
            let mut value = gauntlet("Udf.EditorExit");
            value["gauntlet"]["nativeArgs"] = json!([arg]);
            assert!(parse_candidate(&value.to_string()).is_err(), "{arg}");
        }
        let mut value = gauntlet("Udf.EditorExit");
        value["gauntlet"]["execCmds"] = json!([
            "stat unit",
            "AesWorld.Settings.Get Tier=Low TerrainSettings.MaxLevel"
        ]);
        value["gauntlet"]["nativeArgs"] = json!(["-CustomCase=Terrain", "-CustomFlag"]);
        assert_eq!(parse(value).gauntlet.unwrap().exec_cmds.len(), 2);
    }

    #[test]
    fn malformed_exec_commands_cannot_add_an_exit_or_native_argument() {
        for command in [
            "QUIT_EDITOR",
            "stat unit, QUIT_EDITOR",
            "stat unit; quit",
            "-ExecCmds=x",
            "stat unit & whoami",
        ] {
            let mut value = gauntlet("Udf.EditorExit");
            value["gauntlet"]["execCmds"] = json!([command]);
            assert!(parse_candidate(&value.to_string()).is_err(), "{command}");
        }
    }

    #[test]
    fn names_reject_traversal_and_windows_aliases_before_any_write() {
        let root = TempDir::new().unwrap();
        let profiles = root.path().join("profiles");
        for name in [
            "",
            "../outside",
            "..\\outside",
            "/absolute",
            "C:\\absolute",
            "a:b",
            "CON",
            "nul",
            "COM1",
            "LPT9",
            "a.",
            "a ",
            "a/b",
            "中a",
        ] {
            assert!(
                save_profile(
                    &profiles,
                    name,
                    &scope(ScopeKind::Task),
                    &parse(editor()),
                    None,
                    None
                )
                .is_err(),
                "{name}"
            );
        }
        assert!(!profiles.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        assert!(validate_name("editor-exit").is_ok());
    }

    #[test]
    fn saved_profile_roundtrips_source_and_digest_without_validation_history() {
        let root = TempDir::new().unwrap();
        let candidate = parse(editor());
        let source = scope(ScopeKind::Task);
        let saved =
            save_profile(root.path(), "editor-exit", &source, &candidate, None, None).unwrap();
        assert_eq!(
            saved,
            load_profile(root.path(), "editor-exit").unwrap().unwrap()
        );
        assert_eq!(saved.scope, source);
        assert_eq!(saved.digest, candidate.digest());
        assert_eq!(saved.revision, 1);
        assert_eq!(saved.validation_state(), "pending");
        assert!(saved.profile.required_inputs().is_empty());
    }

    #[test]
    fn changed_profile_invalidates_validation_and_requires_update_reason() {
        let root = TempDir::new().unwrap();
        let candidate = parse(editor());
        let source = scope(ScopeKind::Task);
        let saved =
            save_profile(root.path(), "editor-exit", &source, &candidate, None, None).unwrap();
        let validation = ProfileValidation {
            revision: saved.revision,
            digest: saved.digest.clone(),
            execution_id: "run-001".into(),
            state: "passed".into(),
            engine: "engine".into(),
            project: "project".into(),
            validated_at: "2026-09-05T00:00:00Z".into(),
        };
        assert_eq!(
            record_validation(root.path(), "editor-exit", validation)
                .unwrap()
                .validation_state(),
            "passed"
        );
        let mut next = candidate;
        next.editor
            .as_mut()
            .unwrap()
            .native_args
            .push("-trace=cpu".into());
        assert!(save_profile(root.path(), "editor-exit", &source, &next, None, Some(1)).is_err());
        let updated = save_profile(
            root.path(),
            "editor-exit",
            &source,
            &next,
            Some("增加追踪"),
            Some(1),
        )
        .unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.validation_state(), "pending");
        assert_eq!(updated.last_validation.unwrap().execution_id, "run-001");
    }

    #[test]
    fn stale_revision_rejected_and_unchanged_save_is_idempotent() {
        let root = TempDir::new().unwrap();
        let candidate = parse(editor());
        let source = scope(ScopeKind::Task);
        let saved = save_profile(root.path(), "smoke", &source, &candidate, None, Some(0)).unwrap();
        let bytes = fs::read(root.path().join("smoke.json")).unwrap();
        assert_eq!(
            save_profile(root.path(), "smoke", &source, &candidate, None, Some(1)).unwrap(),
            saved
        );
        assert!(
            save_profile(
                root.path(),
                "smoke",
                &source,
                &candidate,
                Some("过时更新"),
                Some(0)
            )
            .is_err()
        );
        assert_eq!(fs::read(root.path().join("smoke.json")).unwrap(), bytes);
    }

    #[test]
    fn task_configuration_replaces_whole_workspace_configuration() {
        let root = TempDir::new().unwrap();
        let task = root.path().join("task");
        let workspace = root.path().join("workspace");
        let mut global = editor();
        global["map"] = json!("/Game/WorkspaceMap");
        save_profile(
            &workspace,
            "smoke",
            &scope(ScopeKind::Workspace),
            &parse(global),
            None,
            None,
        )
        .unwrap();
        assert!(
            select_profile(Some(&task), &workspace, "smoke")
                .unwrap()
                .unwrap()
                .inherited
        );
        save_profile(
            &task,
            "smoke",
            &scope(ScopeKind::Task),
            &parse(editor()),
            None,
            None,
        )
        .unwrap();
        let selected = select_profile(Some(&task), &workspace, "smoke")
            .unwrap()
            .unwrap();
        assert!(!selected.inherited);
        assert_eq!(selected.path, task.join("smoke.json"));
        assert!(selected.stored.profile.map.is_none());
    }

    #[test]
    fn direct_edits_and_scope_rebinding_do_not_overwrite_existing_config() {
        let root = TempDir::new().unwrap();
        let candidate = parse(editor());
        save_profile(
            root.path(),
            "smoke",
            &scope(ScopeKind::Task),
            &candidate,
            None,
            None,
        )
        .unwrap();
        assert!(
            save_profile(
                root.path(),
                "smoke",
                &scope(ScopeKind::Workspace),
                &candidate,
                Some("改来源"),
                Some(1)
            )
            .is_err()
        );
        let path = root.path().join("smoke.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["profile"]["description"] = json!("外部改动");
        fs::write(&path, value.to_string()).unwrap();
        assert!(load_profile(root.path(), "smoke").is_err());
    }

    #[test]
    fn missing_profile_queries_do_not_create_storage() {
        let root = TempDir::new().unwrap();
        let task = root.path().join("task");
        let workspace = root.path().join("workspace");
        assert!(load_profile(&task, "smoke").unwrap().is_none());
        assert!(
            select_profile(Some(&task), &workspace, "smoke")
                .unwrap()
                .is_none()
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn workspace_host_configuration_is_rejected_without_writes() {
        let root = TempDir::new().unwrap();
        let storage = root.path().join("profiles");
        let profile = parse(gauntlet("Udf.EditorExit"));
        assert!(
            save_profile(
                &storage,
                "exit",
                &scope(ScopeKind::Workspace),
                &profile,
                None,
                None
            )
            .is_err()
        );
        assert!(!storage.exists());
    }

    #[test]
    fn duplicate_native_options_and_unknown_backend_fields_are_rejected() {
        let mut value = editor();
        value["editor"]["nativeArgs"] = json!(["-Trace=cpu", "--trace=gpu"]);
        assert!(parse_candidate(&value.to_string()).is_err());
        let mut value = gauntlet("Udf.EditorExit");
        value["gauntlet"]["unknown"] = json!(true);
        assert!(parse_candidate(&value.to_string()).is_err());
        let mut value = editor();
        value["editor"]["window"] = json!({"mode":"windowed","width":1280,"height":720,"left":0});
        assert!(parse_candidate(&value.to_string()).is_err());
        let value = json!({"schemaVersion":1,"description":"资产处理","useWhen":"检查命令行", "project":"main",
            "backend":"commandlet","commandlet":{"name":"ResavePackages","nativeArgs":["-PackageFolder=/Game/AesWorld"]}});
        assert_eq!(parse(value).commandlet.unwrap().name, "ResavePackages");
    }

    #[test]
    fn two_updates_cannot_both_consume_the_same_revision() {
        use std::sync::{Arc, Barrier};
        let root = TempDir::new().unwrap();
        let source = scope(ScopeKind::Task);
        let candidate = parse(editor());
        save_profile(root.path(), "smoke", &source, &candidate, None, None).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|index| {
                let root = root.path().to_owned();
                let source = source.clone();
                let mut candidate = candidate.clone();
                candidate.description = format!("并发更新 {index}");
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    save_profile(
                        &root,
                        "smoke",
                        &source,
                        &candidate,
                        Some("并发测试"),
                        Some(1),
                    )
                })
            })
            .collect();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            load_profile(root.path(), "smoke")
                .unwrap()
                .unwrap()
                .revision,
            2
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn failed_atomic_replacement_keeps_the_original_and_removes_only_its_temporary() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = TempDir::new().unwrap();
        let mut stored = save_profile(
            root.path(),
            "smoke",
            &scope(ScopeKind::Task),
            &parse(editor()),
            None,
            None,
        )
        .unwrap();
        let destination = root.path().join("smoke.json");
        let before = fs::read(&destination).unwrap();
        let held_file = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&destination)
            .unwrap();
        stored.last_reason = "模拟文件被占用".into();
        assert!(write_stored(root.path(), &stored).is_err());
        drop(held_file);
        assert_eq!(fs::read(&destination).unwrap(), before);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn junction_profile_root_cannot_redirect_writes() {
        let root = TempDir::new().unwrap();
        let target = root.path().join("outside");
        fs::create_dir(&target).unwrap();
        let link = root.path().join("profiles");
        crate::junction::create(&target, &link).unwrap();
        let result = save_profile(
            &link,
            "smoke",
            &scope(ScopeKind::Task),
            &parse(editor()),
            None,
            None,
        );
        crate::junction::delete(&link).unwrap();
        assert!(result.is_err());
        assert_eq!(fs::read_dir(target).unwrap().count(), 0);
    }
}
