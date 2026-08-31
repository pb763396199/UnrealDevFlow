use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::host;
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
    pub output: PathBuf,
    #[serde(default)]
    pub disabled_plugins: Vec<String>,
    pub revision: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureResult {
    pub profile_path: PathBuf,
    pub configuration: String,
    pub container: Container,
    pub output: PathBuf,
    pub disabled_plugins: Vec<String>,
    pub revision: u64,
    pub changed: bool,
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

fn defaults(binding: &Binding) -> PackageProfile {
    PackageProfile {
        schema_version: 1,
        task_uid: binding.task_uid.clone(),
        created: binding.created.clone(),
        host: binding.host.clone(),
        project: binding.project.clone(),
        engine: binding.engine.clone(),
        configuration: "Development".into(),
        container: Container::Pak,
        output: binding.host.join("Saved/UnrealDevFlow/Packages/Win64"),
        disabled_plugins: Vec::new(),
        revision: 0,
    }
}

fn reject_unknown(value: &toml::Value) -> Result<()> {
    let allowed = BTreeSet::from([
        "schema_version",
        "task_uid",
        "created",
        "host",
        "project",
        "engine",
        "configuration",
        "container",
        "output",
        "disabled_plugins",
        "revision",
    ]);
    for key in value.as_table().into_iter().flat_map(|table| table.keys()) {
        if !allowed.contains(key.as_str()) {
            return Err(UdfError::Other(format!("unknown profile field: {key}")));
        }
    }
    Ok(())
}

fn load_profile(path: &Path, binding: &Binding) -> Result<Option<PackageProfile>> {
    if !path.is_file() {
        return Ok(None);
    }
    let value: toml::Value = fs::read_to_string(path)?
        .parse()
        .map_err(|e| UdfError::Other(format!("配置 TOML 无效：{e}")))?;
    reject_unknown(&value)?;
    let mut profile = defaults(binding);
    if let Some(table) = value.as_table() {
        if let Some(v) = table.get("configuration").and_then(toml::Value::as_str) {
            profile.configuration = v.into();
        }
        if let Some(v) = table.get("container").and_then(toml::Value::as_str) {
            profile.container = Container::parse(v)?;
        }
        if let Some(v) = table.get("output").and_then(toml::Value::as_str) {
            profile.output = v.into();
        }
        if let Some(v) = table
            .get("disabled_plugins")
            .and_then(toml::Value::as_array)
        {
            profile.disabled_plugins = v
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_string)
                .collect();
        }
        if let Some(v) = table.get("revision").and_then(toml::Value::as_integer) {
            profile.revision = v as u64;
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
    Ok(Some(profile))
}

pub fn configure(
    workspace: Option<String>,
    task: Option<String>,
    configuration: Option<String>,
    container: Option<String>,
    output: Option<PathBuf>,
    disable_plugin: Vec<String>,
    file: Option<PathBuf>,
    reason: Option<String>,
) -> Result<()> {
    if workspace.is_none() && task.is_none() {
        return Err(UdfError::Other(
            "configure 必须指定 --task 或 --workspace".into(),
        ));
    }
    let config = Config::load()?;
    let binding = resolve_binding(&config, workspace.as_deref(), task.as_deref())?;
    let path = profile_path(&binding)?;
    if file.is_some()
        && (configuration.is_some()
            || container.is_some()
            || output.is_some()
            || !disable_plugin.is_empty())
    {
        return Err(UdfError::Other("--file 不能和常用配置参数同时使用".into()));
    }
    let mut profile = load_profile(&path, &binding)?.unwrap_or_else(|| defaults(&binding));
    let before = profile.clone();
    if let Some(file) = file {
        profile = load_profile(&file, &binding)?
            .ok_or_else(|| UdfError::Other(format!("候选配置不存在：{}", file.display())))?;
    } else {
        if let Some(value) = configuration {
            profile.configuration = value;
        }
        if let Some(value) = container {
            profile.container = Container::parse(&value)?;
        }
        if let Some(value) = output {
            profile.output = value;
        }
        for plugin in disable_plugin {
            if !profile.disabled_plugins.contains(&plugin) {
                profile.disabled_plugins.push(plugin);
            }
        }
        profile.disabled_plugins.sort();
    }
    let changed = profile != before;
    if changed {
        if before.revision > 0 && reason.as_deref().unwrap_or("").trim().is_empty() {
            return Err(UdfError::Other("修改已有配置必须提供 --reason".into()));
        }
        profile.revision = before.revision + 1;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &path,
            toml::to_string_pretty(&profile)
                .map_err(|e| UdfError::Other(format!("配置序列化失败：{e}")))?,
        )?;
    }
    crate::output::emit(
        "package configure",
        ConfigureResult {
            profile_path: path,
            configuration: profile.configuration,
            container: profile.container,
            output: profile.output,
            disabled_plugins: profile.disabled_plugins,
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
    load_profile(&profile_path(&binding)?, &binding)
}
