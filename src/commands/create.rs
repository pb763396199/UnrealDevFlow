//! Create command implementation (v2 multi-plugin support)

use crate::cli::{DepOverride, DepOverrideKind};
use crate::config::{Config, WorkspaceConfig};
use crate::error::{GitError, Result, UdfError};
use crate::git;
use crate::host::{self, DependencyPlugin, DependencySource, PrimaryPlugin, TaskMeta};
use crate::output;
use crate::plugin::{DiscoveredPlugin, PluginSource, scanner, uplugin};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const CREATE_BASE_BRANCH: &str = "dev";

struct PrimaryPlan {
    name: String,
    source_repo: PathBuf,
    based_on: String,
    worktree_rel: PathBuf,
    worktree_abs: PathBuf,
}

struct PrimarySelection {
    names: Vec<String>,
}

#[derive(Default)]
struct DependencyOverrides {
    project: HashMap<String, PathBuf>,
    engine: HashSet<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    description: &str,
    custom_id: Option<String>,
    custom_branch: Option<String>,
    base_ref: Option<String>,
    prompt: Option<String>,
    workspace: Option<String>,
    primary: Option<Vec<String>>,
    overrides: Vec<DepOverride>,
    skip_confirm: bool,
) -> Result<()> {
    let config = Config::load()?;
    let (workspace_name, workspace_config) = config.resolve_workspace(workspace.as_deref())?;
    crate::commands::workspace::validate_workspace(&workspace_config)?;

    let task_id = match custom_id {
        Some(id) => id,
        None => suggest_task_id(description),
    };
    validate_task_id(&task_id)?;

    let host_dir = host::task_host_dir(
        &workspace_config.hosts_root,
        Some(&workspace_name),
        &task_id,
    );
    if host_dir.exists() {
        return Err(UdfError::TaskAlreadyExists(task_id));
    }

    // === Resolve primary plugins ===
    let plugins_root = workspace_config.effective_plugins_root().ok_or_else(|| {
        UdfError::Other(
            "未配置 plugins_root（或 v1 plugin_path），请先运行 unrealdevflow configure"
                .to_string(),
        )
    })?;
    let project_plugin_locations = scanner::enumerate_plugin_locations(&plugins_root);
    let primary_selection = resolve_primary_names(primary, &workspace_config)?;
    let primary_names = primary_selection.names;
    let mut preferred_primary_plugins = preferred_primary_plugins(&workspace_config)?;
    for override_value in &overrides {
        if primary_names.contains(&override_value.name)
            && let DepOverrideKind::CustomPath(path) = &override_value.kind
        {
            preferred_primary_plugins.insert(override_value.name.clone(), path.clone());
        }
    }
    let mut project_plugins = resolve_required_project_plugins(
        &primary_names,
        &project_plugin_locations,
        &preferred_primary_plugins,
        "主插件",
    )?;
    validate_primary_names(&primary_names, &project_plugins)?;
    validate_primary_sources_are_main_worktrees(&primary_names, &project_plugins)?;
    let branch_name =
        custom_branch.unwrap_or_else(|| format!("task/{}/{}", workspace_name, task_id));
    validate_branch_name(&branch_name)?;
    let primary_plans = prepare_primary_plans(
        &primary_names,
        &project_plugins,
        &host_dir,
        &branch_name,
        base_ref.as_deref(),
    )?;

    // === Discover dependencies via .uplugin parsing ===
    let engine_plugin_locations = scanner::enumerate_plugin_locations(
        &scanner::engine_plugins_root(&workspace_config.engine_path),
    );
    let combined_overrides = combined_overrides(
        &workspace_config,
        &overrides,
        &project_plugin_locations,
        &engine_plugin_locations,
    )?;

    let mut all_dep_names: Vec<String> = Vec::new();
    let mut seen_deps: HashSet<String> = HashSet::new();
    let mut primary_descriptor_names = Vec::new();
    for primary_name in &primary_names {
        let primary_dir = project_plugins
            .get(primary_name)
            .ok_or_else(|| UdfError::Other(format!("主插件目录不存在：{}", primary_name)))?;
        primary_descriptor_names.extend(uplugin::read_plugin_names(primary_dir)?);
    }
    primary_descriptor_names.sort();
    primary_descriptor_names.dedup();
    for primary_name in &primary_names {
        let primary_dir = project_plugins
            .get(primary_name)
            .ok_or_else(|| UdfError::Other(format!("主插件目录不存在：{}", primary_name)))?;
        let deps = uplugin::read_dependencies(primary_dir)?;
        for d in deps {
            if !seen_deps.contains(&d) && !primary_descriptor_names.contains(&d) {
                seen_deps.insert(d.clone());
                all_dep_names.push(d);
            }
        }
    }

    // Expand project-plugin dependencies transitively. UBT resolves engine
    // plugin internals itself, but every project dependency must be present in
    // the generated Host, including dependencies several descriptors deep.
    let mut dependency_index = 0;
    while dependency_index < all_dep_names.len() {
        let dependency_name = all_dep_names[dependency_index].clone();
        dependency_index += 1;

        let explicit_project = combined_overrides.project.get(&dependency_name).cloned();
        let project_path = match explicit_project {
            Some(path) => Some(path),
            None => unique_project_plugin_path(
                &dependency_name,
                &project_plugin_locations,
                &HashMap::new(),
                "递归依赖插件",
            )?,
        };
        let forced_engine = combined_overrides.engine.contains(&dependency_name);
        let engine_exists = engine_plugin_locations
            .get(&dependency_name)
            .map(|locations| !locations.is_empty())
            .unwrap_or(false);
        if forced_engine
            || (engine_exists && !combined_overrides.project.contains_key(&dependency_name))
        {
            continue;
        }
        let Some(project_path) = project_path else {
            continue;
        };
        for transitive in uplugin::read_dependencies(&project_path)? {
            if !primary_descriptor_names.contains(&transitive)
                && seen_deps.insert(transitive.clone())
            {
                all_dep_names.push(transitive);
            }
        }
    }

    add_relevant_dependency_plugins(
        &mut project_plugins,
        &all_dep_names,
        &project_plugin_locations,
        &combined_overrides.project,
    )?;
    let engine_plugins = resolve_relevant_engine_plugins(
        &all_dep_names,
        &engine_plugin_locations,
        &combined_overrides.engine,
    )?;

    let resolution = scanner::resolve_dependencies(
        &all_dep_names,
        &engine_plugins,
        &project_plugins,
        &combined_overrides.project,
        &combined_overrides.engine,
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
    if !resolution.missing.is_empty() {
        return Err(UdfError::Other(format!(
            "以下依赖无法解析，已拒绝创建任务：{}\n\
             请修正 plugins_root/engine_path，或使用 --override-dep <name>=<absolute-path> 显式提供。",
            resolution.missing.join(", ")
        )));
    }
    validate_project_dependency_sources(&workspace_config, &resolution.project)?;

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
    let engine_version = workspace_config
        .engine_path
        .file_name()
        .map(|n| n.to_string_lossy().replace("UE_", ""))
        .unwrap_or_else(|| "5.5".to_string());

    let project_dep_names: Vec<String> =
        resolution.project.iter().map(|p| p.name.clone()).collect();
    if let Err(err) = host::create_host_with_plugins(
        &host_dir,
        &task_id,
        &engine_version,
        &primary_descriptor_names,
        &project_dep_names,
    ) {
        cleanup_partial_create(&host_dir, &primary_plans, &branch_name);
        return Err(err);
    }

    // === Create one worktree per primary plugin ===
    let mut primary_meta: Vec<PrimaryPlugin> = Vec::new();
    let create_result = (|| -> Result<Vec<PrimaryPlugin>> {
        for plan in &primary_plans {
            output::print_info(&format!(
                "Creating worktree for primary plugin '{}' from {} ...",
                plan.name,
                &plan.based_on[..8.min(plan.based_on.len())]
            ));
            git::worktree::add(
                &plan.source_repo,
                &plan.worktree_abs,
                &plan.based_on,
                &branch_name,
            )?;
            git::worktree::update_submodules(&plan.worktree_abs)?;
            primary_meta.push(PrimaryPlugin {
                name: plan.name.clone(),
                source_repo: plan.source_repo.clone(),
                worktree: plan.worktree_rel.clone(),
                branch: branch_name.clone(),
                based_on: plan.based_on.clone(),
            });
        }
        Ok(primary_meta)
    })();
    let primary_meta = match create_result {
        Ok(meta) => meta,
        Err(err) => {
            cleanup_partial_create(&host_dir, &primary_plans, &branch_name);
            return Err(err);
        }
    };

    // === Create dependency junctions for project deps ===
    let mut dep_meta: Vec<DependencyPlugin> = Vec::new();
    let dep_result = (|| -> Result<Vec<DependencyPlugin>> {
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
        Ok(dep_meta)
    })();
    let dep_meta = match dep_result {
        Ok(meta) => meta,
        Err(err) => {
            cleanup_partial_create(&host_dir, &primary_plans, &branch_name);
            return Err(err);
        }
    };

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
        workspace: Some(workspace_name.clone()),
        task_uid: Some(format!("{}/{}", workspace_name, task_id)),
        context: Some(host::TaskContext::from_workspace(
            &workspace_name,
            &workspace_config,
        )),
    };
    if let Err(err) = host::write_meta(&host_dir, &meta) {
        cleanup_partial_create(&host_dir, &primary_plans, &branch_name);
        return Err(err);
    }

    output::print_success(&format!("Task '{}' created successfully!", task_id));
    output::print_info(&format!("  Workspace: {}", workspace_name));
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
    output::print_info(&format!(
        "  Build:  unrealdevflow build {}/{}",
        workspace_name, task_id
    ));
    output::print_info(&format!(
        "  Switch: unrealdevflow switch {}/{}",
        workspace_name, task_id
    ));
    Ok(())
}

fn resolve_primary_names(
    primary: Option<Vec<String>>,
    workspace: &WorkspaceConfig,
) -> Result<PrimarySelection> {
    let mut names: Vec<String> = match primary {
        Some(list) => list
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => Vec::new(),
    };
    if names.is_empty()
        && let Some(legacy) = workspace.legacy_primary_plugin()
    {
        output::print_info(&format!(
            "未指定 --primary，使用 v1 兼容默认主插件：{}",
            legacy
        ));
        names.push(legacy);
    }
    if names.is_empty() {
        return Err(UdfError::Other(
            "未指定主插件。请加 --primary <Name1,Name2> 或在 config.toml 配置 plugin_path 作为默认值".to_string(),
        ));
    }
    // dedup preserving order
    let mut seen = HashSet::new();
    names.retain(|n| seen.insert(n.clone()));
    Ok(PrimarySelection { names })
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

fn resolve_required_project_plugins(
    names: &[String],
    locations: &HashMap<String, Vec<PathBuf>>,
    preferred: &HashMap<String, PathBuf>,
    role: &str,
) -> Result<HashMap<String, PathBuf>> {
    let mut out = HashMap::new();
    let mut missing = Vec::new();
    for name in names {
        match unique_project_plugin_path(name, locations, preferred, role)? {
            Some(path) => {
                out.insert(name.clone(), path);
            }
            None => missing.push(name.clone()),
        }
    }
    if !missing.is_empty() {
        return Err(UdfError::Other(format!(
            "以下{}在项目 plugins_root 下找不到：{}",
            role,
            missing.join(", ")
        )));
    }
    Ok(out)
}

fn unique_project_plugin_path(
    name: &str,
    locations: &HashMap<String, Vec<PathBuf>>,
    preferred: &HashMap<String, PathBuf>,
    role: &str,
) -> Result<Option<PathBuf>> {
    if let Some(path) = preferred.get(name) {
        return Ok(Some(path.clone()));
    }
    let Some(paths) = locations.get(name) else {
        return Ok(None);
    };
    if paths.len() == 1 {
        return Ok(paths.first().cloned());
    }
    let paths = paths
        .iter()
        .map(|path| format!("  - {:?}", path))
        .collect::<Vec<_>>()
        .join("\n");
    Err(UdfError::Other(format!(
        "plugins_root 中发现重复插件 '{}'（{}）：\n{}\n\
         该插件参与当前任务，必须唯一。请把 plugins_root 收窄、改名旧副本，或对依赖使用 --override-dep 显式指定。",
        name, role, paths
    )))
}

fn preferred_primary_plugins(workspace: &WorkspaceConfig) -> Result<HashMap<String, PathBuf>> {
    let mut out = HashMap::new();
    let Some(plugin_path) = &workspace.plugin_path else {
        return Ok(out);
    };
    let plugin_name = uplugin::read_plugin_name(plugin_path)?;
    out.insert(plugin_name, plugin_path.clone());
    Ok(out)
}

fn add_relevant_dependency_plugins(
    project_plugins: &mut HashMap<String, PathBuf>,
    dependency_names: &[String],
    locations: &HashMap<String, Vec<PathBuf>>,
    overrides: &HashMap<String, PathBuf>,
) -> Result<()> {
    for name in dependency_names {
        if project_plugins.contains_key(name) || overrides.contains_key(name) {
            continue;
        }
        if let Some(path) =
            unique_project_plugin_path(name, locations, &HashMap::new(), "依赖插件")?
        {
            project_plugins.insert(name.clone(), path);
        }
    }
    Ok(())
}

fn resolve_relevant_engine_plugins(
    dependency_names: &[String],
    locations: &HashMap<String, Vec<PathBuf>>,
    engine_overrides: &HashSet<String>,
) -> Result<HashMap<String, PathBuf>> {
    let mut out = HashMap::new();
    for name in dependency_names {
        if let Some(path) = unique_engine_plugin_path(name, locations, "引擎依赖插件")? {
            out.insert(name.clone(), path);
        }
    }
    for name in engine_overrides {
        let path = unique_engine_plugin_path(name, locations, "override-dep engine")?.ok_or_else(
            || {
                UdfError::Other(format!(
                    "--override-dep {}=engine 失败：引擎中未找到该插件",
                    name
                ))
            },
        )?;
        out.insert(name.clone(), path);
    }
    Ok(out)
}

fn unique_engine_plugin_path(
    name: &str,
    locations: &HashMap<String, Vec<PathBuf>>,
    role: &str,
) -> Result<Option<PathBuf>> {
    let Some(paths) = locations.get(name) else {
        return Ok(None);
    };
    if paths.len() == 1 {
        return Ok(paths.first().cloned());
    }
    let paths = paths
        .iter()
        .map(|path| format!("  - {:?}", path))
        .collect::<Vec<_>>()
        .join("\n");
    Err(UdfError::Other(format!(
        "引擎 Plugins 中发现重复插件 '{}'（{}）：\n{}\n\
         该插件参与当前任务，必须唯一。请收窄/清理 Engine Plugins，或使用 --override-dep <name>=<absolute-path> 显式指定。",
        name, role, paths
    )))
}

fn validate_project_dependency_sources(
    workspace: &WorkspaceConfig,
    deps: &[DiscoveredPlugin],
) -> Result<()> {
    for dep in deps {
        let Some(path) = dep.path.as_ref() else {
            return Err(UdfError::Other(format!(
                "项目依赖插件 '{}' 缺少 source path",
                dep.name
            )));
        };
        crate::commands::workspace::validate_plugin_source_path(
            workspace,
            &format!("项目依赖插件 '{}'", dep.name),
            path,
            false,
        )?;
    }
    Ok(())
}

fn validate_task_id(task_id: &str) -> Result<()> {
    let mut chars = task_id.chars();
    let Some(first) = chars.next() else {
        return Err(UdfError::Other(
            "task id 不能为空。请使用 --id <kebab-case>".to_string(),
        ));
    };
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return Err(invalid_task_id(task_id));
    }
    if !chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-') {
        return Err(invalid_task_id(task_id));
    }
    Ok(())
}

fn invalid_task_id(task_id: &str) -> UdfError {
    UdfError::Other(format!(
        "task id 非法：'{}'。只允许小写英文、数字和连字符，且不能包含路径分隔符。示例：prefab-save-bug",
        task_id
    ))
}

fn validate_branch_name(branch_name: &str) -> Result<()> {
    let output = std::process::Command::new("git")
        .args(["check-ref-format", "--branch", branch_name])
        .output()?;
    if !output.status.success() {
        return Err(UdfError::Other(format!(
            "非法 Git 分支名：'{}'。请使用例如 feature/test 或 task/my-task。",
            branch_name
        )));
    }
    Ok(())
}

fn prepare_primary_plans(
    names: &[String],
    project_plugins: &HashMap<String, PathBuf>,
    host_dir: &Path,
    branch_name: &str,
    base_ref: Option<&str>,
) -> Result<Vec<PrimaryPlan>> {
    let mut plans = Vec::new();
    for name in names {
        let source_repo = project_plugins
            .get(name)
            .cloned()
            .ok_or_else(|| UdfError::Other(format!("主插件目录不存在：{}", name)))?;
        let source_repo = dunce::canonicalize(&source_repo).unwrap_or(source_repo);
        let repo = git::open_repo(&source_repo)?;
        if base_ref.is_none() {
            let current_branch = git::get_current_branch(&repo)?;
            if current_branch != CREATE_BASE_BRANCH {
                return Err(UdfError::Other(format!(
                    "主插件 '{}' 当前分支是 '{}'，不能创建任务。\n\
                 请先切回 '{}' 并确保它是最新基线。",
                    name, current_branch, CREATE_BASE_BRANCH
                )));
            }
            let status = git::status_porcelain(&source_repo)?;
            if !status.is_empty() {
                return Err(UdfError::Other(format!(
                    "主插件 '{}' 的主仓工作区不干净，不能创建任务：\n{}",
                    name, status
                )));
            }
        }
        let based_on = match base_ref {
            Some(reference) => git::resolve_commit(&source_repo, reference)?,
            None => git::get_current_commit(&repo)?,
        };
        if git::branch_exists(&source_repo, branch_name)? {
            return Err(UdfError::Other(format!(
                "任务分支已存在：{} ({:?})",
                branch_name, source_repo
            )));
        }

        let worktree_rel = PathBuf::from("Plugins").join(name);
        let worktree_abs = host_dir.join(&worktree_rel);
        if worktree_abs.exists() {
            return Err(UdfError::Other(format!(
                "任务 worktree 路径已存在：{:?}",
                worktree_abs
            )));
        }

        plans.push(PrimaryPlan {
            name: name.clone(),
            source_repo,
            based_on,
            worktree_rel,
            worktree_abs,
        });
    }
    Ok(plans)
}

fn validate_primary_sources_are_main_worktrees(
    names: &[String],
    project_plugins: &HashMap<String, PathBuf>,
) -> Result<()> {
    for name in names {
        let Some(source_repo) = project_plugins.get(name) else {
            continue;
        };
        match git::linked_worktree_main(source_repo) {
            Ok(Some(main_worktree)) => {
                let suggested_root = main_worktree
                    .parent()
                    .map(|path| path.to_path_buf())
                    .unwrap_or_else(|| main_worktree.clone());
                return Err(UdfError::Other(format!(
                    "主插件 '{}' 当前解析到 Git linked worktree：{:?}\n\
                     这会让新任务错误地基于另一个任务分支创建。\n\
                     主 checkout 是：{:?}\n\
                     请把 workspace/config 的 plugins_root 改为主插件仓库根目录，例如：{:?}",
                    name, source_repo, main_worktree, suggested_root
                )));
            }
            Ok(None) => {}
            Err(UdfError::Git(GitError::NotARepo(_))) => {
                return Err(UdfError::Other(format!(
                    "主插件 '{}' 不是 Git 仓库：{:?}",
                    name, source_repo
                )));
            }
            Err(UdfError::Git(GitError::CommandFailed(_))) => {
                return Err(UdfError::Other(format!(
                    "无法确认主插件 '{}' 的 Git 主 checkout：{:?}",
                    name, source_repo
                )));
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

/// Undo a `create` that failed part way through.
///
/// Works from the plans rather than from what was recorded as created: a
/// worktree that git made but that we never got to record must still go.
fn cleanup_partial_create(host_dir: &Path, primary_plans: &[PrimaryPlan], branch_name: &str) {
    let mut source_repositories = HashSet::new();
    for plan in primary_plans.iter().rev() {
        source_repositories.insert(plan.source_repo.clone());
        if plan.worktree_abs.exists()
            && let Err(err) = git::worktree::remove(&plan.source_repo, &plan.worktree_abs)
        {
            output::print_warning(&format!(
                "回滚 worktree 失败 {:?}：{}",
                plan.worktree_abs, err
            ));
        }
    }
    if host_dir.exists()
        && let Err(err) = host::delete_host(host_dir)
    {
        match quarantine_partial_host(host_dir) {
            Ok(path) => output::print_warning(&format!(
                "回滚 Host 目录时遇到错误：{err}；残留已隔离到 {:?}",
                path
            )),
            Err(quarantine_error) => output::print_warning(&format!(
                "回滚 Host 目录失败 {:?}：{err}；隔离也失败：{quarantine_error}",
                host_dir
            )),
        }
    }
    for repository in &source_repositories {
        if let Err(err) = git::worktree::prune(repository) {
            output::print_warning(&format!(
                "回滚 worktree 元数据失败 {:?}：{}",
                repository, err
            ));
        }
    }
    for plan in primary_plans {
        if git::branch_exists(&plan.source_repo, branch_name).unwrap_or(false)
            && let Err(err) = git::delete_branch_safe(&plan.source_repo, branch_name)
        {
            output::print_warning(&format!(
                "回滚分支失败 '{}' ({:?})：{}",
                branch_name, plan.source_repo, err
            ));
        }
    }
    if let Some(workspace_dir) = host_dir.parent() {
        let _ = std::fs::remove_dir(workspace_dir);
    }
}

/// Move a Host that refuses to delete out of the way instead of leaving it
/// where the next `create` would trip over it.
fn quarantine_partial_host(host_dir: &Path) -> std::io::Result<PathBuf> {
    let workspace_dir = host_dir.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Host 缺少 workspace 父目录",
        )
    })?;
    let hosts_root = workspace_dir.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Host 缺少 hosts root")
    })?;
    let quarantine_root = hosts_root.join(".udf-failed");
    std::fs::create_dir_all(&quarantine_root)?;
    let destination = quarantine_root.join(format!(
        "failed-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    std::fs::rename(host_dir, &destination)?;
    Ok(destination)
}

fn combined_overrides(
    workspace: &WorkspaceConfig,
    cli_overrides: &[DepOverride],
    project_plugin_locations: &HashMap<String, Vec<PathBuf>>,
    engine_plugin_locations: &HashMap<String, Vec<PathBuf>>,
) -> Result<DependencyOverrides> {
    let mut out = DependencyOverrides {
        project: workspace.plugin_overrides.clone(),
        engine: HashSet::new(),
    };
    for ov in cli_overrides {
        match &ov.kind {
            DepOverrideKind::CustomPath(p) => {
                out.project.insert(ov.name.clone(), p.clone());
                out.engine.remove(&ov.name);
            }
            DepOverrideKind::Project => {
                let path = unique_project_plugin_path(
                    &ov.name,
                    project_plugin_locations,
                    &HashMap::new(),
                    "override-dep project",
                )?
                .ok_or_else(|| {
                    UdfError::Other(format!(
                        "--override-dep {}=project 失败：项目中未找到该插件",
                        ov.name
                    ))
                })?;
                out.project.insert(ov.name.clone(), path);
                out.engine.remove(&ov.name);
            }
            DepOverrideKind::Engine => {
                if unique_engine_plugin_path(
                    &ov.name,
                    engine_plugin_locations,
                    "override-dep engine",
                )?
                .is_none()
                {
                    return Err(UdfError::Other(format!(
                        "--override-dep {}=engine 失败：引擎中未找到该插件",
                        ov.name
                    )));
                }
                out.engine.insert(ov.name.clone());
                out.project.remove(&ov.name);
            }
        }
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
    let id: String = id
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .collect();
    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        "task".to_string()
    } else {
        id
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_host_can_be_quarantined_without_recursive_deletion() {
        let root = tempfile::tempdir().unwrap();
        let host_dir = root.path().join("W-overlong-workspace").join("T-task_Host");
        std::fs::create_dir_all(host_dir.join("Plugins/AesWorld")).unwrap();
        std::fs::write(host_dir.join("Plugins/AesWorld/partial.txt"), "partial").unwrap();

        let quarantined = quarantine_partial_host(&host_dir).unwrap();

        assert!(!host_dir.exists());
        assert!(quarantined.starts_with(root.path().join(".udf-failed")));
        assert!(quarantined.join("Plugins/AesWorld/partial.txt").is_file());
    }
}
