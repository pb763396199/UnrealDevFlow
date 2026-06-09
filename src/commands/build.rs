//! Build command implementation

use crate::config::Config;
use crate::error::{BuildError, Result, UdfError};
use crate::host::{self, BuildStatus};
use crate::output;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run(task_id: &str, background: bool, no_mutex: bool, safe: bool) -> Result<()> {
    let config = Config::load()?;

    // Get task host
    let host_dir = host::get_task_host(&config.hosts_root, task_id)?;
    let uproject_path = host_dir.join(format!("T-{}_Host.uproject", task_id));

    if !uproject_path.exists() {
        return Err(UdfError::TaskNotFound(task_id.to_string()));
    }

    // Detect engine path
    let engine_path = &config.engine_path;

    let build_bat = engine_path
        .join("Engine")
        .join("Build")
        .join("BatchFiles")
        .join("Build.bat");
    if !build_bat.exists() {
        return Err(BuildError::BuildBatNotFound(build_bat).into());
    }

    // === Log isolation: each build gets its own log file ===
    let log_dir = host_dir.join("Logs");
    fs::create_dir_all(&log_dir)?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let ubt_log = log_dir.join(format!("Build_{}.log", timestamp));

    // === Smart mutex selection ===
    let mutex_mode = determine_mutex_mode(no_mutex, safe, engine_path);

    output::print_info(&format!("Building task '{}'...", task_id));
    output::print_info(&format!("  Project: {:?}", uproject_path));
    output::print_info(&format!("  Engine: {:?}", engine_path));
    output::print_info(&format!("  Mutex: {}", mutex_mode));
    output::print_info(&format!("  UBT Log: {:?}", ubt_log));
    output::print_info("  Strict mode: -FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity");

    // Build arguments - Strict mode (default)
    // Catches header/cpp mismatches that UBT optimizations may otherwise hide.
    //
    // Source: UE_5.5/Engine/Source/Programs/UnrealBuildTool/Configuration/
    //   BuildConfiguration.cs, TargetDescriptor.cs, TargetRules.cs
    //
    // Strict flags (~10-20% slower, but catches dependency bugs):
    //   -FailIfGeneratedCodeChanges  Fail if UHT-generated .generated.h is stale
    //   -NoUBTMakefiles              Bypass UBT dependency-graph cache
    //   -DisableAdaptiveUnity        Disable heuristic that excludes "working set" files
    let mut args = vec![
        "UnrealEditor".to_string(),
        "Win64".to_string(),
        "Development".to_string(),
        format!("-Project={}", uproject_path.to_string_lossy()),
        "-architecture=x64".to_string(),
        format!("-Log={}", ubt_log.to_string_lossy()),
        mutex_mode.clone(),
        "-FailIfGeneratedCodeChanges".to_string(),
        "-NoUBTMakefiles".to_string(),
        "-DisableAdaptiveUnity".to_string(),
    ];

    // Execute build
    let mut cmd = Command::new(&build_bat);
    cmd.args(&args);

    if background {
        // === Background mode: capture stdout/stderr to console log ===
        let console_log = log_dir.join(format!("Console_{}.log", timestamp));
        let log_file = std::fs::File::create(&console_log)?;

        cmd.stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file));

        let child = cmd.spawn()?;

        // Update meta with build status
        let mut meta = host::read_meta(&host_dir)?;
        meta.build_pid = Some(child.id());
        meta.build_log = Some(ubt_log.clone());
        meta.console_log = Some(console_log.clone());
        meta.build_status = Some(BuildStatus {
            state: "building".to_string(),
            started: chrono::Utc::now().to_rfc3339(),
            finished: None,
            exit_code: None,
            mutex_mode: mutex_mode.clone(),
        });
        host::write_meta(&host_dir, &meta)?;

        output::print_success(&format!(
            "Build started in background (PID: {})",
            child.id()
        ));
        output::print_info(&format!("  Console: {:?}", console_log));
        output::print_info(&format!(
            "  Check status: unrealdevflow build-status {}",
            task_id
        ));
    } else {
        let status = cmd.status()?;

        // Update meta with build result
        let mut meta = host::read_meta(&host_dir)?;
        meta.last_built = Some(chrono::Utc::now().to_rfc3339());
        meta.build_pid = None;
        meta.build_log = Some(ubt_log.clone());
        meta.build_status = Some(BuildStatus {
            state: if status.success() {
                "success".to_string()
            } else {
                "failed".to_string()
            },
            started: meta
                .build_status
                .as_ref()
                .map(|s| s.started.clone())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            finished: Some(chrono::Utc::now().to_rfc3339()),
            exit_code: Some(status.code().unwrap_or(-1)),
            mutex_mode: mutex_mode.clone(),
        });
        host::write_meta(&host_dir, &meta)?;

        if !status.success() {
            output::print_error(&format!(
                "Build failed with exit code: {}",
                status.code().unwrap_or(-1)
            ));
            output::print_info(&format!("  Check log: {:?}", ubt_log));
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
        output::print_info(&format!("  UBT Log: {:?}", ubt_log));
    }

    Ok(())
}

/// Determine mutex mode based on flags and engine state
fn determine_mutex_mode(no_mutex: bool, safe: bool, engine_path: &Path) -> String {
    if safe {
        return "-WaitMutex".to_string();
    }
    if no_mutex {
        return "-NoMutex".to_string();
    }
    // Default: smart detection
    if engine_intermediate_ready(engine_path) {
        "-NoMutex".to_string() // Daily build, safe to parallelize
    } else {
        "-WaitMutex".to_string() // First build, need to queue
    }
}

/// Check if engine intermediate files are ready (shared PCH etc.)
fn engine_intermediate_ready(engine_path: &Path) -> bool {
    let shared_dir = engine_path
        .join("Engine")
        .join("Intermediate")
        .join("Build")
        .join("Shared");

    if !shared_dir.exists() {
        return false;
    }

    // Check if directory has any files (engine has been compiled before)
    fs::read_dir(&shared_dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}
