//! Durable task-to-project routes used by cleanup recovery.

use crate::config::Config;
use crate::error::{Result, UdfError};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

const ROUTES_FILE_NAME: &str = "task-routes.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRoute {
    pub task_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_uid: Option<String>,
    pub host_dir: PathBuf,
    pub project_paths: Vec<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RouteLedger {
    version: u32,
    #[serde(default)]
    routes: Vec<TaskRoute>,
}

pub fn record(route: TaskRoute) -> Result<()> {
    let path = routes_path()?;
    let mut ledger = load(&path)?;
    ledger.routes.retain(|existing| {
        !same_task(
            existing.task_ref.as_str(),
            route.task_ref.as_str(),
            route.task_uid.as_deref(),
        )
    });
    ledger.routes.push(route);
    ledger.version = 1;
    save(&path, &ledger)
}

pub fn find(task_ref: &str) -> Result<Vec<TaskRoute>> {
    let path = routes_path()?;
    Ok(load(&path)?
        .routes
        .into_iter()
        .filter(|route| same_task(route.task_ref.as_str(), task_ref, None))
        .collect())
}

pub fn remove(task_ref: &str, task_uid: Option<&str>) -> Result<()> {
    let path = routes_path()?;
    if !path.exists() {
        return Ok(());
    }
    let mut ledger = load(&path)?;
    let before = ledger.routes.len();
    ledger
        .routes
        .retain(|route| !same_task(route.task_ref.as_str(), task_ref, task_uid));
    if before == ledger.routes.len() {
        return Ok(());
    }
    if ledger.routes.is_empty() {
        fs::remove_file(path)?;
        return Ok(());
    }
    save(&path, &ledger)
}

fn same_task(recorded_ref: &str, requested_ref: &str, task_uid: Option<&str>) -> bool {
    recorded_ref.eq_ignore_ascii_case(requested_ref)
        || recorded_ref
            .rsplit('/')
            .next()
            .is_some_and(|id| id.eq_ignore_ascii_case(requested_ref))
        || task_uid.is_some_and(|uid| recorded_ref.eq_ignore_ascii_case(uid))
}

fn routes_path() -> Result<PathBuf> {
    Ok(Config::config_dir()?.join(ROUTES_FILE_NAME))
}

fn load(path: &Path) -> Result<RouteLedger> {
    if !path.exists() {
        return Ok(RouteLedger {
            version: 1,
            routes: Vec::new(),
        });
    }
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|error| UdfError::Other(error.to_string()))
}

fn save(path: &Path, ledger: &RouteLedger) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_vec_pretty(ledger)?;
    let temp_path =
        path.with_file_name(format!(".{}.{}.tmp", ROUTES_FILE_NAME, std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = File::create(&temp_path)?;
        file.write_all(&content)?;
        file.sync_all()?;
        fs::rename(&temp_path, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result.map_err(Into::into)
}
