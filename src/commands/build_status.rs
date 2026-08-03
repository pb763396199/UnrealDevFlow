//! Build status command implementation

use crate::config::Config;
use crate::error::Result;
use crate::host::{self, BuildStatus};
use crate::output;
use std::fs;

pub fn run(task_id: &str) -> Result<()> {
    let config = Config::load()?;
    let (host_dir, mut meta, _task_context) = host::resolve_task(&config, task_id)?;

    let process_running = meta.build_pid.is_some_and(is_process_running);
    if reconcile_exited_background_build(&mut meta, process_running) {
        host::write_meta(&host_dir, &meta)?;
        output::print_warning(
            "后台构建进程已退出，但拿不到可靠退出码；状态标记为 unknown，需要看日志或重新受控构建来对账。",
        );
    }

    output::print_info(&format!("Build status for task '{}':", task_id));

    // Check build status from meta
    match &meta.build_status {
        Some(status) => {
            output::print_info(&format!("  State: {}", status.state));
            output::print_info(&format!("  Started: {}", status.started));
            if let Some(finished) = &status.finished {
                output::print_info(&format!("  Finished: {}", finished));
            }
            if let Some(exit_code) = status.exit_code {
                output::print_info(&format!("  Exit code: {}", exit_code));
            }
            output::print_info(&format!("  Mutex mode: {}", status.mutex_mode));
        }
        None => {
            output::print_info("  No build status recorded");
        }
    }

    // Check if PID is still running
    if let Some(pid) = meta.build_pid {
        if is_process_running(pid) {
            output::print_warning(&format!("  Process {} is still running", pid));
        } else {
            output::print_info(&format!("  Process {} has exited", pid));
        }
    }

    // Show log file locations
    if let Some(build_log) = &meta.build_log
        && build_log.exists()
    {
        output::print_info(&format!("  UBT Log: {:?}", build_log));
        // Show last few lines of log
        if let Ok(content) = fs::read_to_string(build_log) {
            let lines: Vec<&str> = content.lines().collect();
            if lines.len() > 5 {
                output::print_info("  Last 5 lines of UBT log:");
                for line in lines.iter().rev().take(5).rev() {
                    output::print_info(&format!("    {}", line));
                }
            }
        }
    }

    if let Some(console_log) = &meta.console_log
        && console_log.exists()
    {
        output::print_info(&format!("  Console Log: {:?}", console_log));
    }

    // Show last built time
    if let Some(last_built) = &meta.last_built {
        output::print_info(&format!("  Last built: {}", last_built));
    }

    Ok(())
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
