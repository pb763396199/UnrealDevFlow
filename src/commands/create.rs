//! Create command implementation (v2 multi-plugin support)

use crate::cli::{DepOverride, DepOverrideKind};
use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host::{self, DependencyPlugin, DependencySource, PrimaryPlugin, TaskMeta};
use crate::output;
use crate::plugin::{DiscoveredPlugin, PluginSource, scanner, uplugin};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn run(
    description: &str,
    custom_id: Option<String>,
    prompt: Option<String>,
    primary: Option<Vec<String>>,
    overrides: Vec<DepOverride>,
    skip_confirm: bool,
) -> Result<()> {
    let config = Config::load()?;

    let task_id = match custom_id {
        Some(id) => id,
        None => suggest_task_id(description),
    };

    let host_dir = config.hosts_root.join(format!("T-{}_Host", task_id));
    if host_dir.exists() {
        return Err(UdfError::TaskAlreadyExists(task_id));
    }

    // === Resolve primary plugins ===
    let plugins_root = config.effective_plugins_root().ok_or_else(|| {
        UdfError::Other(
            "未配置 plugins_root（或 v1 plugin_path），请先运行 unrealdevflow configure"
                .to_string(),
        )
    })?;
    let project_plugins = scanner::enumerate_plugins(&plugins_root);

    let primary_names = resolve_primary_names(primary, &config)?;
    validate_primary_names(&primary_names, &project_plugins)?;

    // === Discover dependencies via .uplugin parsing ===
    let engine_plugins =
        scanner::enumerate_plugins(&scanner::engine_plugins_root(&config.engine_path));
    let combined_overrides =
        combined_overrides(&config, &overrides, &project_plugins, &engine_plugins)?;

    let mut all_dep_names: Vec<String> = Vec::new();
    let mut seen_deps: HashSet<String> = HashSet::new();
    for primary_name in &primary_names {
        let primary_dir = project_plugins
            .get(primary_name)
            .ok_or_else(|| UdfError::Other(format!("主插件目录不存在：{}", primary_name)))?;
        let deps = uplugin::read_dependencies(primary_dir)?;
        for d in deps {
            if !seen_deps.contains(&d) && !primary_names.contains(&d) {
                seen_deps.insert(d.clone());
                all_dep_names.push(d);
            }
        }
    }

    let resolution = scanner::resolve_dependencies(
        &all_dep_names,
        &engine_plugins,
        &project_plugins,
        &combined_overrides,
    )?;

    if !resolution.conflict.is_empty() {
        output::print_warning(
            "以下依赖同时存在于引擎和项目，请使用 --override-dep <name>=engine|project 指定：",
        );
        for (engine, project) in &resolution.conflict {
            output::print_warning(&format!(
                "  - {}: engine={:?} / project={:?}",
                engine.name, engine.path, project.path
            ));
        }
        return Err(UdfError::Other(
            "存在引擎/项目同名插件冲突，请用 --override-dep 解决后重试".to_string(),
        ));
    }

    // === Preview ===
    print_resolution_preview(&primary_names, &resolution);

    if !skip_confirm {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!("Create task '{}'?", task_id))
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
        if !confirmed {
            output::print_info("Task creation cancelled.");
            return Ok(());
        }
    }

    // === Create Host directory with enabled plugin list ===
    let engine_version = config
        .engine_path
        .file_name()
        .map(|n| n.to_string_lossy().replace("UE_", ""))
        .unwrap_or_else(|| "5.5".to_string());

    let project_dep_names: Vec<String> =
        resolution.project.iter().map(|p| p.name.clone()).collect();
    host::create_host_with_plugins(
        &host_dir,
        &task_id,
        &engine_version,
        &primary_names,
        &project_dep_names,
    )?;

    // === Create one worktree per primary plugin ===
    let mut primary_meta: Vec<PrimaryPlugin> = Vec::new();
    let branch_name = format!("task-{}", task_id);
    for name in &primary_names {
        let source_repo = project_plugins.get(name).cloned().unwrap();
        let repo = git::open_repo(&source_repo)?;
        let commit = git::get_current_commit(&repo)?;
        output::print_info(&format!(
            "Creating worktree for primary plugin '{}' from {} ...",
            name,
            &commit[..8.min(commit.len())]
        ));
        let worktree_rel = PathBuf::from("Plugins").join(name);
        let worktree_abs = host_dir.join(&worktree_rel);
        git::worktree::add(&source_repo, &worktree_abs, &commit, &branch_name)?;
        primary_meta.push(PrimaryPlugin {
            name: name.clone(),
            source_repo,
            worktree: worktree_rel,
            branch: branch_name.clone(),
            based_on: commit,
        });
    }

    // === Create dependency junctions for project deps ===
    let mut dep_meta: Vec<DependencyPlugin> = Vec::new();
    for dep in &resolution.project {
        let source_path = dep.path.clone().unwrap();
        let junction_rel = PathBuf::from("Plugins").join(&dep.name);
        let junction_abs = host_dir.join(&junction_rel);
        if let Some(parent) = junction_abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::junction::create(&source_path, &junction_abs)?;
        output::print_info(&format!(
            "Created dependency junction '{}' -> {:?}",
            dep.name, source_path
        ));
        dep_meta.push(DependencyPlugin {
            name: dep.name.clone(),
            source: DependencySource::Project,
            source_path,
            junction: Some(junction_rel),
        });
    }
    for dep in &resolution.engine {
        dep_meta.push(DependencyPlugin {
            name: dep.name.clone(),
            source: DependencySource::Engine,
            source_path: dep.path.clone().unwrap_or_default(),
            junction: None,
        });
    }

    // === Persist meta ===
    let first_primary = primary_meta
        .first()
        .ok_or_else(|| UdfError::Other("没有主插件被创建".to_string()))?;
    let meta = TaskMeta {
        schema_version: host::CURRENT_SCHEMA_VERSION,
        id: task_id.clone(),
        name: description.to_string(),
        branch: first_primary.branch.clone(),
        created: Utc::now().to_rfc3339(),
        based_on: first_primary.based_on.clone(),
        status: "active".to_string(),
        prompt,
        last_built: None,
        build_pid: None,
        build_log: None,
        console_log: None,
        build_status: None,
        primary_plugins: primary_meta,
        dependency_plugins: dep_meta,
    };
    host::write_meta(&host_dir, &meta)?;

    output::print_success(&format!("Task '{}' created successfully!", task_id));
    output::print_info(&format!("  Host: {:?}", host_dir));
    output::print_info(&format!("  Primary plugins: {}", primary_names.join(", ")));
    let project_deps_summary: Vec<String> =
        resolution.project.iter().map(|p| p.name.clone()).collect();
    if !project_deps_summary.is_empty() {
        output::print_info(&format!(
            "  Project dependencies (Junction): {}",
            project_deps_summary.join(", ")
        ));
    }
    let engine_deps_summary: Vec<String> =
        resolution.engine.iter().map(|p| p.name.clone()).collect();
    if !engine_deps_summary.is_empty() {
        output::print_info(&format!(
            "  Engine dependencies (auto-enabled): {}",
            engine_deps_summary.join(", ")
        ));
    }
    if !resolution.missing.is_empty() {
        output::print_warning(&format!(
            "  Missing dependencies (not found anywhere): {}",
            resolution.missing.join(", ")
        ));
        output::print_warning(
            "    Use --override-dep <name>=<absolute-path> to provide a custom location.",
        );
    }
    output::print_info(&format!("  Build:  unrealdevflow build {}", task_id));
    output::print_info(&format!("  Switch: unrealdevflow switch {}", task_id));
    Ok(())
}

fn resolve_primary_names(primary: Option<Vec<String>>, config: &Config) -> Result<Vec<String>> {
    let mut names: Vec<String> = match primary {
        Some(list) => list
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => Vec::new(),
    };
    if names.is_empty() {
        if let Some(legacy) = config.legacy_primary_plugin() {
            output::print_info(&format!(
                "未指定 --primary，使用 v1 兼容默认主插件：{}",
                legacy
            ));
            names.push(legacy);
        }
    }
    if names.is_empty() {
        return Err(UdfError::Other(
            "未指定主插件。请加 --primary <Name1,Name2> 或在 config.toml 配置 plugin_path 作为默认值".to_string(),
        ));
    }
    // dedup preserving order
    let mut seen = HashSet::new();
    names.retain(|n| seen.insert(n.clone()));
    Ok(names)
}

fn validate_primary_names(
    names: &[String],
    project_plugins: &HashMap<String, PathBuf>,
) -> Result<()> {
    let mut missing = Vec::new();
    for n in names {
        let dir_ok = project_plugins.contains_key(n);
        if !dir_ok {
            missing.push(n.clone());
        }
    }
    if !missing.is_empty() {
        return Err(UdfError::Other(format!(
            "以下主插件在项目 plugins_root 下找不到：{}",
            missing.join(", ")
        )));
    }
    Ok(())
}

fn combined_overrides(
    config: &Config,
    cli_overrides: &[DepOverride],
    project_plugins: &HashMap<String, PathBuf>,
    engine_plugins: &HashMap<String, PathBuf>,
) -> Result<HashMap<String, PathBuf>> {
    let mut out: HashMap<String, PathBuf> = config.plugin_overrides.clone();
    for ov in cli_overrides {
        let path = match &ov.kind {
            DepOverrideKind::CustomPath(p) => p.clone(),
            DepOverrideKind::Project => {
                project_plugins.get(&ov.name).cloned().ok_or_else(|| {
                    UdfError::Other(format!(
                        "--override-dep {}=project 失败：项目中未找到该插件",
                        ov.name
                    ))
                })?
            }
            DepOverrideKind::Engine => engine_plugins.get(&ov.name).cloned().ok_or_else(|| {
                UdfError::Other(format!(
                    "--override-dep {}=engine 失败：引擎中未找到该插件",
                    ov.name
                ))
            })?,
        };
        out.insert(ov.name.clone(), path);
    }
    Ok(out)
}

fn print_resolution_preview(
    primary_names: &[String],
    resolution: &crate::plugin::DependencyResolution,
) {
    println!();
    output::print_info("Task plan preview");
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!(
        "  Primary plugins ({}): {}",
        primary_names.len(),
        primary_names.join(", ")
    ));
    output::print_info(&format!(
        "  Project dependencies (Junction, {}):",
        resolution.project.len()
    ));
    for dep in &resolution.project {
        output::print_info(&format!(
            "    - {} -> {:?}",
            dep.name,
            dep.path.as_ref().unwrap_or(&PathBuf::new())
        ));
    }
    output::print_info(&format!(
        "  Engine dependencies (auto-enabled, {}):",
        resolution.engine.len()
    ));
    for dep in &resolution.engine {
        output::print_info(&format!("    - {}", dep.name));
    }
    if !resolution.missing.is_empty() {
        output::print_warning(&format!(
            "  Missing dependencies ({}):",
            resolution.missing.len()
        ));
        for name in &resolution.missing {
            output::print_warning(&format!("    - {}", name));
        }
    }
    println!("─────────────────────────────────────────────────────────────");
}

fn suggest_task_id(description: &str) -> String {
    let words: Vec<&str> = description.split_whitespace().take(3).collect();
    let id: String = words
        .iter()
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join("-");
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

// Allow unused helper to keep API surface, in case callers need a Discovered → Path map.
#[allow(dead_code)]
fn discovered_paths(items: &[DiscoveredPlugin]) -> Vec<(String, PluginSource, PathBuf)> {
    items
        .iter()
        .map(|d| (d.name.clone(), d.source, d.path.clone().unwrap_or_default()))
        .collect()
}

#[allow(dead_code)]
fn ensure_parent(p: &Path) -> Result<()> {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
