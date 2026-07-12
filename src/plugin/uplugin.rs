//! `.uplugin` descriptor discovery and parsing.

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
pub struct UPluginModule {
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct UPluginFile {
    #[serde(rename = "Plugins", default)]
    plugins: Vec<UPluginDependency>,
    #[serde(rename = "Modules", default)]
    modules: Vec<UPluginModule>,
}

/// Locate every descriptor in either a single-plugin directory or a repository
/// bundle containing several Unreal plugins.
pub fn find_uplugin_files(plugin_dir: &Path) -> Result<Vec<PathBuf>> {
    if !plugin_dir.is_dir() {
        return Err(UdfError::Other(format!("插件目录不存在：{:?}", plugin_dir)));
    }

    let mut descriptors = Vec::new();
    let mut pending = vec![plugin_dir.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                if !matches!(
                    name.as_str(),
                    ".git" | "binaries" | "intermediate" | "saved" | "deriveddatacache"
                ) {
                    pending.push(path);
                }
            } else if path
                .extension()
                .map(|ext| ext == "uplugin")
                .unwrap_or(false)
            {
                descriptors.push(path);
            }
        }
    }
    descriptors.sort();
    Ok(descriptors)
}

pub fn find_uplugin_file(plugin_dir: &Path) -> Result<PathBuf> {
    find_uplugin_files(plugin_dir)?
        .into_iter()
        .next()
        .ok_or_else(|| UdfError::Other(format!("插件目录中找不到 .uplugin 文件：{:?}", plugin_dir)))
}

pub fn read_plugin_names(plugin_dir: &Path) -> Result<Vec<String>> {
    let names = find_uplugin_files(plugin_dir)?
        .into_iter()
        .filter_map(|path| path.file_stem()?.to_str().map(str::to_string))
        .collect::<Vec<_>>();
    if names.is_empty() {
        return Err(UdfError::Other(format!(
            "插件目录中找不到 .uplugin 文件：{:?}",
            plugin_dir
        )));
    }
    Ok(names)
}

pub fn read_plugin_name(plugin_dir: &Path) -> Result<String> {
    let uplugin_path = find_uplugin_file(plugin_dir)?;
    uplugin_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
        .ok_or_else(|| UdfError::Other(format!("无法解析插件名：{:?}", uplugin_path)))
}

fn parse_descriptor(path: &Path) -> Result<UPluginFile> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|error| UdfError::Other(format!("解析 .uplugin 失败 {:?}：{}", path, error)))
}

pub fn read_dependencies(plugin_dir: &Path) -> Result<Vec<String>> {
    let mut dependencies = Vec::new();
    for descriptor in find_uplugin_files(plugin_dir)? {
        dependencies.extend(
            parse_descriptor(&descriptor)?
                .plugins
                .into_iter()
                .filter(|dependency| dependency.enabled)
                .map(|dependency| dependency.name),
        );
    }
    dependencies.sort();
    dependencies.dedup();
    Ok(dependencies)
}

pub fn read_module_names(plugin_dir: &Path) -> Result<Vec<String>> {
    let mut modules = Vec::new();
    for descriptor in find_uplugin_files(plugin_dir)? {
        modules.extend(
            parse_descriptor(&descriptor)?
                .modules
                .into_iter()
                .map(|module| module.name.trim().to_string())
                .filter(|name| !name.is_empty()),
        );
    }
    modules.sort();
    modules.dedup();
    Ok(modules)
}
