//! Build command implementation (v2 multi-plugin, profile-aware)

use crate::build_profile::{BuildProfile, MutexMode, resolve_mutex};
use crate::config::Config;
use crate::editor;
use crate::error::{BuildError, Result, UdfError};
use crate::host::{self, BuildStatus, DependencyPlugin, DependencySource, PrimaryPlugin};
use crate::output;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn run(
    task_id: &str,
    background: bool,
    mutex_mode: MutexMode,
    validator_hint: bool,
    primary_only: bool,
    profile: BuildProfile,
    build_log_dir: Option<PathBuf>,
) -> Result<()> {
    let config = Config::load()?;

    let (host_dir, mut meta, task_context) = host::resolve_task(&config, task_id)?;
    crate::migration::backfill_source_repo(&mut meta, &config);

    let uproject_path = host_dir.join(format!("{}.uproject", host::task_project_name(&meta.id)));

    if !uproject_path.exists() {
        return Err(UdfError::TaskNotFound(task_id.to_string()));
    }

    let engine_path = &task_context.engine_path;

    let build_bat = engine_path
        .join("Engine")
        .join("Build")
        .join("BatchFiles")
        .join("Build.bat");
    if !build_bat.exists() {
        return Err(BuildError::BuildBatNotFound(build_bat).into());
    }

    // === Dependency dirty check (warn but don't block) ===
    check_dependency_dirty(&meta.dependency_plugins);

    // === Log directory: structured, profile-prefixed, overridable ===
    let log_dir = build_log_dir.unwrap_or_else(|| host_dir.join("Logs").join("UBT"));
    fs::create_dir_all(&log_dir)?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let ubt_log = log_dir.join(format!("Build_{}_{}.log", profile.label(), timestamp));

    let engine_ready = engine_intermediate_ready(engine_path);
    let effective_mutex = resolve_mutex(mutex_mode, validator_hint, engine_ready);

    output::print_info(&format!("Building task '{}'...", task_id));
    output::print_info(&format!("  Project: {:?}", uproject_path));
    output::print_info(&format!("  Engine:  {:?}", engine_path));
    output::print_info(&format!("  Profile: {}", profile.label()));
    output::print_info(&format!(
        "  Mutex:   {} (mode={:?} validator={} engine_ready={})",
        effective_mutex.as_arg(),
        mutex_mode,
        validator_hint,
        engine_ready
    ));
    output::print_info(&format!("  UBT Log: {:?}", ubt_log));
    if validator_hint {
        output::print_info("  ⚠ Validator mode: -NoMutex preferred to avoid queueing.");
    }
    let primary_only_modules = if primary_only {
        primary_only_module_names(&host_dir, &meta.primary_plugins)?
    } else {
        Vec::new()
    };
    if primary_only {
        output::print_info(&format!(
            "  Scope:   --primary-only ({} module(s))",
            primary_only_modules.len()
        ));
    }

    // === Mutex safety checks ===
    if effective_mutex == MutexMode::NoMutex {
        // NoMutex mode: check if another UBT is running (data safety)
        if editor::is_ubt_running() {
            output::print_warning("⚠ Another UBT process is running.");
            output::print_warning(
                "  -NoMutex may cause file conflicts in Engine/Intermediate/Build/Shared/.",
            );
            output::print_warning("  Consider using --mutex wait to queue safely.");
        }
    } else if effective_mutex == MutexMode::Wait {
        // WaitMutex mode: check if Editor is running (user experience)
        if editor::is_editor_running() {
            output::print_warning("⚠ UnrealEditor is currently running.");
            output::print_warning("  UBT will wait until you close the Editor.");
            output::print_warning("  Please close the Editor to proceed, or use Ctrl+C to cancel.");
        }
    }

    // === Build the UBT argument vector. NEVER fall back to string concat. ===
    let mut args: Vec<String> = vec![
        "UnrealEditor".to_string(),
        "Win64".to_string(),
        "Development".to_string(),
        format!("-Project={}", uproject_path.to_string_lossy()),
        "-architecture=x64".to_string(),
        format!("-Log={}", ubt_log.to_string_lossy()),
        effective_mutex.as_arg().to_string(),
    ];

    // Strict-compilation flags from the profile
    for flag in profile.flags() {
        args.push((*flag).to_string());
    }

    // Optional -Module= per primary plugin (used for incremental single-plugin builds)
    if primary_only {
        for module in &primary_only_modules {
            args.push(format!("-Module={}", module));
        }
    }

    let mut cmd = Command::new(&build_bat);
    cmd.args(&args);

    if background {
        let console_log = log_dir.join(format!("Console_{}_{}.log", profile.label(), timestamp));
        let log_file = std::fs::File::create(&console_log)?;

        cmd.stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file));

        let child = cmd.spawn()?;

        meta.build_pid = Some(child.id());
        meta.build_log = Some(ubt_log.clone());
        meta.console_log = Some(console_log.clone());
        meta.build_status = Some(BuildStatus {
            state: "building".to_string(),
            started: chrono::Utc::now().to_rfc3339(),
            finished: None,
            exit_code: None,
            mutex_mode: effective_mutex.as_arg().to_string(),
        });
        host::write_meta(&host_dir, &meta)?;

        output::print_success(&format!(
            "Build started in background (PID: {})",
            child.id()
        ));
        output::print_info(&format!("  Console: {:?}", console_log));
        output::print_info(&format!("  Check status: udf build status {}", task_id));
    } else {
        let status = cmd.status()?;

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
            mutex_mode: effective_mutex.as_arg().to_string(),
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

        // Verify a DLL was produced for every primary plugin.
        verify_primary_dlls(&host_dir, &meta.primary_plugins)?;

        output::print_success(&format!("Task '{}' built successfully!", task_id));
        output::print_info(&format!("  UBT Log: {:?}", ubt_log));
    }

    Ok(())
}

fn verify_primary_dlls(host_dir: &Path, primary_plugins: &[PrimaryPlugin]) -> Result<()> {
    if primary_plugins.is_empty() {
        return Ok(());
    }
    for plugin in primary_plugins {
        let dll_path = host_dir
            .join(&plugin.worktree)
            .join("Binaries")
            .join("Win64")
            .join(format!("UnrealEditor-{}.dll", plugin.name));

        if !dll_path.exists() {
            output::print_warning(&format!(
                "Primary plugin DLL not found: {:?} (module-level DLLs may still be valid)",
                dll_path
            ));
            continue;
        }

        let dll_meta = fs::metadata(&dll_path)?;
        let dll_modified = dll_meta.modified()?;
        let now = std::time::SystemTime::now();
        let age = now.duration_since(dll_modified).unwrap_or_default();

        if age.as_secs() > 60 {
            output::print_warning(&format!(
                "Primary plugin DLL is {} seconds old: {:?}",
                age.as_secs(),
                dll_path
            ));
        }

        output::print_info(&format!("  DLL: {:?}", dll_path));
    }
    Ok(())
}

fn primary_only_module_names(
    host_dir: &Path,
    primary_plugins: &[PrimaryPlugin],
) -> Result<Vec<String>> {
    let mut modules = Vec::new();
    for plugin in primary_plugins {
        let plugin_dir = host_dir.join(&plugin.worktree);
        let declared_modules = crate::plugin::uplugin::read_module_names(&plugin_dir)?;
        if declared_modules.is_empty() {
            modules.push(plugin.name.clone());
        } else {
            modules.extend(declared_modules);
        }
    }

    let mut seen = std::collections::HashSet::new();
    modules.retain(|module| seen.insert(module.clone()));
    Ok(modules)
}

fn check_dependency_dirty(deps: &[DependencyPlugin]) {
    for dep in deps {
        if dep.source != DependencySource::Project {
            continue;
        }
        let status = match Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&dep.source_path)
            .output()
        {
            Ok(o) => o,
            Err(_) => continue,
        };
        if !status.status.success() {
            continue;
        }
        let stdout = String::from_utf8_lossy(&status.stdout);
        if !stdout.trim().is_empty() {
            output::print_warning(&format!(
                "Dependency '{}' has uncommitted changes in main repo {:?}",
                dep.name, dep.source_path
            ));
            output::print_warning(
                "  Dependencies are treated as read-only by UnrealDevFlow; commit or stash before build.",
            );
        }
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

    fs::read_dir(&shared_dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

#[allow(dead_code)]
fn join_worktree(host_dir: &Path, rel: &Path) -> PathBuf {
    host_dir.join(rel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn primary_only_uses_uplugin_module_names_not_plugin_name() {
        let temp = TempDir::new().expect("temp dir");
        let host_dir = temp.path();
        let plugin_dir = host_dir.join("Plugins").join("AesWorld");
        fs::create_dir_all(&plugin_dir).expect("plugin dir");
        fs::write(
            plugin_dir.join("AesWorld.uplugin"),
            r#"{
                "FileVersion": 3,
                "Modules": [
                    { "Name": "AesEarth", "Type": "Runtime" },
                    { "Name": "AesPOI", "Type": "Runtime" }
                ]
            }"#,
        )
        .expect("uplugin");

        let modules = primary_only_module_names(
            host_dir,
            &[PrimaryPlugin {
                name: "AesWorld".to_string(),
                source_repo: PathBuf::from("unused"),
                worktree: PathBuf::from("Plugins/AesWorld"),
                branch: "task/test".to_string(),
                based_on: "abc123".to_string(),
            }],
        )
        .expect("module names");

        assert_eq!(modules, vec!["AesEarth".to_string(), "AesPOI".to_string()]);
        assert!(!modules.contains(&"AesWorld".to_string()));
    }

    #[test]
    fn primary_only_keeps_plugin_name_fallback_for_legacy_empty_uplugin() {
        let temp = TempDir::new().expect("temp dir");
        let host_dir = temp.path();
        let plugin_dir = host_dir.join("Plugins").join("SimplePlugin");
        fs::create_dir_all(&plugin_dir).expect("plugin dir");
        fs::write(plugin_dir.join("SimplePlugin.uplugin"), "{}").expect("uplugin");

        let modules = primary_only_module_names(
            host_dir,
            &[PrimaryPlugin {
                name: "SimplePlugin".to_string(),
                source_repo: PathBuf::from("unused"),
                worktree: PathBuf::from("Plugins/SimplePlugin"),
                branch: "task/test".to_string(),
                based_on: "abc123".to_string(),
            }],
        )
        .expect("module names");

        assert_eq!(modules, vec!["SimplePlugin".to_string()]);
    }
}
