//! Configuration management for UnrealDevFlow

use crate::error::{Result, UdfError};
use dialoguer::{Input, Confirm};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hosts_root: PathBuf,
    pub plugin_path: PathBuf,
    pub default_project: Option<PathBuf>,
    pub engine_path: Option<PathBuf>,
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

pub fn run_configure() -> Result<Config> {
    println!("Welcome to UnrealDevFlow configuration!\n");

    let hosts_root: String = Input::new()
        .with_prompt("Hosts root directory")
        .default("F:\\ShanghaiP4\\neon\\Hosts".to_string())
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    let plugin_path: String = Input::new()
        .with_prompt("Plugin main repository path")
        .default("F:\\ShanghaiP4\\neon\\Plugins\\AesWorld".to_string())
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    let default_project: String = Input::new()
        .with_prompt("Default UE project path (optional, press Enter to skip)")
        .default("F:\\ShanghaiP4\\neon\\UGA\\DEV_5_7".to_string())
        .allow_empty(true)
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    let engine_path: String = Input::new()
        .with_prompt("UE engine path (optional, press Enter to skip)")
        .default("D:\\Unreal Engine\\UE_5.7".to_string())
        .allow_empty(true)
        .interact_text()
        .map_err(|e| UdfError::Other(format!("Input error: {}", e)))?;

    let config = Config {
        hosts_root: PathBuf::from(hosts_root),
        plugin_path: PathBuf::from(plugin_path),
        default_project: if default_project.is_empty() { None } else { Some(PathBuf::from(default_project)) },
        engine_path: if engine_path.is_empty() { None } else { Some(PathBuf::from(engine_path)) },
    };

    config.save()?;

    println!("\nConfiguration saved to: {:?}", Config::config_path()?);
    Ok(config)
}
