//! Source selection result shared by build and package planning.
//!
//! A source only decides where inputs and revisions come from. It must not
//! change the shape of the generated build/package plan.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceContext {
    pub source_ref: SourceRef,
    pub workspace: String,
    pub project: PathBuf,
    pub engine: PathBuf,
    pub plugins_root: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<PathBuf>,
    pub primary_plugins: Vec<SourcePlugin>,
    pub dependency_plugins: Vec<SourcePlugin>,
    pub source_revision: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSourceInput {
    pub workspace: String,
    pub project: PathBuf,
    pub engine: PathBuf,
    pub plugins_root: PathBuf,
    pub primary_plugins: Vec<SourcePlugin>,
    pub dependency_plugins: Vec<SourcePlugin>,
    pub source_revision: String,
}

#[allow(dead_code)]
impl WorkspaceSourceInput {
    pub fn new(
        workspace: impl Into<String>,
        project: PathBuf,
        engine: PathBuf,
        plugins_root: PathBuf,
        source_revision: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            project,
            engine,
            plugins_root,
            primary_plugins: Vec::new(),
            dependency_plugins: Vec::new(),
            source_revision: source_revision.into(),
        }
    }

    pub fn with_primary_plugins(mut self, primary_plugins: Vec<SourcePlugin>) -> Self {
        self.primary_plugins = primary_plugins;
        self
    }

    pub fn with_dependency_plugins(mut self, dependency_plugins: Vec<SourcePlugin>) -> Self {
        self.dependency_plugins = dependency_plugins;
        self
    }
}

#[allow(dead_code)]
pub fn resolve_workspace_source(input: WorkspaceSourceInput) -> SourceContext {
    SourceContext::from_workspace(SourceEnvironment {
        workspace: input.workspace,
        project: input.project,
        engine: input.engine,
        plugins_root: input.plugins_root,
        primary_plugins: input.primary_plugins,
        dependency_plugins: input.dependency_plugins,
        source_revision: input.source_revision,
    })
}

impl SourceContext {
    pub fn from_task(input: TaskSourceContext) -> Self {
        Self {
            source_ref: SourceRef::task(input.task_ref),
            workspace: input.environment.workspace,
            project: input.environment.project,
            engine: input.environment.engine,
            plugins_root: input.environment.plugins_root,
            host: Some(input.host),
            primary_plugins: input.environment.primary_plugins,
            dependency_plugins: input.environment.dependency_plugins,
            source_revision: input.environment.source_revision,
        }
    }

    pub fn from_workspace(environment: SourceEnvironment) -> Self {
        let workspace = environment.workspace;
        Self {
            source_ref: SourceRef::workspace(workspace.clone()),
            workspace,
            project: environment.project,
            engine: environment.engine,
            plugins_root: environment.plugins_root,
            host: None,
            primary_plugins: environment.primary_plugins,
            dependency_plugins: environment.dependency_plugins,
            source_revision: environment.source_revision,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEnvironment {
    pub workspace: String,
    pub project: PathBuf,
    pub engine: PathBuf,
    pub plugins_root: PathBuf,
    pub primary_plugins: Vec<SourcePlugin>,
    pub dependency_plugins: Vec<SourcePlugin>,
    pub source_revision: String,
}

impl SourceEnvironment {
    pub fn new(
        workspace: impl Into<String>,
        project: PathBuf,
        engine: PathBuf,
        plugins_root: PathBuf,
        source_revision: impl Into<String>,
    ) -> Self {
        Self {
            workspace: workspace.into(),
            project,
            engine,
            plugins_root,
            primary_plugins: Vec::new(),
            dependency_plugins: Vec::new(),
            source_revision: source_revision.into(),
        }
    }

    pub fn with_primary_plugins(mut self, primary_plugins: Vec<SourcePlugin>) -> Self {
        self.primary_plugins = primary_plugins;
        self
    }

    pub fn with_dependency_plugins(mut self, dependency_plugins: Vec<SourcePlugin>) -> Self {
        self.dependency_plugins = dependency_plugins;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSourceContext {
    pub environment: SourceEnvironment,
    pub task_ref: String,
    pub host: PathBuf,
}

impl TaskSourceContext {
    pub fn new(environment: SourceEnvironment, task_ref: impl Into<String>, host: PathBuf) -> Self {
        Self {
            environment,
            task_ref: task_ref.into(),
            host,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub kind: SourceKind,
    pub value: String,
}

impl SourceRef {
    pub fn task(task_ref: impl Into<String>) -> Self {
        Self {
            kind: SourceKind::Task,
            value: task_ref.into(),
        }
    }

    pub fn workspace(workspace: impl Into<String>) -> Self {
        Self {
            kind: SourceKind::Workspace,
            value: workspace.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Task,
    Workspace,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePlugin {
    pub name: String,
    pub path: PathBuf,
    pub revision: String,
    pub role: SourcePluginRole,
}

impl SourcePlugin {
    pub fn primary(name: impl Into<String>, path: PathBuf, revision: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path,
            revision: revision.into(),
            role: SourcePluginRole::Primary,
        }
    }

    pub fn dependency(name: impl Into<String>, path: PathBuf, revision: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path,
            revision: revision.into(),
            role: SourcePluginRole::Dependency,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourcePluginRole {
    Primary,
    Dependency,
}
