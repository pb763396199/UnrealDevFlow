//! Editor and UBT process detection and management

use crate::error::{Result, UdfError};
use sysinfo::System;

pub fn is_editor_running() -> bool {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for process in sys.processes().values() {
        if process.name().to_string_lossy().contains("UnrealEditor") {
            return true;
        }
    }
    false
}

pub fn get_editor_processes() -> Vec<sysinfo::Pid> {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    sys.processes()
        .iter()
        .filter(|(_, p)| p.name().to_string_lossy().contains("UnrealEditor"))
        .map(|(pid, _)| *pid)
        .collect()
}

/// Check if UnrealBuildTool is currently running.
/// Detects both UnrealBuildTool.exe and dotnet processes running UnrealBuildTool.dll.
pub fn is_ubt_running() -> bool {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for process in sys.processes().values() {
        let name = process.name().to_string_lossy();
        // Check for UnrealBuildTool.exe
        if name.contains("UnrealBuildTool") {
            return true;
        }
        // Check for dotnet running UnrealBuildTool.dll
        if name.contains("dotnet") {
            let cmd = process.cmd();
            let cmd_str: String = cmd
                .iter()
                .map(|s| s.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ");
            if cmd_str.contains("UnrealBuildTool") {
                return true;
            }
        }
    }
    false
}

pub fn check_editor_and_warn() -> Result<bool> {
    if is_editor_running() {
        crate::output::print_warning("UnrealEditor is currently running.");
        crate::output::print_warning(
            "Junction switch will only take effect on next Editor launch.",
        );

        let should_close = dialoguer::Confirm::new()
            .with_prompt("Do you want to close the Editor now?")
            .default(false)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;

        if should_close {
            // TODO: Implement editor close logic
            crate::output::print_info("Editor close requested. Please close it manually for now.");
        }

        Ok(true)
    } else {
        Ok(false)
    }
}
