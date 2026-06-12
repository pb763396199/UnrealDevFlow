//! Workspace registry commands.

use crate::config::{Config, DEFAULT_WORKSPACE, WorkspaceConfig, sanitize_workspace_name};
use crate::error::{Result, UdfError};
use crate::output;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn add(
    name: String,
    project: PathBuf,
    hosts_root: PathBuf,
    plugins_root: PathBuf,
    engine_path: Option<PathBuf>,
    plugin_path: Option<PathBuf>,
    skip_confirm: bool,
) -> Result<()> {
    let raw_name = name;
    let name = sanitize_workspace_name(&raw_name);
    if name != raw_name {
        output::print_info(&format!(
            "Workspace name normalized: {} -> {}",
            raw_name, name
        ));
    }
    let workspace = build_workspace(project, hosts_root, plugins_root, engine_path, plugin_path)?;
    validate_workspace(&workspace)?;

    output::print_info(&format!("Workspace '{}'", name));
    output::print_info(&format!("  Project: {:?}", workspace.default_project));
    output::print_info(&format!("  Plugins: {:?}", workspace.plugins_root));
    output::print_info(&format!("  Hosts:   {:?}", workspace.hosts_root));
    output::print_info(&format!("  Engine:  {:?}", workspace.engine_path));

    if !skip_confirm {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!("Save workspace '{}'?", name))
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
        if !confirmed {
            output::print_info("Workspace add cancelled.");
            return Ok(());
        }
    }

    let mut config = load_or_seed_config(&name, &workspace)?;
    config.upsert_workspace(name.clone(), workspace);
    config.save()?;
    output::print_success(&format!("Workspace '{}' saved.", name));
    Ok(())
}

pub fn list() -> Result<()> {
    let config = Config::load()?;
    println!("Workspaces:");
    println!("{:<20} {:<50} {:<50}", "Name", "Project", "Plugins");
    println!("{}", "-".repeat(124));
    if config.workspaces.is_empty() {
        let legacy = config.legacy_workspace();
        println!(
            "{:<20} {:<50} {:<50}",
            DEFAULT_WORKSPACE,
            legacy.default_project.display(),
            legacy
                .effective_plugins_root()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "-".to_string())
        );
        return Ok(());
    }
    for name in config.workspace_names() {
        let (_, ws) = config.resolve_workspace(Some(&name))?;
        println!(
            "{:<20} {:<50} {:<50}",
            name,
            ws.default_project.display(),
            ws.effective_plugins_root()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "-".to_string())
        );
    }
    Ok(())
}

pub fn doctor(name: &str) -> Result<()> {
    let name = sanitize_workspace_name(name);
    let config = Config::load()?;
    let (_, workspace) = config.resolve_workspace(Some(&name))?;
    validate_workspace(&workspace)?;
    output::print_success(&format!("Workspace '{}' looks good.", name));
    Ok(())
}

pub fn remove(name: &str, skip_confirm: bool) -> Result<()> {
    let name = sanitize_workspace_name(name);
    let mut config = Config::load()?;
    if !config.workspaces.contains_key(&name) {
        return Err(UdfError::Other(format!("workspace '{}' 不存在", name)));
    }
    if !skip_confirm {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!("Remove workspace '{}' registration?", name))
            .default(false)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
        if !confirmed {
            output::print_info("Workspace remove cancelled.");
            return Ok(());
        }
    }
    config.workspaces.remove(&name);
    if config.last_used_workspace.as_deref() == Some(name.as_str()) {
        config.last_used_workspace = None;
    }
    config.save()?;
    output::print_success(&format!("Workspace '{}' removed.", name));
    Ok(())
}

pub fn build_workspace(
    project: PathBuf,
    hosts_root: PathBuf,
    plugins_root: PathBuf,
    engine_path: Option<PathBuf>,
    plugin_path: Option<PathBuf>,
) -> Result<WorkspaceConfig> {
    let engine_path = match engine_path {
        Some(path) => path,
        None => crate::config::detect_engine_path(&project)?,
    };
    Ok(WorkspaceConfig {
        hosts_root,
        plugin_path,
        default_project: project,
        engine_path,
        plugins_root: Some(plugins_root),
        plugin_overrides: HashMap::new(),
    })
}

pub fn validate_workspace(workspace: &WorkspaceConfig) -> Result<()> {
    if !workspace.default_project.exists() {
        return Err(UdfError::Other(format!(
            "UE 项目不存在：{:?}",
            workspace.default_project
        )));
    }
    if find_uproject(&workspace.default_project).is_none() {
        return Err(UdfError::Other(format!(
            "UE 项目目录中没有 .uproject 文件：{:?}",
            workspace.default_project
        )));
    }
    let plugins_root = workspace.effective_plugins_root().ok_or_else(|| {
        UdfError::Other("workspace 未配置 plugins_root 或 plugin_path".to_string())
    })?;
    if !plugins_root.exists() {
        return Err(UdfError::Other(format!(
            "plugins_root 不存在：{:?}",
            plugins_root
        )));
    }
    if !workspace.hosts_root.exists() {
        std::fs::create_dir_all(&workspace.hosts_root)?;
    }
    let build_bat = workspace
        .engine_path
        .join("Engine")
        .join("Build")
        .join("BatchFiles")
        .join("Build.bat");
    if !build_bat.exists() {
        return Err(UdfError::Other(format!(
            "引擎 Build.bat 不存在：{:?}",
            build_bat
        )));
    }
    Ok(())
}

fn load_or_seed_config(name: &str, workspace: &WorkspaceConfig) -> Result<Config> {
    if Config::exists() {
        return Config::load();
    }
    Ok(Config {
        hosts_root: workspace.hosts_root.clone(),
        plugin_path: workspace.plugin_path.clone(),
        default_project: workspace.default_project.clone(),
        engine_path: workspace.engine_path.clone(),
        plugins_root: workspace.plugins_root.clone(),
        plugin_overrides: workspace.plugin_overrides.clone(),
        workspaces: HashMap::from([(name.to_string(), workspace.clone())]),
        last_used_workspace: Some(name.to_string()),
    })
}

fn find_uproject(project_dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(project_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|e| e == "uproject").unwrap_or(false))
}
