//! Plugin discovery and classification.
//!
//! Scans engine and project plugin roots to locate `.uplugin` files and
//! classifies dependency requests into engine / project / conflict / missing.

use super::{DependencyResolution, DiscoveredPlugin, PluginSource};
use crate::error::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Recursively walk a plugins root to collect plugin-name → plugin-dir.
///
/// A plugin directory is one that contains a `.uplugin` file.
/// We do not descend into a directory that already has a `.uplugin`.
pub fn enumerate_plugins(root: &Path) -> HashMap<String, PathBuf> {
    let mut result = HashMap::new();
    if !root.is_dir() {
        return result;
    }
    walk(root, &mut result, 0);
    result
}

fn walk(dir: &Path, out: &mut HashMap<String, PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut found_uplugin = false;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "uplugin").unwrap_or(false) {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                out.entry(stem.to_string()).or_insert_with(|| dir.to_path_buf());
                found_uplugin = true;
            }
        } else if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if name.starts_with('.')
                || name.eq_ignore_ascii_case("Binaries")
                || name.eq_ignore_ascii_case("Intermediate")
                || name.eq_ignore_ascii_case("Content")
                || name.eq_ignore_ascii_case("Source")
                || name.eq_ignore_ascii_case("Resources")
                || name.eq_ignore_ascii_case("Config")
                || name.eq_ignore_ascii_case("Shaders")
            {
                continue;
            }
            subdirs.push(path);
        }
    }
    if !found_uplugin {
        for sub in subdirs {
            walk(&sub, out, depth + 1);
        }
    }
}

/// Resolve a list of dependency names against engine and project plugin maps,
/// honoring user overrides.
///
/// Override semantics:
/// - If the override path exists, the plugin is classified as `Project` (or any
///   custom location) and engine duplicates are silently ignored.
/// - If override is `None`, classification rule is:
///     - present only in engine → Engine
///     - present only in project → Project
///     - present in both → Conflict (user must resolve)
///     - present in neither → Missing
pub fn resolve_dependencies(
    deps: &[String],
    engine_plugins: &HashMap<String, PathBuf>,
    project_plugins: &HashMap<String, PathBuf>,
    overrides: &HashMap<String, PathBuf>,
) -> Result<DependencyResolution> {
    let mut resolution = DependencyResolution::default();
    for name in deps {
        if let Some(override_path) = overrides.get(name) {
            resolution.project.push(DiscoveredPlugin {
                name: name.clone(),
                source: PluginSource::Project,
                path: Some(override_path.clone()),
            });
            continue;
        }
        let in_engine = engine_plugins.get(name).cloned();
        let in_project = project_plugins.get(name).cloned();
        match (in_engine, in_project) {
            (Some(_), Some(p)) => {
                let engine_path = engine_plugins.get(name).cloned();
                resolution.conflict.push((
                    DiscoveredPlugin {
                        name: name.clone(),
                        source: PluginSource::Engine,
                        path: engine_path,
                    },
                    DiscoveredPlugin {
                        name: name.clone(),
                        source: PluginSource::Project,
                        path: Some(p),
                    },
                ));
            }
            (Some(p), None) => resolution.engine.push(DiscoveredPlugin {
                name: name.clone(),
                source: PluginSource::Engine,
                path: Some(p),
            }),
            (None, Some(p)) => resolution.project.push(DiscoveredPlugin {
                name: name.clone(),
                source: PluginSource::Project,
                path: Some(p),
            }),
            (None, None) => resolution.missing.push(name.clone()),
        }
    }
    Ok(resolution)
}

/// Default engine-plugin root derived from a UE engine install.
pub fn engine_plugins_root(engine_path: &Path) -> PathBuf {
    engine_path.join("Engine").join("Plugins")
}
