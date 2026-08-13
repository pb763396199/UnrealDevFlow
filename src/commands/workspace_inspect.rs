//! Inspect resolved workspace source context.

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::output;
use crate::plugin::uplugin;
use crate::source_context::{
    SourceContext, SourcePlugin, WorkspaceSourceInput, resolve_workspace_source,
};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInspection {
    pub workspace: String,
    pub project: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uproject: Option<PathBuf>,
    pub engine: PathBuf,
    pub plugins_root: PathBuf,
    pub source_revision: String,
    pub source: SourceContext,
}

pub fn run(name: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let inspection = inspect_workspace(&config, name.as_deref())?;
    output::emit("workspace inspect", inspection, render_inspection);
    Ok(())
}

pub fn inspect_workspace(config: &Config, requested: Option<&str>) -> Result<WorkspaceInspection> {
    let (workspace, resolved) = config.resolve_workspace(requested)?;
    let plugins_root = resolved.effective_plugins_root().ok_or_else(|| {
        UdfError::Other("workspace has no plugins_root or plugin_path".to_string())
    })?;
    let source_revision = resolved
        .plugin_path
        .as_deref()
        .or(Some(plugins_root.as_path()))
        .map(git_revision)
        .unwrap_or_else(|| "unknown".to_string());
    let primary_plugins = resolved
        .plugin_path
        .as_ref()
        .map(|path| {
            let name = uplugin::read_plugin_name(path)
                .or_else(|_| plugin_name_from_path(path))
                .unwrap_or_else(|_| "unknown".to_string());
            SourcePlugin::primary(name, path.clone(), source_revision.clone())
        })
        .into_iter()
        .collect();
    let source = resolve_workspace_source(
        WorkspaceSourceInput::new(
            workspace.clone(),
            resolved.default_project.clone(),
            resolved.engine_path.clone(),
            plugins_root.clone(),
            source_revision.clone(),
        )
        .with_primary_plugins(primary_plugins),
    );

    Ok(WorkspaceInspection {
        workspace,
        project: resolved.default_project.clone(),
        uproject: find_uproject(&resolved.default_project),
        engine: resolved.engine_path,
        plugins_root,
        source_revision,
        source,
    })
}

fn render_inspection(data: &WorkspaceInspection) -> String {
    let mut lines = vec![
        format!("Workspace: {}", data.workspace),
        format!("Project:   {}", data.project.display()),
        format!(
            "UProject:   {}",
            data.uproject
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!("Engine:    {}", data.engine.display()),
        format!("Plugins:   {}", data.plugins_root.display()),
        format!("Revision:  {}", data.source_revision),
    ];
    if data.source.primary_plugins.is_empty() {
        lines.push("Primary plugins: -".to_string());
    } else {
        lines.push("Primary plugins:".to_string());
        for plugin in &data.source.primary_plugins {
            lines.push(format!("  - {} ({})", plugin.name, plugin.path.display()));
        }
    }
    lines.join("\n")
}

fn find_uproject(project_dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(project_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .map(|ext| ext == "uproject")
                .unwrap_or(false)
        })
}

fn plugin_name_from_path(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| {
            UdfError::Other(format!(
                "cannot infer plugin name from path: {}",
                path.display()
            ))
        })
}

fn git_revision(path: &Path) -> String {
    crate::git::command_stdout(path, &["rev-parse", "HEAD"])
        .unwrap_or_else(|_| "unknown".to_string())
}
