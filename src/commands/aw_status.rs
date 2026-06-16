//! AgentWatcher read-only status command.

use crate::error::Result;
use serde_json::json;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let config_path = crate::config::Config::config_path()
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let config = crate::config::Config::load().ok();
    let configured = config.is_some();
    let workspace_names = config
        .as_ref()
        .map(|config| config.workspace_names())
        .unwrap_or_default();
    let workspace_count = workspace_names.len();
    let state = crate::state::GlobalState::load().ok();
    let active_projects = state
        .as_ref()
        .map(|state| {
            let mut projects = state
                .projects
                .iter()
                .map(|(name, project)| {
                    json!({
                        "name": name,
                        "path": project.path.to_string_lossy().to_string(),
                        "activeTask": project.active_task.clone(),
                        "previousTask": project.previous_task.clone(),
                        "junctionCount": project.junctions.len(),
                    })
                })
                .collect::<Vec<_>>();
            projects.sort_by(|left, right| {
                left["name"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(right["name"].as_str().unwrap_or_default())
            });
            projects
        })
        .unwrap_or_default();
    let result = json!({
        "schemaVersion": 1,
        "kind": "AgentWatcherModuleResult",
        "moduleName": "UnrealDevFlow",
        "commandName": "status",
        "status": "success",
        "summary": "开发流状态检查完成",
        "exitCode": 0,
        "details": {
            "displayName": "开发流",
            "version": env!("UDF_VERSION_LONG"),
            "configured": configured,
            "configPath": config_path,
            "workspaceCount": workspace_count,
            "workspaces": workspace_names,
            "activeProjects": active_projects,
            "currentDirectory": cwd,
            "capabilities": [
                {
                    "name": "workspaceStatus",
                    "safety": "readOnly",
                    "summary": "列出已注册 UE 工作区和配置路径"
                },
                {
                    "name": "taskStatus",
                    "safety": "readOnly",
                    "summary": "列出当前活动任务、上一个任务和 Junction 数量"
                },
                {
                    "name": "futureTaskCreate",
                    "safety": "boundedWrite",
                    "summary": "后续阶段可接入 create/start/switch，但本阶段不开放"
                }
            ]
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
