use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::host;
use crate::project_packaging::{PackageContainer as ProjectContainer, ProjectPackagingSnapshot};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Container {
    Loose,
    Pak,
    Iostore,
}

impl Container {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "loose" => Ok(Self::Loose),
            "pak" => Ok(Self::Pak),
            "iostore" => Ok(Self::Iostore),
            other => Err(UdfError::Other(format!(
                "不支持的 package container：{other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CookMode {
    #[default]
    Iterate,
    Full,
}

impl CookMode {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "full" => Ok(Self::Full),
            "iterate" => Ok(Self::Iterate),
            other => Err(UdfError::Other(format!("不支持的 Cook 模式：{other}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageProfile {
    pub schema_version: u32,
    pub task_uid: String,
    pub created: String,
    pub host: PathBuf,
    pub project: PathBuf,
    pub engine: PathBuf,
    pub configuration: String,
    pub container: Container,
    pub name: String,
    pub output: PathBuf,
    #[serde(default)]
    pub disabled_plugins: Vec<String>,
    pub revision: u64,
    #[serde(default)]
    pub last_reason: String,
    #[serde(default)]
    pub content_digest: String,
    #[serde(default)]
    pub project_settings: Option<ProjectPackagingSnapshot>,
    #[serde(default)]
    pub cook_mode: CookMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureResult {
    pub profile_path: PathBuf,
    pub configuration: String,
    pub container: Container,
    pub name: String,
    pub output: PathBuf,
    pub disabled_plugins: Vec<String>,
    pub cook_mode: CookMode,
    pub revision: u64,
    pub changed: bool,
}

#[derive(Debug, Default)]
pub struct ConfigureOptions {
    pub workspace: Option<String>,
    pub task: Option<String>,
    pub configuration: Option<String>,
    pub container: Option<String>,
    pub cook_mode: Option<String>,
    pub name: Option<String>,
    pub output: Option<PathBuf>,
    pub disable_plugin: Vec<String>,
    pub file: Option<PathBuf>,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub struct Binding {
    pub task_uid: String,
    pub created: String,
    pub host: PathBuf,
    pub project: PathBuf,
    pub engine: PathBuf,
}

pub fn resolve_binding(
    config: &Config,
    workspace: Option<&str>,
    task: Option<&str>,
) -> Result<Binding> {
    if let Some(task_ref) = task {
        let (host_dir, meta, context) = host::resolve_task(config, task_ref)?;
        return Ok(Binding {
            task_uid: meta
                .task_uid
                .unwrap_or_else(|| format!("{}/{}", context.workspace, meta.id)),
            created: meta.created,
            host: host_dir,
            project: context.default_project,
            engine: context.engine_path,
        });
    }
    let (name, workspace_config) = config.resolve_workspace(workspace)?;
    let project = workspace_config.default_project.clone();
    Ok(Binding {
        task_uid: format!("workspace/{name}"),
        created: String::new(),
        host: project.clone(),
        project,
        engine: workspace_config.engine_path,
    })
}

pub fn profile_path(binding: &Binding) -> Result<PathBuf> {
    if binding.created.is_empty() {
        Ok(Config::config_dir()?
            .join("package")
            .join("profiles")
            .join(format!("{}.toml", binding.task_uid.replace('/', "_"))))
    } else {
        Ok(binding.host.join(".udf-package.toml"))
    }
}

fn defaults(
    binding: &Binding,
    project_settings: Option<&ProjectPackagingSnapshot>,
) -> PackageProfile {
    let configuration = project_settings
        .and_then(|settings| settings.packaging.configuration.clone())
        .unwrap_or_else(|| "Development".into());
    let name = default_name(binding, &configuration);
    let container = project_settings
        .map(|settings| match settings.packaging.container {
            ProjectContainer::Loose => Container::Loose,
            ProjectContainer::Pak => Container::Pak,
            ProjectContainer::Iostore => Container::Iostore,
        })
        .unwrap_or(Container::Pak);
    PackageProfile {
        schema_version: 1,
        task_uid: binding.task_uid.clone(),
        created: binding.created.clone(),
        host: binding.host.clone(),
        project: binding.project.clone(),
        engine: binding.engine.clone(),
        configuration,
        container,
        name: name.clone(),
        output: binding.host.join("Saved/UnrealDevFlow/Packages").join(name),
        disabled_plugins: Vec::new(),
        revision: 0,
        last_reason: String::new(),
        content_digest: String::new(),
        project_settings: project_settings.cloned(),
        cook_mode: CookMode::Iterate,
    }
}

fn default_name(binding: &Binding, configuration: &str) -> String {
    let project = binding
        .project
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project");
    let scope = binding.task_uid.rsplit('/').next().unwrap_or("workspace");
    format!("{project}-{scope}-Win64-{configuration}")
}

fn normalize_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return Err(UdfError::Other("package name 不能为空或为 . / ..".into()));
    }
    if name
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
    {
        return Err(UdfError::Other(
            "package name 只能包含英文、数字、连字符、下划线和点".into(),
        ));
    }
    Ok(name.to_string())
}

fn reject_unknown(value: &toml::Value) -> Result<()> {
    let allowed = BTreeSet::from([
        "schema_version",
        "task_uid",
        "created",
        "host",
        "engine",
        "revision",
        "last_reason",
        "content_digest",
        "project_settings",
        "cook",
        "source",
        "build",
        "package",
        "plugins",
    ]);
    for key in value.as_table().into_iter().flat_map(|table| table.keys()) {
        if !allowed.contains(key.as_str()) {
            return Err(UdfError::Other(format!("unknown profile field: {key}")));
        }
    }
    Ok(())
}

fn reject_nested(value: &toml::Value, section: &str, allowed: &[&str]) -> Result<()> {
    let Some(table) = value.get(section).and_then(toml::Value::as_table) else {
        return Ok(());
    };
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(UdfError::Other(format!(
                "unknown profile field: {section}.{key}"
            )));
        }
    }
    Ok(())
}

fn profile_digest(profile: &PackageProfile) -> String {
    let value = (
        profile.schema_version,
        &profile.task_uid,
        &profile.created,
        &profile.host,
        &profile.project,
        &profile.engine,
        &profile.configuration,
        &profile.container,
        &profile.name,
        &profile.output,
        &profile.disabled_plugins,
        profile.revision,
        &profile.last_reason,
        &profile.project_settings,
        profile.cook_mode,
    );
    format!(
        "md5:{:x}",
        Md5::digest(serde_json::to_vec(&value).unwrap_or_default())
    )
}

fn legacy_profile_digest(profile: &PackageProfile) -> String {
    let value = (
        profile.schema_version,
        &profile.task_uid,
        &profile.created,
        &profile.host,
        &profile.project,
        &profile.engine,
        &profile.configuration,
        &profile.container,
        &profile.name,
        &profile.output,
        &profile.disabled_plugins,
        profile.revision,
        &profile.last_reason,
    );
    format!(
        "md5:{:x}",
        Md5::digest(serde_json::to_vec(&value).unwrap_or_default())
    )
}

fn project_profile_digest_without_cook(profile: &PackageProfile) -> String {
    let value = (
        profile.schema_version,
        &profile.task_uid,
        &profile.created,
        &profile.host,
        &profile.project,
        &profile.engine,
        &profile.configuration,
        &profile.container,
        &profile.name,
        &profile.output,
        &profile.disabled_plugins,
        profile.revision,
        &profile.last_reason,
        &profile.project_settings,
    );
    format!(
        "md5:{:x}",
        Md5::digest(serde_json::to_vec(&value).unwrap_or_default())
    )
}

fn load_profile(
    path: &Path,
    binding: &Binding,
    verify_digest: bool,
) -> Result<Option<PackageProfile>> {
    if !path.is_file() {
        return Ok(None);
    }
    let value: toml::Value = fs::read_to_string(path)?
        .parse()
        .map_err(|e| UdfError::Other(format!("配置 TOML 无效：{e}")))?;
    reject_unknown(&value)?;
    reject_nested(&value, "source", &["project"])?;
    reject_nested(&value, "build", &["configuration"])?;
    reject_nested(&value, "package", &["container", "name", "output"])?;
    reject_nested(&value, "plugins", &["disabled"])?;
    reject_nested(
        &value,
        "project_settings",
        &[
            "digest",
            "source_files",
            "source_digests",
            "packaging",
            "cooker",
            "maps",
        ],
    )?;
    reject_nested(&value, "cook", &["mode"])?;
    let mut profile = defaults(binding, None);
    let has_cook_section = value.get("cook").is_some();
    if !has_cook_section {
        // Profiles created before cook-mode existed were full package recipes.
        // Preserve that fixed behavior instead of changing it during upgrade.
        profile.cook_mode = CookMode::Full;
    }
    if let Some(table) = value.as_table() {
        if let Some(v) = table
            .get("source")
            .and_then(|v| v.get("project"))
            .and_then(toml::Value::as_str)
        {
            profile.project = v.into();
        }
        if let Some(v) = table
            .get("build")
            .and_then(|v| v.get("configuration"))
            .and_then(toml::Value::as_str)
        {
            profile.configuration = v.into();
        }
        if let Some(v) = table
            .get("package")
            .and_then(|v| v.get("container"))
            .and_then(toml::Value::as_str)
        {
            profile.container = Container::parse(v)?;
        }
        if let Some(v) = table
            .get("package")
            .and_then(|v| v.get("name"))
            .and_then(toml::Value::as_str)
        {
            profile.name = normalize_name(v)?;
        }
        if let Some(v) = table
            .get("package")
            .and_then(|v| v.get("output"))
            .and_then(toml::Value::as_str)
        {
            profile.output = v.into();
        }
        if let Some(v) = table
            .get("plugins")
            .and_then(|v| v.get("disabled"))
            .and_then(toml::Value::as_array)
        {
            profile.disabled_plugins = v
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_string)
                .collect();
        }
        if let Some(value) = table.get("project_settings") {
            profile.project_settings =
                Some(value.clone().try_into().map_err(|error| {
                    UdfError::Other(format!("project_settings 配置无效：{error}"))
                })?);
        }
        if let Some(value) = table
            .get("cook")
            .and_then(|value| value.get("mode"))
            .and_then(toml::Value::as_str)
        {
            profile.cook_mode = CookMode::parse(value)?;
        }
        if let Some(v) = table.get("revision").and_then(toml::Value::as_integer) {
            profile.revision = v as u64;
        }
        if let Some(v) = table.get("last_reason").and_then(toml::Value::as_str) {
            profile.last_reason = v.to_string();
        }
        if let Some(v) = table.get("content_digest").and_then(toml::Value::as_str) {
            profile.content_digest = v.to_string();
        }
        for (key, expected, actual) in [
            (
                "task_uid",
                &profile.task_uid,
                table
                    .get("task_uid")
                    .and_then(toml::Value::as_str)
                    .unwrap_or(""),
            ),
            (
                "created",
                &profile.created,
                table
                    .get("created")
                    .and_then(toml::Value::as_str)
                    .unwrap_or(""),
            ),
        ] {
            if table.contains_key(key) && actual != expected {
                return Err(UdfError::Other(format!(
                    "候选配置不属于当前 task 实例：{key}"
                )));
            }
        }
    }
    let expected_digest = if profile.project_settings.is_some() {
        if value.get("cook").is_some() {
            profile_digest(&profile)
        } else {
            project_profile_digest_without_cook(&profile)
        }
    } else {
        legacy_profile_digest(&profile)
    };
    if verify_digest
        && !profile.content_digest.is_empty()
        && profile.content_digest != expected_digest
    {
        return Err(UdfError::Other(
            "当前打包配置已被直接修改；请使用 package configure --file 接纳候选配置并提供 --reason"
                .into(),
        ));
    }
    Ok(Some(profile))
}

fn serialized_profile(profile: &PackageProfile) -> Result<String> {
    let mut value = toml::map::Map::new();
    value.insert(
        "schema_version".into(),
        toml::Value::Integer(profile.schema_version as i64),
    );
    value.insert("task_uid".into(), profile.task_uid.clone().into());
    value.insert("created".into(), profile.created.clone().into());
    value.insert(
        "host".into(),
        profile.host.to_string_lossy().to_string().into(),
    );
    value.insert(
        "engine".into(),
        profile.engine.to_string_lossy().to_string().into(),
    );
    value.insert(
        "revision".into(),
        toml::Value::Integer(profile.revision as i64),
    );
    value.insert("last_reason".into(), profile.last_reason.clone().into());
    value.insert(
        "content_digest".into(),
        profile.content_digest.clone().into(),
    );
    if let Some(settings) = &profile.project_settings {
        value.insert(
            "project_settings".into(),
            toml::Value::try_from(settings)
                .map_err(|error| UdfError::Other(format!("项目打包设置序列化失败：{error}")))?,
        );
    }
    value.insert(
        "cook".into(),
        toml::Value::Table(toml::map::Map::from_iter([(
            "mode".into(),
            format!("{:?}", profile.cook_mode)
                .to_ascii_lowercase()
                .into(),
        )])),
    );
    value.insert(
        "source".into(),
        toml::Value::Table(toml::map::Map::from_iter([(
            "project".into(),
            profile.project.to_string_lossy().to_string().into(),
        )])),
    );
    value.insert(
        "build".into(),
        toml::Value::Table(toml::map::Map::from_iter([(
            "configuration".into(),
            profile.configuration.clone().into(),
        )])),
    );
    value.insert(
        "package".into(),
        toml::Value::Table(toml::map::Map::from_iter([
            (
                "container".into(),
                format!("{:?}", profile.container)
                    .to_ascii_lowercase()
                    .into(),
            ),
            ("name".into(), profile.name.clone().into()),
            (
                "output".into(),
                profile.output.to_string_lossy().to_string().into(),
            ),
        ])),
    );
    value.insert(
        "plugins".into(),
        toml::Value::Table(toml::map::Map::from_iter([(
            "disabled".into(),
            toml::Value::Array(
                profile
                    .disabled_plugins
                    .iter()
                    .cloned()
                    .map(toml::Value::String)
                    .collect(),
            ),
        )])),
    );
    toml::to_string_pretty(&toml::Value::Table(value))
        .map_err(|e| UdfError::Other(format!("配置序列化失败：{e}")))
}

fn write_profile(path: &Path, profile: &mut PackageProfile) -> Result<()> {
    profile.content_digest = profile_digest(profile);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("toml.tmp.{}", std::process::id()));
    fs::write(&temporary, serialized_profile(profile)?)?;
    if path.exists() {
        let backup = path.with_extension(format!("toml.bak.{}", std::process::id()));
        fs::rename(path, &backup)?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::rename(&backup, path);
            return Err(error.into());
        }
        let _ = fs::remove_file(backup);
    } else {
        fs::rename(temporary, path)?;
    }
    Ok(())
}

pub fn configure(options: ConfigureOptions) -> Result<()> {
    if options.workspace.is_none() && options.task.is_none() {
        return Err(UdfError::Other(
            "configure 必须指定 --task 或 --workspace".into(),
        ));
    }
    let config = Config::load()?;
    let binding = resolve_binding(
        &config,
        options.workspace.as_deref(),
        options.task.as_deref(),
    )?;
    let project_settings = crate::project_packaging::load_snapshot(&binding.project)?;
    let path = profile_path(&binding)?;
    if options.file.is_some()
        && (options.configuration.is_some()
            || options.container.is_some()
            || options.cook_mode.is_some()
            || options.name.is_some()
            || options.output.is_some()
            || !options.disable_plugin.is_empty())
    {
        return Err(UdfError::Other("--file 不能和常用配置参数同时使用".into()));
    }
    // A complete candidate is the explicit escape hatch for accepting a
    // reviewed manual edit; ordinary configure calls must reject drift.
    let existing = load_profile(&path, &binding, options.file.is_none())?;
    let was_existing = existing.is_some();
    let mut profile = existing.unwrap_or_else(|| defaults(&binding, Some(&project_settings)));
    let before = profile.clone();
    if let Some(file) = options.file {
        profile = load_profile(&file, &binding, false)?
            .ok_or_else(|| UdfError::Other(format!("候选配置不存在：{}", file.display())))?;
        if was_existing && profile.revision != before.revision {
            return Err(UdfError::Other(format!(
                "候选配置 revision={} 已过时，当前 revision={}；请从当前配置重新生成候选",
                profile.revision, before.revision
            )));
        }
    } else {
        if profile.project_settings.is_none() {
            profile.project_settings = Some(project_settings.clone());
        }
        if let Some(value) = options.configuration {
            profile.configuration = value;
        }
        if let Some(value) = options.container {
            profile.container = Container::parse(&value)?;
        }
        if let Some(value) = options.cook_mode {
            profile.cook_mode = CookMode::parse(&value)?;
        }
        if let Some(value) = options.name.as_deref() {
            let name = normalize_name(value)?;
            let base = options
                .output
                .as_deref()
                .or_else(|| profile.output.parent())
                .ok_or_else(|| UdfError::Other("无法确定 package 输出根目录".into()))?;
            profile.name = name.clone();
            profile.output = base.join(name);
        }
        if let Some(value) = options.output {
            profile.output = if !was_existing || options.name.is_some() {
                value.join(&profile.name)
            } else {
                value
            };
        }
        for plugin in options.disable_plugin {
            if !profile.disabled_plugins.contains(&plugin) {
                profile.disabled_plugins.push(plugin);
            }
        }
        profile.disabled_plugins.sort();
    }
    let changed = !was_existing || profile != before;
    if changed {
        if before.revision > 0 && options.reason.as_deref().unwrap_or("").trim().is_empty() {
            return Err(UdfError::Other("修改已有配置必须提供 --reason".into()));
        }
        profile.revision = before.revision + 1;
        profile.last_reason = options
            .reason
            .clone()
            .unwrap_or_else(|| "首次创建打包配置".to_string());
        write_profile(&path, &mut profile)?;
    }
    crate::output::emit(
        "package configure",
        ConfigureResult {
            profile_path: path,
            configuration: profile.configuration,
            container: profile.container,
            name: profile.name,
            output: profile.output,
            disabled_plugins: profile.disabled_plugins,
            cook_mode: profile.cook_mode,
            revision: profile.revision,
            changed,
        },
        |r| {
            format!(
                "package configure: {} (revision {})",
                if r.changed { "saved" } else { "unchanged" },
                r.revision
            )
        },
    );
    Ok(())
}

pub fn load_for_project(
    config: &Config,
    workspace: Option<&str>,
    task: Option<&str>,
) -> Result<Option<PackageProfile>> {
    let binding = resolve_binding(config, workspace, task)?;
    let profile = load_profile(&profile_path(&binding)?, &binding, true)?;
    if let Some(profile) = profile.as_ref()
        && let Some(saved) = profile.project_settings.as_ref()
    {
        let current = crate::project_packaging::load_snapshot(&profile.project)?;
        if current.digest != saved.digest {
            return Err(UdfError::Other(
                "项目原生打包设置已变化；请使用 package configure --reason 显式更新固定配置".into(),
            ));
        }
    }
    Ok(profile)
}
