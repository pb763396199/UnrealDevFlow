//! `.uplugin` parsing.
//!
//! A `.uplugin` is JSON describing an Unreal plugin. We only need the
//! `"Plugins"` array, which lists this plugin's dependencies.

use crate::error::{Result, UdfError};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct UPluginDependency {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Enabled", default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct UPluginFile {
    #[serde(rename = "Plugins", default)]
    plugins: Vec<UPluginDependency>,
}

/// Locate the `.uplugin` file inside a plugin directory.
pub fn find_uplugin_file(plugin_dir: &Path) -> Result<PathBuf> {
    if !plugin_dir.is_dir() {
        return Err(UdfError::Other(format!(
            "插件目录不存在：{:?}",
            plugin_dir
        )));
    }
    for entry in fs::read_dir(plugin_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "uplugin").unwrap_or(false) {
            return Ok(path);
        }
    }
    Err(UdfError::Other(format!(
        "插件目录中找不到 .uplugin 文件：{:?}",
        plugin_dir
    )))
}

/// Read the dependency list from a plugin directory.
///
/// Returns only `Enabled = true` dependencies; missing fields default to false
/// and are filtered out.
pub fn read_dependencies(plugin_dir: &Path) -> Result<Vec<String>> {
    let uplugin_path = find_uplugin_file(plugin_dir)?;
    let content = fs::read_to_string(&uplugin_path)?;
    let parsed: UPluginFile = serde_json::from_str(&content).map_err(|e| {
        UdfError::Other(format!(
            "解析 .uplugin 失败 {:?}：{}",
            uplugin_path, e
        ))
    })?;

    Ok(parsed
        .plugins
        .into_iter()
        .filter(|d| d.enabled)
        .map(|d| d.name)
        .collect())
}
