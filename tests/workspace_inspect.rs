#![allow(dead_code)]

#[path = "../src/source_context.rs"]
mod source_context;

use source_context::{SourceKind, SourcePlugin, WorkspaceSourceInput, resolve_workspace_source};
use std::path::PathBuf;

#[test]
fn workspace_source_helper_resolves_workspace_context_without_host() {
    let project = PathBuf::from(r"F:\ShanghaiP4\neon\UGA\DEV_1");
    let engine = PathBuf::from(r"F:\Unreal Engine\UE_5.5");
    let plugins_root = PathBuf::from(r"F:\ShanghaiP4\neon\Plugins");
    let plugin_path = plugins_root.join("AesWorld");

    let source = resolve_workspace_source(
        WorkspaceSourceInput::new(
            "neon-dev1",
            project.clone(),
            engine.clone(),
            plugins_root.clone(),
            "abc123",
        )
        .with_primary_plugins(vec![SourcePlugin::primary(
            "AesWorld",
            plugin_path.clone(),
            "abc123",
        )]),
    );

    assert_eq!(source.source_ref.kind, SourceKind::Workspace);
    assert_eq!(source.source_ref.value, "neon-dev1");
    assert_eq!(source.workspace, "neon-dev1");
    assert_eq!(source.project, project);
    assert_eq!(source.engine, engine);
    assert_eq!(source.plugins_root, plugins_root);
    assert_eq!(source.host, None);
    assert_eq!(source.source_revision, "abc123");
    assert_eq!(source.primary_plugins[0].name, "AesWorld");
    assert_eq!(source.primary_plugins[0].path, plugin_path);
}

#[test]
fn workspace_source_helper_keeps_dependency_plugins() {
    let plugins_root = PathBuf::from(r"F:\ShanghaiP4\neon\Plugins");

    let source = resolve_workspace_source(
        WorkspaceSourceInput::new(
            "neon-dev1",
            PathBuf::from(r"F:\ShanghaiP4\neon\UGA\DEV_1"),
            PathBuf::from(r"F:\Unreal Engine\UE_5.5"),
            plugins_root.clone(),
            "workspace-rev",
        )
        .with_dependency_plugins(vec![SourcePlugin::dependency(
            "AesShared",
            plugins_root.join("AesShared"),
            "dep-rev",
        )]),
    );

    assert_eq!(source.dependency_plugins.len(), 1);
    assert_eq!(source.dependency_plugins[0].name, "AesShared");
    assert_eq!(source.dependency_plugins[0].revision, "dep-rev");
}
