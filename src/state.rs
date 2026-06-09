//! Global state management for UnrealDevFlow

use crate::error::{Result, UdfError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalState {
    pub version: u32,
    pub projects: HashMap<String, ProjectState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JunctionState {
    pub plugin_name: String,
    pub junction_path: PathBuf,
    pub junction_target: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub path: PathBuf,
    pub active_task: Option<String>,
    /// v1 legacy single-junction path. Kept for backward compatibility; mirror
    /// of the first entry in `junctions` when populated.
    pub junction_path: PathBuf,
    /// v1 legacy single-junction target.
    pub junction_target: Option<PathBuf>,
    pub last_switch: Option<DateTime<Utc>>,
    pub previous_task: Option<String>,
    /// v2: all junctions managed for this project under the active task.
    #[serde(default)]
    pub junctions: Vec<JunctionState>,
}

impl GlobalState {
    pub fn state_path() -> Result<PathBuf> {
        let config_dir = crate::config::Config::config_dir()?;
        Ok(config_dir.join("state.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::state_path()?;
        if !path.exists() {
            return Ok(Self {
                version: 1,
                projects: HashMap::new(),
            });
        }
        let content = fs::read_to_string(&path)?;
        let state: GlobalState = serde_json::from_str(&content)?;
        Ok(state)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::state_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn get_project(&self, project_name: &str) -> Option<&ProjectState> {
        self.projects.get(project_name)
    }

    pub fn get_project_mut(&mut self, project_name: &str) -> Option<&mut ProjectState> {
        self.projects.get_mut(project_name)
    }

    pub fn set_project(&mut self, project_name: String, state: ProjectState) {
        self.projects.insert(project_name, state);
    }

    pub fn update_active_task(
        &mut self,
        project_name: &str,
        task_id: &str,
        junction_target: PathBuf,
    ) -> Result<()> {
        let project = self
            .projects
            .get_mut(project_name)
            .ok_or_else(|| UdfError::Other(format!("Project not found: {}", project_name)))?;

        project.previous_task = project.active_task.clone();
        project.active_task = Some(task_id.to_string());
        project.junction_target = Some(junction_target);
        project.last_switch = Some(Utc::now());

        self.save()
    }
}

/// Compute the canonical junction path for a plugin inside a UE project.
pub fn junction_path_for(project_path: &Path, plugin_name: &str) -> PathBuf {
    project_path.join("Plugins").join(plugin_name)
}
