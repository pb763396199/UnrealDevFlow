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
    skip_skill_install: bool,
) -> Result<()> {
    let project = match project {
        Some(path) => path,
        None => detect_project_from_cwd()?,
    };
    let project = absolutize_path(project)?;
    let plugins_root = match plugins_root {
        Some(path) => path,
        None => infer_plugins_root(&project)?,
    };
    let plugins_root = absolutize_path(plugins_root)?;
    let (plugins_root, mut default_plugin_path) = normalize_plugins_root_input(plugins_root);
    if default_plugin_path.is_none()
        && let Ok(cwd) = std::env::current_dir()
    {
        default_plugin_path = detect_plugin_from_path(&cwd, &plugins_root);
    }
    if let Some(path) = default_plugin_path.take() {
        default_plugin_path = Some(absolutize_path(path)?);
    }
    let hosts_root = match hosts_root {
        Some(path) => path,
        None => infer_hosts_root(&project),
    };
    let hosts_root = absolutize_path(hosts_root)?;
    let engine_path = match engine_path {
        Some(path) => Some(absolutize_path(path)?),
        None => None,
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
    if let Some(plugin_path) = &default_plugin_path {
        output::print_info(&format!("  Default primary plugin: {:?}", plugin_path));
    }
    output::print_info(&format!("  Hosts:   {:?}", hosts_root));
    if let Some(engine) = &engine_path {
        output::print_info(&format!("  Engine:  {:?}", engine));
    } else {
        output::print_info("  Engine:  auto-detect from .uproject");
    }

    // Call the inner forms: `init` is one command, so it emits one envelope.
    let saved = crate::commands::workspace::add_inner(
        workspace_name.clone(),
        project,
        hosts_root,
        plugins_root,
        engine_path,
        default_plugin_path,
        skip_confirm,
    )?;
    let healthy = crate::commands::workspace::doctor_inner(&workspace_name, false)?.healthy;

    if skip_skill_install {
        output::print_warning("AI skill install skipped by --skip-skill-install.");
    } else {
        crate::commands::skills::install_inner(true, None)?;
    }

    output::emit(
        "workspace init",
        InitSummary {
            workspace: workspace_name,
            saved,
            healthy,
            skill_installed: !skip_skill_install,
        },
        render_init,
    );
    Ok(())
}

/// What `workspace init` set up, in one place instead of scattered log lines.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InitSummary {
    workspace: String,
    #[serde(flatten)]
    saved: crate::commands::workspace::WorkspaceSaved,
    healthy: bool,
    skill_installed: bool,
}

fn render_init(data: &InitSummary) -> String {
    let mut lines = vec![format!("✓ Workspace '{}' is ready.", data.workspace)];
    if data.healthy {
        lines.push("  Health check: passed".to_string());
    }
    lines.push(format!(
        "  AI skill: {}",
        if data.skill_installed {
            "installed"
        } else {
            "skipped"
        }
    ));
    lines.join("\n")
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

fn absolutize_path(path: PathBuf) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(dunce::canonicalize(&absolute).unwrap_or(absolute))
}

fn contains_uplugin(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "uplugin")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn normalize_plugins_root_input(path: PathBuf) -> (PathBuf, Option<PathBuf>) {
    if contains_uplugin(&path)
        && let Some(parent) = path.parent()
    {
        return (parent.to_path_buf(), Some(path));
    }
    (path, None)
}

fn detect_plugin_from_path(path: &Path, plugins_root: &Path) -> Option<PathBuf> {
    let root = dunce::canonicalize(plugins_root).unwrap_or_else(|_| plugins_root.to_path_buf());
    let start = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !start.starts_with(&root) {
        return None;
    }
    for dir in start.ancestors() {
        if dir == root {
            break;
        }
        if contains_uplugin(dir) {
            return Some(dir.to_path_buf());
        }
    }
    None
}

fn infer_plugins_root(project: &Path) -> Result<PathBuf> {
    let mut project_plugins = None;
    for ancestor in project.ancestors() {
        let candidate = ancestor.join("Plugins");
        if candidate.exists() {
            if !candidate.starts_with(project) {
                return Ok(candidate);
            }
            project_plugins.get_or_insert(candidate);
        }
    }
    if let Some(candidate) = project_plugins {
        return Err(UdfError::Other(format!(
            "只找到 UE 项目内的 Plugins 目录：{:?}\n\
             该目录通常由 switch 改写，不能作为稳定主插件仓库根。\n\
             请使用 --plugins-root 显式指定项目外层主插件仓库根目录。",
            candidate
        )));
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn infer_plugins_root_prefers_outer_repo_root_over_project_plugins() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let project = root.join("UGA").join("DEV");
        let project_plugins = project.join("Plugins");
        let repo_plugins = root.join("Plugins");
        std::fs::create_dir_all(&project_plugins).expect("project plugins");
        std::fs::create_dir_all(&repo_plugins).expect("repo plugins");

        assert_eq!(
            infer_plugins_root(&project).expect("plugins root"),
            repo_plugins
        );
    }

    #[test]
    fn infer_plugins_root_rejects_project_plugins_when_no_outer_root_exists() {
        let temp = TempDir::new().expect("temp dir");
        let project = temp.path().join("DEV");
        let project_plugins = project.join("Plugins");
        std::fs::create_dir_all(&project_plugins).expect("project plugins");

        assert!(infer_plugins_root(&project).is_err());
    }

    #[test]
    fn normalize_plugins_root_accepts_primary_plugin_directory() {
        let temp = TempDir::new().expect("temp dir");
        let plugins_root = temp.path().join("Plugins");
        let plugin = plugins_root.join("AesWorld");
        std::fs::create_dir_all(&plugin).expect("plugin dir");
        std::fs::write(plugin.join("AesWorld.uplugin"), "{}").expect("uplugin");

        let (normalized_root, default_plugin) = normalize_plugins_root_input(plugin.clone());

        assert_eq!(normalized_root, plugins_root);
        assert_eq!(default_plugin, Some(plugin));
    }

    #[test]
    fn detect_plugin_from_path_finds_plugin_ancestor_under_root() {
        let temp = TempDir::new().expect("temp dir");
        let plugins_root = temp.path().join("Plugins");
        let plugin = plugins_root.join("AesWorld");
        let source = plugin.join("Source").join("AesWorld");
        std::fs::create_dir_all(&source).expect("source dir");
        std::fs::write(plugin.join("AesWorld.uplugin"), "{}").expect("uplugin");

        assert_eq!(
            detect_plugin_from_path(&source, &plugins_root),
            Some(plugin)
        );
    }

    #[test]
    fn detect_plugin_from_path_ignores_paths_outside_root() {
        let temp = TempDir::new().expect("temp dir");
        let plugins_root = temp.path().join("Plugins");
        let other = temp.path().join("Other").join("AesWorld");
        std::fs::create_dir_all(&other).expect("other dir");
        std::fs::write(other.join("AesWorld.uplugin"), "{}").expect("uplugin");

        assert_eq!(detect_plugin_from_path(&other, &plugins_root), None);
    }
}
