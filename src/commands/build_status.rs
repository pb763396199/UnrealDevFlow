//! Build status command implementation

use crate::config::Config;
use crate::error::Result;
use crate::host::{self, BuildStatus};
use crate::output;
use std::fs;

pub fn run(task_ref: Option<String>) -> Result<()> {
    let config = Config::load()?;
    // Reading build status is a query, so it defaults to the task you were
    // last working on — same rule as `build check` and `task next`.
    let task_id = match task_ref {
        Some(value) => value,
        None => crate::commands::simple::latest_task_ref(&config)?,
    };
    let task_id = task_id.as_str();
    let (host_dir, mut meta, _task_context) = host::resolve_task(&config, task_id)?;

    let process_running = meta.build_pid.is_some_and(is_process_running);
    if reconcile_exited_background_build(&mut meta, process_running) {
        host::write_meta(&host_dir, &meta)?;
        output::print_warning(
            "后台构建进程已退出，但拿不到可靠退出码；状态标记为 unknown，需要看日志或重新受控构建来对账。",
        );
    }

    let running = meta.build_pid.map(is_process_running);
    let log_tail = meta
        .build_log
        .as_ref()
        .filter(|path| path.exists())
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|content| {
            let lines: Vec<&str> = content.lines().collect();
            lines
                .iter()
                .rev()
                .take(5)
                .rev()
                .map(|line| (*line).to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let data = BuildStatusReport {
        task_ref: task_id.to_string(),
        state: meta
            .build_status
            .as_ref()
            .map(|status| status.state.clone()),
        started: meta.build_status.as_ref().map(|s| s.started.clone()),
        finished: meta.build_status.as_ref().and_then(|s| s.finished.clone()),
        exit_code: meta.build_status.as_ref().and_then(|s| s.exit_code),
        mutex_mode: meta.build_status.as_ref().map(|s| s.mutex_mode.clone()),
        build_pid: meta.build_pid,
        process_running: running,
        ubt_log: meta
            .build_log
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        console_log: meta
            .console_log
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        last_built: meta.last_built.clone(),
        log_tail,
    };
    output::emit("build status", data, render_status);
    Ok(())
}

/// Everything a caller needs to decide whether a build is done, and how it went.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildStatusReport {
    task_ref: String,
    state: Option<String>,
    started: Option<String>,
    finished: Option<String>,
    exit_code: Option<i32>,
    mutex_mode: Option<String>,
    build_pid: Option<u32>,
    process_running: Option<bool>,
    ubt_log: Option<String>,
    console_log: Option<String>,
    last_built: Option<String>,
    log_tail: Vec<String>,
}

fn render_status(data: &BuildStatusReport) -> String {
    let mut lines = vec![format!("Build status for task '{}':", data.task_ref)];
    match &data.state {
        Some(state) => {
            lines.push(format!("  State: {}", state));
            if let Some(started) = &data.started {
                lines.push(format!("  Started: {}", started));
            }
            if let Some(finished) = &data.finished {
                lines.push(format!("  Finished: {}", finished));
            }
            if let Some(code) = data.exit_code {
                lines.push(format!("  Exit code: {}", code));
            }
            if let Some(mode) = &data.mutex_mode {
                lines.push(format!("  Mutex mode: {}", mode));
            }
        }
        None => lines.push("  No build status recorded".to_string()),
    }
    if let (Some(pid), Some(running)) = (data.build_pid, data.process_running) {
        lines.push(format!(
            "  Process {} {}",
            pid,
            if running {
                "is still running"
            } else {
                "has exited"
            }
        ));
    }
    if let Some(log) = &data.ubt_log {
        lines.push(format!("  UBT Log: {}", log));
    }
    if !data.log_tail.is_empty() {
        lines.push("  Last 5 lines of UBT log:".to_string());
        for line in &data.log_tail {
            lines.push(format!("    {}", line));
        }
    }
    if let Some(log) = &data.console_log {
        lines.push(format!("  Console Log: {}", log));
    }
    if let Some(last) = &data.last_built {
        lines.push(format!("  Last built: {}", last));
    }
    lines.join(
        "
",
    )
}

/// A background build whose process is gone leaves no exit code behind.
///
/// Leaving it as `building` forever reads as "still compiling", which is worse
/// than admitting we do not know: mark it `unknown` so the next controlled
/// build reconciles it.
fn reconcile_exited_background_build(meta: &mut host::TaskMeta, process_running: bool) -> bool {
    if meta
        .build_status
        .as_ref()
        .is_none_or(|status| status.state != "building")
        || meta.build_pid.is_none()
        || process_running
    {
        return false;
    }

    let started = meta
        .build_status
        .as_ref()
        .map(|status| status.started.clone())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let mutex_mode = meta
        .build_status
        .as_ref()
        .map(|status| status.mutex_mode.clone())
        .unwrap_or_default();
    meta.build_pid = None;
    meta.build_status = Some(BuildStatus {
        state: "unknown".to_string(),
        started,
        finished: Some(chrono::Utc::now().to_rfc3339()),
        exit_code: None,
        mutex_mode,
    });
    true
}

/// Check if a process is still running by PID
fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{CURRENT_SCHEMA_VERSION, TaskMeta};

    fn building_meta() -> TaskMeta {
        TaskMeta {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: "background-build".to_string(),
            name: "background-build".to_string(),
            branch: "feature/background-build".to_string(),
            created: "2026-07-16T00:00:00Z".to_string(),
            based_on: "0123456789abcdef".to_string(),
            status: "active".to_string(),
            prompt: None,
            last_built: None,
            build_pid: Some(42),
            build_log: None,
            console_log: None,
            build_status: Some(BuildStatus {
                state: "building".to_string(),
                started: "2026-07-16T00:00:00Z".to_string(),
                finished: None,
                exit_code: None,
                mutex_mode: "-WaitMutex".to_string(),
            }),
            primary_plugins: Vec::new(),
            dependency_plugins: Vec::new(),
            workspace: Some("test".to_string()),
            task_uid: Some("test/background-build".to_string()),
            context: None,
        }
    }

    #[test]
    fn exited_background_process_becomes_unknown_not_success() {
        let mut meta = building_meta();
        assert!(reconcile_exited_background_build(&mut meta, false));
        let status = meta.build_status.expect("reconciled status");
        assert_eq!(status.state, "unknown");
        assert_eq!(status.exit_code, None);
        assert!(status.finished.is_some());
        assert_eq!(meta.build_pid, None);
    }

    #[test]
    fn running_background_process_remains_building() {
        let mut meta = building_meta();
        assert!(!reconcile_exited_background_build(&mut meta, true));
        assert_eq!(meta.build_status.unwrap().state, "building");
        assert_eq!(meta.build_pid, Some(42));
    }
}
