//! Cleanup of project-side Junctions owned by a task.
//!
//! A task Host contains the writable worktrees, but `task switch` installs
//! Junctions in one or more real UE projects. Those project entries must be
//! removed before the Host is deleted. In particular, `Path::exists()` is not
//! suitable here: on Windows it follows a Junction and returns false after
//! the target has already gone away.

use crate::config::Config;
use crate::error::Result;
use crate::host::{self, TaskMeta};
use crate::state::GlobalState;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Copy)]
pub struct CleanupReport {
    pub matched: usize,
    pub removed: usize,
    pub failed: usize,
}

impl CleanupReport {
    pub fn clean(self) -> bool {
        self.failed == 0
    }
}

/// Remove every project-side Junction whose target is inside `host_dir`.
///
/// The state ledger is the fast/precise source, while the direct project scan
/// is a repair path for stale or incomplete state. Both paths use filesystem
/// metadata, so dangling Junctions are included.
pub fn cleanup_for_task(
    config: &Config,
    task_ref: &str,
    host_dir: &Path,
    meta: &TaskMeta,
) -> Result<CleanupReport> {
    let task_uid = meta.task_uid.as_deref();
    let roots = [host_dir.to_path_buf()];
    let host_junctions = meta
        .dependency_plugins
        .iter()
        .filter_map(|dependency| dependency.junction.as_ref())
        .map(|relative| host_dir.join(relative))
        .collect();
    let task_project = meta
        .context
        .as_ref()
        .map(|context| context.default_project.clone());
    cleanup_for_host_roots(
        config,
        task_ref,
        &roots,
        task_uid,
        task_project.as_deref(),
        host_junctions,
    )
}

/// Recovery variant used when the Host itself is already missing.
pub fn cleanup_for_missing_task(config: &Config, task_ref: &str) -> Result<CleanupReport> {
    let roots = candidate_task_hosts(config, task_ref);
    cleanup_for_host_roots(config, task_ref, &roots, None, None, Vec::new())
}

fn candidate_task_hosts(config: &Config, task_ref: &str) -> Vec<PathBuf> {
    let (workspace, task_id) = host::parse_task_ref(task_ref);
    let mut roots = BTreeSet::new();

    if let Some(workspace) = workspace {
        if let Ok((name, workspace_config)) = config.resolve_workspace(Some(&workspace)) {
            roots.insert(host::task_host_dir(
                &workspace_config.hosts_root,
                Some(&name),
                &task_id,
            ));
        }
    } else {
        roots.insert(host::task_host_dir(&config.hosts_root, None, &task_id));
        for (name, workspace_config) in &config.workspaces {
            roots.insert(host::task_host_dir(
                &workspace_config.hosts_root,
                Some(name),
                &task_id,
            ));
        }
    }

    roots.into_iter().collect()
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn is_inside(path: &Path, root: &Path) -> bool {
    let path = normalize(path);
    let root = normalize(root);
    path == root || path.starts_with(&(root + "\\"))
}

fn task_target(target: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| is_inside(target, root))
}

fn add_candidate(candidates: &mut Vec<PathBuf>, seen: &mut BTreeSet<String>, path: PathBuf) {
    let key = normalize(&path);
    if seen.insert(key) {
        candidates.push(path);
    }
}

fn add_state_candidates(
    candidates: &mut Vec<PathBuf>,
    seen: &mut BTreeSet<String>,
    project_state: &crate::state::ProjectState,
    roots: &[PathBuf],
) {
    for junction in &project_state.junctions {
        if task_target(&junction.junction_target, roots) {
            add_candidate(candidates, seen, junction.junction_path.clone());
        }
    }
    // v1 state only had one project-side Junction.
    if !project_state.junction_path.as_os_str().is_empty()
        && project_state
            .junction_target
            .as_ref()
            .is_some_and(|target| task_target(target, roots))
    {
        add_candidate(candidates, seen, project_state.junction_path.clone());
    }
}

fn add_project_path(projects: &mut Vec<PathBuf>, path: PathBuf) {
    if !projects
        .iter()
        .any(|existing| normalize(existing) == normalize(&path))
    {
        projects.push(path);
    }
}

fn configured_project_paths(
    config: &Config,
    state: &GlobalState,
    task_project: Option<&Path>,
) -> Vec<PathBuf> {
    let mut projects = Vec::new();
    if let Some(task_project) = task_project {
        add_project_path(&mut projects, task_project.to_path_buf());
    }
    add_project_path(&mut projects, config.default_project.clone());
    for workspace in config.workspaces.values() {
        add_project_path(&mut projects, workspace.default_project.clone());
    }
    for project_state in state.projects.values() {
        add_project_path(&mut projects, project_state.path.clone());
    }
    projects
}

fn collect_project_candidates(
    config: &Config,
    state: &GlobalState,
    roots: &[PathBuf],
    task_project: Option<&Path>,
) -> (Vec<PathBuf>, Vec<(PathBuf, PathBuf)>) {
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();

    for project_state in state.projects.values() {
        add_state_candidates(&mut candidates, &mut seen, project_state, roots);
        for junction in &project_state.junctions {
            if task_target(&junction.junction_target, roots) {
                records.push((
                    junction.junction_path.clone(),
                    junction.junction_target.clone(),
                ));
            }
        }
        if let Some(target) = &project_state.junction_target
            && task_target(target, roots)
        {
            records.push((project_state.junction_path.clone(), target.clone()));
        }
    }

    // State can be stale or absent. Include every configured project so a
    // Junction left behind before the first state save is still recoverable.
    // Only inspect immediate Plugins children; never recurse into arbitrary
    // project content.
    for project_path in configured_project_paths(config, state, task_project) {
        let plugins_dir = project_path.join("Plugins");
        let Ok(entries) = std::fs::read_dir(plugins_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !crate::junction::exists(&path).unwrap_or(false) {
                continue;
            }
            if let Ok(target) = crate::junction::get_target(&path)
                && task_target(&target, roots)
            {
                add_candidate(&mut candidates, &mut seen, path.clone());
                records.push((path, target));
            }
        }
    }

    (candidates, records)
}

fn task_id(task_ref: &str) -> &str {
    task_ref.rsplit('/').next().unwrap_or(task_ref)
}

fn active_task_matches(active_task: Option<&str>, task_ref: &str, task_uid: Option<&str>) -> bool {
    let Some(active_task) = active_task else {
        return false;
    };
    active_task == task_ref
        || active_task == task_id(task_ref)
        || task_uid.is_some_and(|uid| active_task == uid)
}

fn cleanup_for_host_roots(
    config: &Config,
    task_ref: &str,
    roots: &[PathBuf],
    task_uid: Option<&str>,
    task_project: Option<&Path>,
    extra_candidates: Vec<PathBuf>,
) -> Result<CleanupReport> {
    let mut state = GlobalState::load()?;
    let (mut candidates, records) = collect_project_candidates(config, &state, roots, task_project);
    let mut seen = candidates.iter().map(|path| normalize(path)).collect();
    for path in extra_candidates {
        add_candidate(&mut candidates, &mut seen, path);
    }
    let record_targets: BTreeSet<String> = records
        .iter()
        .map(|(_, target)| normalize(target))
        .collect();
    let mut report = CleanupReport::default();
    let mut failed_paths = BTreeSet::new();

    for path in candidates {
        let key = normalize(&path);
        match crate::junction::exists(&path) {
            Ok(true) => {
                report.matched += 1;
                match crate::junction::delete(&path) {
                    Ok(()) => report.removed += 1,
                    Err(error) => {
                        report.failed += 1;
                        failed_paths.insert(key);
                        crate::output::print_warning(&format!(
                            "Failed to remove task Junction {:?}: {}",
                            path, error
                        ));
                    }
                }
            }
            Ok(false) => {}
            Err(error) => {
                report.failed += 1;
                failed_paths.insert(key);
                crate::output::print_warning(&format!(
                    "Failed to inspect task Junction {:?}: {}",
                    path, error
                ));
            }
        }
    }

    let mut state_changed = false;
    for project_state in state.projects.values_mut() {
        let before = project_state.junctions.len();
        project_state.junctions.retain(|junction| {
            let owned = record_targets.contains(&normalize(&junction.junction_target));
            !owned || failed_paths.contains(&normalize(&junction.junction_path))
        });
        if before != project_state.junctions.len() {
            state_changed = true;
        }

        let legacy_owned = project_state
            .junction_target
            .as_ref()
            .is_some_and(|target| task_target(target, roots));
        if legacy_owned {
            let legacy_failed = failed_paths.contains(&normalize(&project_state.junction_path));
            if !legacy_failed {
                project_state.junction_path = PathBuf::new();
                project_state.junction_target = None;
                state_changed = true;
            }
        }

        if report.failed == 0
            && active_task_matches(project_state.active_task.as_deref(), task_ref, task_uid)
        {
            project_state.active_task = None;
            state_changed = true;
        }

        let first = project_state.junctions.first().cloned();
        let mirrored_path = first.as_ref().map(|entry| entry.junction_path.clone());
        let mirrored_target = first.map(|entry| entry.junction_target);
        if project_state.junction_path != mirrored_path.clone().unwrap_or_default()
            || project_state.junction_target != mirrored_target
        {
            project_state.junction_path = mirrored_path.unwrap_or_default();
            project_state.junction_target = mirrored_target;
            state_changed = true;
        }
    }

    if state_changed {
        state.save()?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkspaceConfig;
    use crate::host::{DependencyPlugin, DependencySource, PrimaryPlugin, TaskContext};
    use crate::state::{GlobalState, JunctionState, ProjectState};
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn config_for(root: &Path) -> Config {
        Config {
            hosts_root: root.join("Hosts"),
            plugin_path: None,
            default_project: root.join("Project"),
            engine_path: root.join("Engine"),
            plugins_root: Some(root.join("Plugins")),
            plugin_overrides: HashMap::new(),
            workspaces: HashMap::from([(
                "test".to_string(),
                WorkspaceConfig {
                    hosts_root: root.join("Hosts"),
                    plugin_path: None,
                    default_project: root.join("Project"),
                    engine_path: root.join("Engine"),
                    plugins_root: Some(root.join("Plugins")),
                    plugin_overrides: HashMap::new(),
                },
            )]),
            last_used_workspace: None,
        }
    }

    fn meta() -> TaskMeta {
        TaskMeta {
            schema_version: 3,
            id: "dangling".to_string(),
            name: "dangling".to_string(),
            branch: "task-dangling".to_string(),
            created: "2026-09-02T00:00:00Z".to_string(),
            based_on: "base".to_string(),
            status: "created".to_string(),
            prompt: None,
            last_built: None,
            build_pid: None,
            build_log: None,
            console_log: None,
            build_status: None,
            primary_plugins: vec![PrimaryPlugin {
                name: "AesWorld".to_string(),
                source_repo: PathBuf::new(),
                worktree: PathBuf::from("Plugins/AesWorld"),
                branch: "task-dangling".to_string(),
                based_on: "base".to_string(),
            }],
            dependency_plugins: vec![DependencyPlugin {
                name: "Dep".to_string(),
                source: DependencySource::Project,
                source_path: PathBuf::new(),
                junction: Some(PathBuf::from("Plugins/Dep")),
            }],
            workspace: Some("test".to_string()),
            task_uid: Some("test/dangling".to_string()),
            context: None,
        }
    }

    #[test]
    #[cfg(windows)]
    fn cleanup_removes_dangling_project_junction_and_preserves_state_target() {
        let root = tempdir().expect("temp dir");
        let project = root.path().join("Project");
        let host = root.path().join("Hosts/W-test/T-dangling_Host");
        let project_link = project.join("Plugins/AesWorld");
        let target = host.join("Plugins/AesWorld");
        let dependency_target = root.path().join("Dependency");
        let dependency_link = host.join("Plugins/Dep");
        let unrelated_target = root.path().join("OtherTask");
        let unrelated_link = project.join("Plugins/Other");
        std::fs::create_dir_all(&target).expect("target");
        std::fs::create_dir_all(&dependency_target).expect("dependency target");
        std::fs::create_dir_all(&unrelated_target).expect("unrelated target");
        std::fs::create_dir_all(project.join("Plugins")).expect("project plugins");
        crate::junction::create(&dependency_target, &dependency_link).expect("dependency junction");
        crate::junction::create(&target, &project_link).expect("junction");
        crate::junction::create(&unrelated_target, &unrelated_link).expect("unrelated junction");
        std::fs::remove_dir_all(&target).expect("remove target");
        std::fs::create_dir_all(root.path().join("config")).expect("config");
        // Rust 2024 marks process-wide environment mutation unsafe. This test
        // is the only in-process caller and uses an isolated temporary state.
        unsafe {
            std::env::set_var("UNREALDEVFLOW_CONFIG_DIR", root.path().join("config"));
        }
        let config = config_for(root.path());
        config.save().expect("config");
        GlobalState {
            version: 2,
            projects: HashMap::from([(
                "test".to_string(),
                ProjectState {
                    path: project.clone(),
                    active_task: Some("test/dangling".to_string()),
                    junction_path: project_link.clone(),
                    junction_target: Some(target.clone()),
                    last_switch: None,
                    previous_task: None,
                    junctions: vec![
                        JunctionState {
                            plugin_name: "AesWorld".to_string(),
                            junction_path: project_link.clone(),
                            junction_target: target.clone(),
                        },
                        JunctionState {
                            plugin_name: "Other".to_string(),
                            junction_path: unrelated_link.clone(),
                            junction_target: unrelated_target.clone(),
                        },
                    ],
                },
            )]),
        }
        .save()
        .expect("state");
        let report = cleanup_for_task(&config, "test/dangling", &host, &meta()).expect("cleanup");
        assert_eq!(report.removed, 2);
        assert!(!crate::junction::exists(&project_link).unwrap_or(false));
        assert!(std::fs::symlink_metadata(&project_link).is_err());
        assert!(std::fs::symlink_metadata(&dependency_link).is_err());
        assert!(dependency_target.is_dir(), "dependency target was touched");
        assert!(crate::junction::exists(&unrelated_link).expect("unrelated junction"));
        assert!(unrelated_target.is_dir(), "unrelated target was touched");
    }

    #[test]
    #[cfg(windows)]
    fn cleanup_scans_configured_project_when_state_has_no_entry() {
        let root = tempdir().expect("temp dir");
        let project = root.path().join("Project");
        let host = root.path().join("Hosts/W-test/T-no-state_Host");
        let project_link = project.join("Plugins/AesWorld");
        let target = host.join("Plugins/AesWorld");
        std::fs::create_dir_all(&target).expect("target");
        std::fs::create_dir_all(project.join("Plugins")).expect("project plugins");
        crate::junction::create(&target, &project_link).expect("project junction");
        std::fs::remove_dir_all(&target).expect("remove Host before cleanup");
        assert!(
            !project_link.exists(),
            "Path::exists follows the broken junction"
        );
        assert!(std::fs::symlink_metadata(&project_link).is_ok());
        std::fs::create_dir_all(root.path().join("config")).expect("config");
        unsafe {
            std::env::set_var("UNREALDEVFLOW_CONFIG_DIR", root.path().join("config"));
        }
        let config = config_for(root.path());
        config.save().expect("config");

        let report = cleanup_for_task(&config, "test/no-state", &host, &meta()).expect("cleanup");

        assert_eq!(
            report.removed, 1,
            "configured project junction was not found"
        );
        assert!(std::fs::symlink_metadata(&project_link).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn cleanup_scans_task_context_project_when_workspace_path_changed() {
        let root = tempdir().expect("temp dir");
        let bound_project = root.path().join("BoundProject");
        let host = root.path().join("Hosts/W-test/T-context_Host");
        let project_link = bound_project.join("Plugins/AesWorld");
        let target = host.join("Plugins/AesWorld");
        std::fs::create_dir_all(&target).expect("target");
        std::fs::create_dir_all(bound_project.join("Plugins")).expect("bound project plugins");
        crate::junction::create(&target, &project_link).expect("project junction");
        std::fs::remove_dir_all(&target).expect("remove Host before cleanup");
        std::fs::create_dir_all(root.path().join("config")).expect("config");
        unsafe {
            std::env::set_var("UNREALDEVFLOW_CONFIG_DIR", root.path().join("config"));
        }
        let config = config_for(root.path());
        config.save().expect("config");
        let mut task_meta = meta();
        task_meta.context = Some(TaskContext {
            workspace: "test".to_string(),
            hosts_root: root.path().join("Hosts"),
            plugin_path: None,
            default_project: bound_project.clone(),
            engine_path: root.path().join("Engine"),
            plugins_root: Some(root.path().join("Plugins")),
            plugin_overrides: HashMap::new(),
        });

        let report = cleanup_for_task(&config, "test/context", &host, &task_meta).expect("cleanup");

        assert_eq!(
            report.removed, 1,
            "task context project junction was not found"
        );
        assert!(std::fs::symlink_metadata(&project_link).is_err());
    }
}
