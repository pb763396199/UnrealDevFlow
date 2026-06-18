//! Configuration management for UnrealDevFlow
//!
//! ## Design (workspace-aware)
//!
//! **One global registry, many named workspaces**:
//! `~/.unrealdevflow/config.toml` stores machine-level environment settings.
//! Each workspace captures one UE project, its Plugins root, Engine path, and
//! Hosts root. New tasks snapshot that workspace into `.udf-meta.json`.
//!
//! Why not mutate a single global default per session?
//! - Concurrent agents can work on different UE projects on the same machine.
//! - A task must remain bound to the project/engine/hosts context it was
//!   created with, even if another session later initializes a different
//!   workspace.
//! - Low-level commands require an explicit workspace when the registry is
//!   ambiguous; user-friendly commands may suggest a name, but they do not
//!   silently cross project boundaries.
//!
//! This is the simplest design that handles the real scenarios:
//! 1. One main project on one machine → one global config
//! 2. Multiple machines (e.g. work + home) → each machine has its own
//!    `~/.unrealdevflow/config.toml`
//! 3. Multiple main projects on the same machine → named workspaces, explicit
//!    task refs like `uga-dev/my-task`

use crate::error::{Result, UdfError};
use dialoguer::Input;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_WORKSPACE: &str = "default";
pub const CONFIG_DIR_ENV: &str = "UNREALDEVFLOW_CONFIG_DIR";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub hosts_root: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_path: Option<PathBuf>,
    pub default_project: PathBuf,
    pub engine_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugins_root: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub plugin_overrides: HashMap<String, PathBuf>,
}

impl WorkspaceConfig {
    pub fn effective_plugins_root(&self) -> Option<PathBuf> {
        if let Some(root) = &self.plugins_root {
            return Some(root.clone());
        }
        self.plugin_path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
    }

    pub fn legacy_primary_plugin(&self) -> Option<String> {
        let plugin_path = self.plugin_path.as_ref()?;
        if let Ok(name) = crate::plugin::uplugin::read_plugin_name(plugin_path) {
            return Some(name);
        }
        plugin_path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hosts_root: PathBuf,
    /// v1 legacy field. Kept for backward compatibility. If `plugins_root` is
    /// also set, `plugins_root` wins for discovery, but `plugin_path` is still
    /// honored as the default primary candidate for tasks created interactively.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_path: Option<PathBuf>,
    pub default_project: PathBuf,
    pub engine_path: PathBuf,
    /// v2: root directory containing all project plugins (one Git repo per
    /// plugin). When set, `create` will discover plugins automatically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugins_root: Option<PathBuf>,
    /// v2: user overrides for plugin resolution. Map of plugin-name → absolute
    /// path. Overrides win over engine/project auto-discovery.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub plugin_overrides: HashMap<String, PathBuf>,
    /// Named workspace registry. New tasks freeze one of these workspace
    /// contexts into `.udf-meta.json` so concurrent sessions do not depend on
    /// the mutable top-level default.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub workspaces: HashMap<String, WorkspaceConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_workspace: Option<String>,
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        if let Ok(path) = std::env::var(CONFIG_DIR_ENV) {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }

        let home = dirs::home_dir()
            .ok_or_else(|| UdfError::Other("Could not determine home directory".to_string()))?;
        Ok(home.join(".unrealdevflow"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Load the global config. Required for every command.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Err(UdfError::NotConfigured);
        }
        let content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| UdfError::Other(format!("Failed to serialize config: {}", e)))?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn exists() -> bool {
        Self::config_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Resolve the project plugins root, falling back to the parent of the v1
    /// `plugin_path` if `plugins_root` is not set.
    pub fn effective_plugins_root(&self) -> Option<PathBuf> {
        self.legacy_workspace().effective_plugins_root()
    }

    /// Resolve the default primary plugin candidate for interactive `create`,
    /// derived from the legacy v1 `plugin_path` field if present.
    pub fn legacy_primary_plugin(&self) -> Option<String> {
        self.legacy_workspace().legacy_primary_plugin()
    }

    pub fn legacy_workspace(&self) -> WorkspaceConfig {
        WorkspaceConfig {
            hosts_root: self.hosts_root.clone(),
            plugin_path: self.plugin_path.clone(),
            default_project: self.default_project.clone(),
            engine_path: self.engine_path.clone(),
            plugins_root: self.plugins_root.clone(),
            plugin_overrides: self.plugin_overrides.clone(),
        }
    }

    pub fn workspace_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.workspaces.keys().cloned().collect();
        if names.is_empty() {
            names.push(DEFAULT_WORKSPACE.to_string());
        }
        names.sort();
        names
    }

    pub fn resolve_workspace(&self, requested: Option<&str>) -> Result<(String, WorkspaceConfig)> {
        if let Some(name) = requested {
            let normalized = sanitize_workspace_name(name);
            if let Some(ws) = self.workspaces.get(&normalized) {
                return Ok((normalized, ws.clone()));
            }
            if normalized == DEFAULT_WORKSPACE {
                return Ok((DEFAULT_WORKSPACE.to_string(), self.legacy_workspace()));
            }
            return Err(UdfError::Other(format!(
                "workspace '{}' 不存在。可用 workspace: {}",
                name,
                self.workspace_names().join(", ")
            )));
        }

        if self.workspaces.len() == 1 {
            let (name, ws) = self.workspaces.iter().next().unwrap();
            return Ok((name.clone(), ws.clone()));
        }

        if self.workspaces.is_empty() {
            return Ok((DEFAULT_WORKSPACE.to_string(), self.legacy_workspace()));
        }

        Err(UdfError::Other(format!(
            "存在多个 workspace，请显式指定 --workspace。可用 workspace: {}",
            self.workspace_names().join(", ")
        )))
    }

    pub fn upsert_workspace(&mut self, name: String, workspace: WorkspaceConfig) {
        self.last_used_workspace = Some(name.clone());
        self.workspaces.insert(name, workspace);
    }

    pub fn suggested_workspace_name(project_path: &Path) -> String {
        let raw = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(DEFAULT_WORKSPACE);
        sanitize_workspace_name(raw)
    }
}

pub fn sanitize_workspace_name(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        DEFAULT_WORKSPACE.to_string()
    } else {
        out
    }
}

/// Detect engine path from EngineAssociation version string
fn detect_engine_path_by_version(version: &str) -> Option<PathBuf> {
    let common_paths = [
        format!("D:\\Unreal Engine\\UE_{}", version),
        format!("E:\\Unreal Engine\\UE_{}", version),
        format!("C:\\Program Files\\Epic Games\\UE_{}", version),
        format!("D:\\Epic Games\\UE_{}", version),
        format!("E:\\Epic Games\\UE_{}", version),
    ];

    for path in &common_paths {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Detect engine path from .uproject file in the given project directory
pub fn detect_engine_path(project_path: &PathBuf) -> Result<PathBuf> {
    let engine_version = read_engine_version(project_path)?;

    match detect_engine_path_by_version(&engine_version) {
        Some(path) => Ok(path),
        None => Err(UdfError::Other(format!(
            "Could not auto-detect engine path for version '{}'. Please specify --engine-path",
            engine_version
        ))),
    }
}

/// Read EngineAssociation from .uproject file
fn read_engine_version(project_path: &PathBuf) -> Result<String> {
    let mut uproject_path = None;
    if let Ok(entries) = fs::read_dir(project_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "uproject").unwrap_or(false) {
                uproject_path = Some(path);
                break;
            }
        }
    }

    let uproject_path = uproject_path
        .ok_or_else(|| UdfError::Other(format!("No .uproject file found in {:?}", project_path)))?;

    let content = fs::read_to_string(&uproject_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| UdfError::Other(format!("Failed to parse .uproject: {}", e)))?;

    json.get("EngineAssociation")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| UdfError::Other("EngineAssociation not found in .uproject".to_string()))
}

pub fn run_configure() -> Result<Config> {
    println!("Welcome to UnrealDevFlow configuration!\n");

    let hosts_root: String = Input::new()
        .with_prompt("Hosts root directory (where task Hosts will be created)")
        .default("F:\\ShanghaiP4\\neon\\Hosts".to_string())
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    let plugins_root: String = Input::new()
        .with_prompt("Plugins root directory (folder containing all project plugin Git repos)")
        .default("F:\\ShanghaiP4\\neon\\Plugins".to_string())
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    let default_project: String = Input::new()
        .with_prompt("Default UE project path (will read EngineAssociation from .uproject)")
        .default("F:\\ShanghaiP4\\neon\\UGA\\DEV".to_string())
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    let default_project_path = PathBuf::from(&default_project);

    println!("\nDetecting engine path from .uproject...");
    let engine_version = read_engine_version(&default_project_path)?;
    println!("  Engine version: {}", engine_version);

    let engine_path = match detect_engine_path_by_version(&engine_version) {
        Some(path) => {
            println!("  Engine path: {:?}", path);
            path
        }
        None => {
            println!(
                "  Could not auto-detect engine path for version {}",
                engine_version
            );
            let manual_path: String = Input::new()
                .with_prompt("Please enter UE engine path manually")
                .interact_text()
                .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;
            PathBuf::from(manual_path)
        }
    };

    let config = Config {
        hosts_root: PathBuf::from(hosts_root),
        plugin_path: None,
        default_project: default_project_path,
        engine_path,
        plugins_root: Some(PathBuf::from(plugins_root)),
        plugin_overrides: HashMap::new(),
        workspaces: HashMap::new(),
        last_used_workspace: None,
    };

    config.save()?;

    println!("\nConfiguration saved to: {:?}", Config::config_path()?);
    Ok(config)
}
