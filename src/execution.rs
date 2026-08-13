//! Shared execution contract for build and package commands.
//!
//! Commands should generate an immutable `ExecutionPlan` first. `check`,
//! human-readable `plan`, execution, and `status` can then consume the same
//! shape instead of each command inventing its own output contract.

use crate::source_context::SourceContext;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionDomain {
    Build,
    Package,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionAction {
    Task,
    Project,
    Plugin,
    Engine,
    Run,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckState {
    Ready,
    NeedsUserInput,
    Blocked,
    Deferred,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionState {
    Planned,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStep {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub argv: Vec<String>,
    pub state: StepState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<PathBuf>,
}

impl ExecutionStep {
    pub fn new<I, S>(
        id: impl Into<String>,
        name: impl Into<String>,
        executable: impl Into<String>,
        args: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let executable = executable.into();
        let mut argv = vec![executable.clone()];
        argv.extend(args.into_iter().map(|arg| arg.as_ref().to_string()));
        Self {
            id: id.into(),
            name: name.into(),
            executable,
            argv,
            state: StepState::Pending,
            started_at: None,
            finished_at: None,
            exit_code: None,
            log: None,
        }
    }

    pub fn shape(&self) -> StepShape {
        StepShape {
            id: self.id.clone(),
            executable: self.executable.clone(),
            argv: self.argv.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentStep {
    pub id: String,
    pub name: String,
}

impl From<&ExecutionStep> for CurrentStep {
    fn from(step: &ExecutionStep) -> Self {
        Self {
            id: step.id.clone(),
            name: step.name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub kind: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRef {
    pub kind: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionPlan {
    pub execution_id: String,
    pub domain: ExecutionDomain,
    pub action: ExecutionAction,
    pub source: SourceContext,
    pub state: ExecutionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<CurrentStep>,
    pub steps: Vec<ExecutionStep>,
    pub artifacts: Vec<ArtifactRef>,
    pub diagnostics: Vec<DiagnosticRef>,
}

impl ExecutionPlan {
    pub fn new(
        execution_id: impl Into<String>,
        domain: ExecutionDomain,
        action: ExecutionAction,
        source: SourceContext,
        steps: Vec<ExecutionStep>,
    ) -> Self {
        let current_step = steps.first().map(CurrentStep::from);
        Self {
            execution_id: execution_id.into(),
            domain,
            action,
            source,
            state: ExecutionState::Planned,
            current_step,
            steps,
            artifacts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn step_shape(&self) -> Vec<StepShape> {
        self.steps.iter().map(ExecutionStep::shape).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepShape {
    pub id: String,
    pub executable: String,
    pub argv: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRecord {
    pub execution_id: String,
    pub domain: ExecutionDomain,
    pub action: ExecutionAction,
    pub source: SourceContext,
    pub state: ExecutionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<CurrentStep>,
    pub steps: Vec<ExecutionStep>,
    pub artifacts: Vec<ArtifactRef>,
    pub diagnostics: Vec<DiagnosticRef>,
}

impl ExecutionRecord {
    pub fn planned(plan: ExecutionPlan) -> Self {
        Self {
            execution_id: plan.execution_id,
            domain: plan.domain,
            action: plan.action,
            source: plan.source,
            state: ExecutionState::Planned,
            current_step: plan.current_step,
            steps: plan.steps,
            artifacts: plan.artifacts,
            diagnostics: plan.diagnostics,
        }
    }

    pub fn with_artifact(mut self, kind: impl Into<String>, path: PathBuf) -> Self {
        self.artifacts.push(ArtifactRef {
            kind: kind.into(),
            path,
        });
        self
    }

    pub fn with_diagnostic<P>(
        mut self,
        kind: impl Into<String>,
        summary: impl Into<String>,
        path: Option<P>,
    ) -> Self
    where
        P: Into<PathBuf>,
    {
        self.diagnostics.push(DiagnosticRef {
            kind: kind.into(),
            summary: summary.into(),
            path: path.map(Into::into),
        });
        self
    }
}

pub fn build_plan(
    execution_id: impl Into<String>,
    action: ExecutionAction,
    source: SourceContext,
    steps: Vec<ExecutionStep>,
) -> ExecutionPlan {
    ExecutionPlan::new(execution_id, ExecutionDomain::Build, action, source, steps)
}

pub fn package_plan(
    execution_id: impl Into<String>,
    action: ExecutionAction,
    source: SourceContext,
    steps: Vec<ExecutionStep>,
) -> ExecutionPlan {
    ExecutionPlan::new(
        execution_id,
        ExecutionDomain::Package,
        action,
        source,
        steps,
    )
}

pub fn plugin_package_steps<I, S>(plugin: impl AsRef<str>, platforms: I) -> Vec<ExecutionStep>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let plugin = plugin.as_ref();
    let platform_list = platforms
        .into_iter()
        .map(|platform| platform.as_ref().to_string())
        .collect::<Vec<_>>()
        .join("+");
    vec![
        ExecutionStep::new(
            "resolve",
            "Resolve plugin closure",
            "udf",
            ["package", "plugin", plugin, "--platform", &platform_list],
        ),
        ExecutionStep::new(
            "build",
            "Build plugin package matrix",
            "UnrealBuildTool",
            ["BuildPlugin", plugin, "-Platforms", &platform_list],
        ),
        ExecutionStep::new(
            "archive",
            "Archive plugin artifacts",
            "udf",
            ["package", "plugin", plugin, "--archive"],
        ),
    ]
}
