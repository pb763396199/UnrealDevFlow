//! Build status command implementation

use crate::config::Config;
use crate::error::Result;
use crate::host;
use crate::output;
use std::fs;

pub fn run(task_id: &str) -> Result<()> {
    let config = Config::load()?;
    let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
    let meta = host::read_meta(&host_dir)?;

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
    if let Some(build_log) = &meta.build_log {
        if build_log.exists() {
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
    }

    if let Some(console_log) = &meta.console_log {
        if console_log.exists() {
            output::print_info(&format!("  Console Log: {:?}", console_log));
        }
    }

    // Show last built time
    if let Some(last_built) = &meta.last_built {
        output::print_info(&format!("  Last built: {}", last_built));
    }

    Ok(())
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
