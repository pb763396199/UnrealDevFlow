//! Workspace registry commands.

use crate::config::{Config, DEFAULT_WORKSPACE, WorkspaceConfig, sanitize_workspace_name};
use crate::error::{GitError, Result, UdfError};
use crate::output;
use crate::plugin::scanner;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

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

pub fn doctor(name: &str, deep: bool) -> Result<()> {
    let name = sanitize_workspace_name(name);
    let config = Config::load()?;
    let (_, workspace) = config.resolve_workspace(Some(&name))?;
    validate_workspace(&workspace)?;
    if deep {
        let plugins_root = workspace.effective_plugins_root().ok_or_else(|| {
            UdfError::Other("workspace 未配置 plugins_root 或 plugin_path".to_string())
        })?;
        validate_plugins_root_sources(&workspace, &plugins_root)?;
        output::print_success(&format!("Workspace '{}' deep plugin scan passed.", name));
    }
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
    validate_plugins_root_shape(workspace, &plugins_root)?;
    validate_default_plugin_path(workspace, &plugins_root)?;
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

fn validate_default_plugin_path(workspace: &WorkspaceConfig, plugins_root: &Path) -> Result<()> {
    let Some(plugin_path) = &workspace.plugin_path else {
        return Ok(());
    };
    validate_plugin_source_path(workspace, "默认主插件", plugin_path, true)?;
    if !plugin_path.starts_with(plugins_root) {
        return Err(UdfError::Other(format!(
            "默认主插件不在 plugins_root 下：{:?}\nplugins_root: {:?}",
            plugin_path, plugins_root
        )));
    }
    Ok(())
}

pub fn validate_plugin_source_path(
    workspace: &WorkspaceConfig,
    label: &str,
    plugin_path: &Path,
    require_git_repo: bool,
) -> Result<()> {
    if !plugin_path.exists() {
        return Err(UdfError::Other(format!(
            "{}路径不存在：{:?}",
            label, plugin_path
        )));
    }
    crate::plugin::uplugin::find_uplugin_file(plugin_path)?;
    if plugin_path.starts_with(workspace.default_project.join("Plugins")) {
        return Err(UdfError::Other(format!(
            "{}位于 UE 项目 Plugins 入口：{:?}\n\
             这是 switch 会改写的可变入口，不能作为 source_repo。",
            label, plugin_path
        )));
    }
    if is_reparse_or_symlink(plugin_path) {
        return Err(UdfError::Other(format!(
            "{}是 Junction/symlink/reparse point：{:?}\n\
             请把 plugins_root 指向真实主仓路径，不能指向可变入口。",
            label, plugin_path
        )));
    }
    match crate::git::linked_worktree_main(plugin_path) {
        Ok(Some(main_worktree)) => {
            let suggested_root = main_worktree
                .parent()
                .map(|path| path.to_path_buf())
                .unwrap_or_else(|| main_worktree.clone());
            return Err(UdfError::Other(format!(
                "{}指向 Git linked worktree：{:?}\n\
                 这会让新任务错误地基于另一个任务分支创建。\n\
                 主 checkout 是：{:?}\n\
                 请把 workspace/config 的 plugins_root 改为主插件仓库根目录，例如：{:?}",
                label, plugin_path, main_worktree, suggested_root
            )));
        }
        Ok(None) => {}
        Err(UdfError::Git(GitError::NotARepo(_))) if require_git_repo => {
            return Err(UdfError::Other(format!(
                "{}不是 Git 仓库：{:?}",
                label, plugin_path
            )));
        }
        Err(UdfError::Git(GitError::CommandFailed(_))) if require_git_repo => {
            return Err(UdfError::Other(format!(
                "无法确认{}的 Git 主 checkout：{:?}",
                label, plugin_path
            )));
        }
        Err(UdfError::Git(GitError::NotARepo(_))) => {}
        Err(UdfError::Git(GitError::CommandFailed(_))) => {}
        Err(err) => return Err(err),
    }
    Ok(())
}

fn is_reparse_or_symlink(path: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if meta.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn validate_plugins_root_shape(workspace: &WorkspaceConfig, plugins_root: &Path) -> Result<()> {
    let root_name = plugins_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if root_name.eq_ignore_ascii_case("Hosts")
        || root_name.starts_with("W-")
        || (root_name.starts_with("T-") && root_name.ends_with("_Host"))
    {
        return Err(UdfError::Other(format!(
            "plugins_root 不能指向 Host/workspace 目录：{:?}",
            plugins_root
        )));
    }

    if plugins_root.starts_with(&workspace.default_project) {
        return Err(UdfError::Other(format!(
            "plugins_root 不能位于 UE 项目目录内：{:?}\n\
             请使用稳定的主插件仓库根目录，例如项目外层的 Plugins 目录。",
            plugins_root
        )));
    }

    if plugins_root.join("Hosts").exists() {
        return Err(UdfError::Other(format!(
            "plugins_root 看起来过宽，包含 Hosts 目录：{:?}\n\
             请指向只包含插件主仓库的目录。",
            plugins_root
        )));
    }

    if let Ok(entries) = std::fs::read_dir(plugins_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .map(|ext| ext == "uproject")
                .unwrap_or(false)
            {
                return Err(UdfError::Other(format!(
                    "plugins_root 看起来是 UE 项目目录而不是插件仓库根：{:?}",
                    plugins_root
                )));
            }
        }
    }

    Ok(())
}

pub fn validate_plugins_root_sources(
    workspace: &WorkspaceConfig,
    plugins_root: &Path,
) -> Result<()> {
    let plugin_locations = scanner::enumerate_plugin_locations(plugins_root);
    for (name, locations) in &plugin_locations {
        if locations.len() > 1 {
            let paths = locations
                .iter()
                .map(|path| format!("  - {:?}", path))
                .collect::<Vec<_>>()
                .join("\n");
            return Err(UdfError::Other(format!(
                "plugins_root 中发现重复插件 '{}'：\n{}\n请把 plugins_root 收窄到唯一的主插件仓库根目录。",
                name, paths
            )));
        }
    }

    let project_plugins_root = workspace.default_project.join("Plugins");
    for (name, locations) in plugin_locations {
        let Some(plugin_dir) = locations.into_iter().next() else {
            continue;
        };
        if plugin_dir.starts_with(&project_plugins_root) {
            return Err(UdfError::Other(format!(
                "插件 '{}' 位于 UE 项目 Plugins 入口：{:?}\n\
                 这是 switch 会改写的可变入口，不能作为 source_repo。",
                name, plugin_dir
            )));
        }
        if is_reparse_or_symlink(&plugin_dir) {
            return Err(UdfError::Other(format!(
                "插件 '{}' 是 Junction/symlink/reparse point：{:?}\n\
                 请把 plugins_root 指向真实主仓路径，不能指向可变入口。",
                name, plugin_dir
            )));
        }
        match crate::git::linked_worktree_main(&plugin_dir) {
            Ok(Some(main_worktree)) => {
                let suggested_root = main_worktree
                    .parent()
                    .map(|path| path.to_path_buf())
                    .unwrap_or_else(|| main_worktree.clone());
                return Err(UdfError::Other(format!(
                    "plugins_root 中的插件 '{}' 指向 Git linked worktree：{:?}\n\
                     这通常表示 workspace 配到了 UE 项目的 Plugins/Junction 目录，\
                     会导致新任务从当前激活任务分支创建。\n\
                     主 checkout 是：{:?}\n\
                     请把 workspace/config 的 plugins_root 改为主插件仓库根目录，例如：{:?}",
                    name, plugin_dir, main_worktree, suggested_root
                )));
            }
            Ok(None) => {}
            Err(UdfError::Git(GitError::NotARepo(_))) => {}
            Err(UdfError::Git(GitError::CommandFailed(_))) => {}
            Err(err) => return Err(err),
        }
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
