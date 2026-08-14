#[path = "../src/execution.rs"]
mod execution;
#[path = "../src/source_context.rs"]
mod source_context;

use execution::{
    CheckState, ExecutionAction, ExecutionDomain, ExecutionRecord, ExecutionState, PlanReport,
    ReadinessCheck, ReadinessReport, StepState, build_plan, package_plan,
};
use serde_json::Value;
use source_context::{
    SourceContext, SourceEnvironment, SourcePlugin, SourceRef, TaskSourceContext,
};
use std::path::PathBuf;

fn project() -> PathBuf {
    PathBuf::from(r"F:\ShanghaiP4\neon\UGA\DEV_1")
}

#[test]
fn check_and_plan_have_distinct_non_execution_contracts() {
    let source = workspace_source();
    let check = ReadinessReport {
        domain: ExecutionDomain::Package,
        action: ExecutionAction::Plugin,
        source: source.clone(),
        readiness: CheckState::Ready,
        checks: vec![ReadinessCheck {
            name: "ubt".to_string(),
            state: CheckState::Ready,
            summary: "UnrealBuildTool is available".to_string(),
        }],
        diagnostics: Vec::new(),
        next_command: Some("udf package plugin AesWorld --workspace neon-dev1".to_string()),
    };
    let plan = PlanReport::new(
        ExecutionDomain::Package,
        ExecutionAction::Plugin,
        source,
        execution::plugin_package_steps("AesWorld", ["Win64"]),
        vec![PathBuf::from(r"F:\Artifacts\AesWorld")],
        Vec::new(),
    );

    let check_json = serde_json::to_value(check).expect("check json");
    let plan_json = serde_json::to_value(plan).expect("plan json");
    assert_eq!(check_json["readiness"], "ready");
    assert!(check_json.get("executionId").is_none());
    assert!(check_json.get("steps").is_none());
    assert!(
        plan_json["planDigest"]
            .as_str()
            .unwrap()
            .starts_with("md5:")
    );
    assert!(plan_json.get("executionId").is_none());
    assert!(plan_json.get("readiness").is_none());
}

fn engine() -> PathBuf {
    PathBuf::from(r"F:\Unreal Engine\UE_5.5")
}

fn plugins_root() -> PathBuf {
    PathBuf::from(r"F:\ShanghaiP4\neon\Plugins")
}

fn main_plugin() -> PathBuf {
    PathBuf::from(r"F:\ShanghaiP4\neon\Plugins\AesWorld")
}

fn host() -> PathBuf {
    PathBuf::from(r"F:\ShanghaiP4\neon\Hosts\W-neon-dev1\T-packaging_Host")
}

fn primary_plugin(path: PathBuf, revision: &str) -> SourcePlugin {
    SourcePlugin::primary("AesWorld", path, revision)
}

fn task_source() -> SourceContext {
    SourceContext::from_task(TaskSourceContext::new(
        SourceEnvironment::new("neon-dev1", project(), engine(), plugins_root(), "task-rev")
            .with_primary_plugins(vec![primary_plugin(
                host().join("Plugins").join("AesWorld"),
                "task-rev",
            )])
            .with_dependency_plugins(vec![SourcePlugin::dependency(
                "AesShared",
                plugins_root().join("AesShared"),
                "dep-rev",
            )]),
        "neon-dev1/packaging",
        host(),
    ))
}

fn workspace_source() -> SourceContext {
    SourceContext::from_workspace(
        SourceEnvironment::new(
            "neon-dev1",
            project(),
            engine(),
            plugins_root(),
            "workspace-rev",
        )
        .with_primary_plugins(vec![primary_plugin(main_plugin(), "workspace-rev")])
        .with_dependency_plugins(vec![SourcePlugin::dependency(
            "AesShared",
            plugins_root().join("AesShared"),
            "dep-rev",
        )]),
    )
}

fn workspace_environment(revision: &str) -> SourceEnvironment {
    SourceEnvironment::new("neon-dev1", project(), engine(), plugins_root(), revision)
}

#[test]
fn source_context_serializes_task_and_workspace_sources_explicitly() {
    let task = serde_json::to_value(task_source()).expect("task source json");
    assert_eq!(task["sourceRef"]["kind"], "task");
    assert_eq!(task["sourceRef"]["value"], "neon-dev1/packaging");
    assert_eq!(task["workspace"], "neon-dev1");
    assert_eq!(
        task["project"].as_str().unwrap(),
        project().to_string_lossy()
    );
    assert_eq!(task["engine"].as_str().unwrap(), engine().to_string_lossy());
    assert_eq!(
        task["pluginsRoot"].as_str().unwrap(),
        plugins_root().to_string_lossy()
    );
    assert_eq!(task["host"].as_str().unwrap(), host().to_string_lossy());
    assert_eq!(task["primaryPlugins"][0]["name"], "AesWorld");
    assert_eq!(task["primaryPlugins"][0]["role"], "primary");
    assert_eq!(task["dependencyPlugins"][0]["role"], "dependency");
    assert_eq!(task["sourceRevision"], "task-rev");

    let workspace = serde_json::to_value(workspace_source()).expect("workspace source json");
    assert_eq!(workspace["sourceRef"]["kind"], "workspace");
    assert_eq!(workspace["sourceRef"]["value"], "neon-dev1");
    assert!(workspace.get("host").is_none());
}

#[test]
fn execution_contract_uses_camel_case_fields_and_fixed_state_words() {
    let plan = package_plan(
        "exec-001",
        ExecutionAction::Plugin,
        task_source(),
        vec![
            execution::ExecutionStep::new(
                "resolve",
                "Resolve plugin closure",
                "udf",
                [
                    "package",
                    "plugin",
                    "AesWorld",
                    "--task",
                    "neon-dev1/packaging",
                ],
            ),
            execution::ExecutionStep::new(
                "archive",
                "Archive plugin package",
                "RunUAT.bat",
                ["BuildPlugin", "-Plugin=AesWorld.uplugin"],
            ),
        ],
    );

    let record = ExecutionRecord::planned(plan)
        .with_artifact("pluginPackage", PathBuf::from(r"F:\Artifacts\AesWorld.zip"))
        .with_diagnostic("preflight", "ready to execute", None::<PathBuf>);

    let json = serde_json::to_value(&record).expect("record json");
    let object = json.as_object().expect("record object");
    assert!(object.contains_key("executionId"));
    assert!(object.contains_key("domain"));
    assert!(object.contains_key("action"));
    assert!(object.contains_key("source"));
    assert!(object.contains_key("state"));
    assert!(object.contains_key("currentStep"));
    assert!(object.contains_key("steps"));
    assert!(object.contains_key("artifacts"));
    assert!(object.contains_key("diagnostics"));

    assert_eq!(json["executionId"], "exec-001");
    assert_eq!(json["domain"], "package");
    assert_eq!(json["action"], "plugin");
    assert_eq!(json["state"], "planned");
    assert_eq!(json["currentStep"]["id"], "resolve");
    assert_eq!(json["steps"][0]["state"], "pending");
    assert_eq!(json["steps"][0]["argv"][0], "udf");
    assert_eq!(json["artifacts"][0]["kind"], "pluginPackage");
    assert_eq!(json["diagnostics"][0]["kind"], "preflight");

    assert_eq!(serde_json::to_value(CheckState::Ready).unwrap(), "ready");
    assert_eq!(
        serde_json::to_value(CheckState::NeedsUserInput).unwrap(),
        "needsUserInput"
    );
    assert_eq!(
        serde_json::to_value(CheckState::Blocked).unwrap(),
        "blocked"
    );
    assert_eq!(
        serde_json::to_value(CheckState::Deferred).unwrap(),
        "deferred"
    );
    assert_eq!(
        serde_json::to_value(ExecutionState::Running).unwrap(),
        "running"
    );
    assert_eq!(
        serde_json::to_value(ExecutionState::Succeeded).unwrap(),
        "succeeded"
    );
    assert_eq!(
        serde_json::to_value(ExecutionState::Failed).unwrap(),
        "failed"
    );
    assert_eq!(
        serde_json::to_value(ExecutionState::Cancelled).unwrap(),
        "cancelled"
    );
    assert_eq!(
        serde_json::to_value(ExecutionState::Unknown).unwrap(),
        "unknown"
    );
    assert_eq!(serde_json::to_value(StepState::Skipped).unwrap(), "skipped");
}

#[test]
fn task_and_workspace_plugin_package_plans_have_the_same_execution_shape() {
    let task_plan = package_plan(
        "task-exec",
        ExecutionAction::Plugin,
        task_source(),
        execution::plugin_package_steps("AesWorld", ["Win64", "Linux"]),
    );
    let workspace_plan = package_plan(
        "workspace-exec",
        ExecutionAction::Plugin,
        workspace_source(),
        execution::plugin_package_steps("AesWorld", ["Win64", "Linux"]),
    );

    assert_eq!(task_plan.domain, workspace_plan.domain);
    assert_eq!(task_plan.action, workspace_plan.action);
    assert_eq!(task_plan.step_shape(), workspace_plan.step_shape());
    assert_eq!(task_plan.steps.len(), 3);

    let task_json = serde_json::to_value(task_plan).expect("task plan json");
    let workspace_json = serde_json::to_value(workspace_plan).expect("workspace plan json");
    assert_eq!(task_json["steps"], workspace_json["steps"]);
    assert_ne!(task_json["source"], workspace_json["source"]);
    assert_eq!(task_json["source"]["sourceRef"]["kind"], "task");
    assert_eq!(workspace_json["source"]["sourceRef"]["kind"], "workspace");
}

#[test]
fn build_and_package_helpers_share_domain_action_and_step_contracts() {
    let build = build_plan(
        "build-engine",
        ExecutionAction::Engine,
        SourceContext::from_workspace(workspace_environment("engine-rev")),
        vec![execution::ExecutionStep::new(
            "build-engine",
            "Compile source engine",
            "Build.bat",
            ["UnrealEditor", "Win64", "Development"],
        )],
    );

    let package = package_plan(
        "package-engine",
        ExecutionAction::Engine,
        SourceContext {
            source_ref: SourceRef::workspace("neon-dev1"),
            ..build.source.clone()
        },
        vec![execution::ExecutionStep::new(
            "installed-build",
            "Create installed build",
            "RunUAT.bat",
            ["BuildGraph", "-target=Make Installed Build Win64"],
        )],
    );

    assert_eq!(build.domain, ExecutionDomain::Build);
    assert_eq!(package.domain, ExecutionDomain::Package);
    assert_eq!(build.action, ExecutionAction::Engine);
    assert_eq!(package.action, ExecutionAction::Engine);

    let build_json: Value = serde_json::to_value(build).expect("build json");
    let package_json: Value = serde_json::to_value(package).expect("package json");
    assert_eq!(build_json["state"], "planned");
    assert_eq!(package_json["state"], "planned");
    assert_eq!(build_json["steps"][0]["executable"], "Build.bat");
    assert_eq!(package_json["steps"][0]["executable"], "RunUAT.bat");
}
