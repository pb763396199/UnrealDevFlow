//! Configuration management for UnrealDevFlow

use crate::error::{Result, UdfError};
use dialoguer::Input;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hosts_root: PathBuf,
    pub plugin_path: PathBuf,
    pub default_project: PathBuf,
    pub engine_path: PathBuf,
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            UdfError::Other("Could not determine home directory".to_string())
        })?;
        Ok(home.join(".unrealdevflow"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

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
}

/// Detect engine path from EngineAssociation version string
fn detect_engine_path(version: &str) -> Option<PathBuf> {
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

/// Read EngineAssociation from .uproject file
fn read_engine_version(project_path: &PathBuf) -> Result<String> {
    // Find .uproject file in project directory
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

    let uproject_path = uproject_path.ok_or_else(|| {
        UdfError::Other(format!("No .uproject file found in {:?}", project_path))
    })?;

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

    // 1. Hosts root directory (required)
    let hosts_root: String = Input::new()
        .with_prompt("Hosts root directory (where task Hosts will be created)")
        .default("F:\\ShanghaiP4\\neon\\Hosts".to_string())
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    // 2. Plugin main repository path (required)
    let plugin_path: String = Input::new()
        .with_prompt("Plugin main repository path (Git repo root)")
        .default("F:\\ShanghaiP4\\neon\\Plugins\\AesWorld".to_string())
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    // 3. Default UE project path (required)
    let default_project: String = Input::new()
        .with_prompt("Default UE project path (will read EngineAssociation from .uproject)")
        .default("F:\\ShanghaiP4\\neon\\UGA\\DEV".to_string())
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    let default_project_path = PathBuf::from(&default_project);

    // 4. Auto-detect engine path from .uproject
    println!("\nDetecting engine path from .uproject...");
    let engine_version = read_engine_version(&default_project_path)?;
    println!("  Engine version: {}", engine_version);

    let engine_path = match detect_engine_path(&engine_version) {
        Some(path) => {
            println!("  Engine path: {:?}", path);
            path
        }
        None => {
            println!("  Could not auto-detect engine path for version {}", engine_version);
            let manual_path: String = Input::new()
                .with_prompt("Please enter UE engine path manually")
                .interact_text()
                .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;
            PathBuf::from(manual_path)
        }
    };

    let config = Config {
        hosts_root: PathBuf::from(hosts_root),
        plugin_path: PathBuf::from(plugin_path),
        default_project: default_project_path,
        engine_path,
    };

    config.save()?;

    println!("\nConfiguration saved to: {:?}", Config::config_path()?);
    Ok(config)
}
