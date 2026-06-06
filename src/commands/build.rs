//! Build command implementation

use crate::config::Config;
use crate::error::{BuildError, Result, UdfError};
use crate::host;
use crate::output;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn run(task_id: &str, background: bool, no_mutex: bool) -> Result<()> {
    let config = Config::load()?;

    // Get task host
    let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
    let uproject_path = host_dir.join(format!("T-{}_Host.uproject", task_id));

    if !uproject_path.exists() {
        return Err(UdfError::TaskNotFound(task_id.to_string()));
    }

    // Detect engine path
    let engine_path = &config.engine_path;

    let build_bat = engine_path.join("Engine").join("Build").join("BatchFiles").join("Build.bat");
    if !build_bat.exists() {
        return Err(BuildError::BuildBatNotFound(build_bat).into());
    }

    output::print_info(&format!("Building task '{}'...", task_id));
    output::print_info(&format!("  Project: {:?}", uproject_path));
    output::print_info(&format!("  Engine: {:?}", engine_path));

    // Build arguments
    let mut args = vec![
        "UnrealEditor".to_string(),
        "Win64".to_string(),
        "Development".to_string(),
        format!("-Project={}", uproject_path.to_string_lossy()),
        "-architecture=x64".to_string(),
    ];

    if no_mutex {
        args.push("-NoMutex".to_string());
    } else {
        args.push("-WaitMutex".to_string());
    }

    // Execute build
    let mut cmd = Command::new(&build_bat);
    cmd.args(&args);

    if background {
        output::print_info("Starting background build...");
        let child = cmd.spawn()?;
        output::print_success(&format!("Build started in background (PID: {})", child.id()));
    } else {
        let status = cmd.status()?;

        if !status.success() {
            return Err(BuildError::BuildFailed(status.code().unwrap_or(-1)).into());
        }

        // Verify DLL was produced
        let dll_path = host_dir
            .join("Plugins")
            .join("AesWorld")
            .join("Binaries")
            .join("Win64")
            .join("UnrealEditor-AesWorld.dll");

        if !dll_path.exists() {
            return Err(BuildError::DllNotProduced(dll_path).into());
        }

        // Check DLL timestamp
        let dll_meta = fs::metadata(&dll_path)?;
        let dll_modified = dll_meta.modified()?;
        let now = std::time::SystemTime::now();
        let age = now.duration_since(dll_modified).unwrap_or_default();

        if age.as_secs() > 60 {
            output::print_warning(&format!("DLL is {} seconds old", age.as_secs()));
        }

        output::print_success(&format!("Task '{}' built successfully!", task_id));
        output::print_info(&format!("  DLL: {:?}", dll_path));

        // Update .udf-meta.json
        let mut meta = host::read_meta(&host_dir)?;
        meta.last_built = Some(chrono::Utc::now().to_rfc3339());
        host::write_meta(&host_dir, &meta)?;
    }

    Ok(())
}
