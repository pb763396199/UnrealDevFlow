//! Friendly first-run initializer.

use crate::error::{Result, UdfError};
use crate::output;
use std::path::{Path, PathBuf};

pub fn run(
    workspace: Option<String>,
    project: Option<PathBuf>,
    plugins_root: Option<PathBuf>,
    hosts_root: Option<PathBuf>,
    engine_path: Option<PathBuf>,
    skip_confirm: bool,
) -> Result<()> {
    let project = match project {
        Some(path) => path,
        None => detect_project_from_cwd()?,
    };
    let project = dunce::canonicalize(&project).unwrap_or(project);
    let plugins_root = match plugins_root {
        Some(path) => path,
        None => infer_plugins_root(&project)?,
    };
    let hosts_root = match hosts_root {
        Some(path) => path,
        None => infer_hosts_root(&project),
    };
    let suggested = workspace.unwrap_or_else(|| suggest_workspace_name(&project));
    let workspace_name = if skip_confirm {
        suggested
    } else {
        dialoguer::Input::new()
            .with_prompt("Workspace name")
            .default(suggested)
            .interact_text()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?
    };

    output::print_info("Detected workspace:");
    output::print_info(&format!("  Project: {:?}", project));
    output::print_info(&format!("  Plugins: {:?}", plugins_root));
    output::print_info(&format!("  Hosts:   {:?}", hosts_root));
    if let Some(engine) = &engine_path {
        output::print_info(&format!("  Engine:  {:?}", engine));
    } else {
        output::print_info("  Engine:  auto-detect from .uproject");
    }

    crate::commands::workspace::add(
        workspace_name.clone(),
        project,
        hosts_root,
        plugins_root,
        engine_path,
        None,
        skip_confirm,
    )?;

    if let Err(e) = crate::commands::skills::install(true, None) {
        output::print_warning(&format!("AI skill install skipped/failed: {}", e));
    }
    crate::commands::workspace::doctor(&workspace_name)?;
    Ok(())
}

fn detect_project_from_cwd() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    for dir in cwd.ancestors() {
        if contains_uproject(dir) {
            return Ok(dir.to_path_buf());
        }
    }
    Err(UdfError::Other(
        "无法自动找到 .uproject，请使用 --project 指定 UE 项目目录".to_string(),
    ))
}

fn contains_uproject(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "uproject")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn infer_plugins_root(project: &Path) -> Result<PathBuf> {
    for ancestor in project.ancestors() {
        let candidate = ancestor.join("Plugins");
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(UdfError::Other(
        "无法自动找到 Plugins 目录，请使用 --plugins-root 指定".to_string(),
    ))
}

fn infer_hosts_root(project: &Path) -> PathBuf {
    for ancestor in project.ancestors() {
        let candidate = ancestor.join("Hosts");
        if candidate.exists() {
            return candidate;
        }
    }
    project
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(project)
        .join("Hosts")
}

fn suggest_workspace_name(project: &Path) -> String {
    let leaf = project
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("dev");
    let root = project
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(leaf);
    crate::config::sanitize_workspace_name(&format!("{}-{}", root, leaf))
}
