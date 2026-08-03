//! UnrealDevFlow - Unreal Engine plugin parallel development workflow tool

// The CLI keeps several recovery/inspection helpers and structured error
// variants that are intentionally not wired into every release yet.
#![allow(dead_code)]

mod build_policy;
mod build_profile;
mod cli;
mod commands;
mod config;
mod editor;
mod error;
mod git;
mod host;
mod junction;
mod migration;
mod output;
mod plugin;
mod state;

use clap::Parser;
use cli::{Cli, Commands};
use error::Result;
use tracing_subscriber::EnvFilter;

fn main() {
    let cli = Cli::parse();
    // Decide how this run talks before anything can print.
    output::set_format(&cli.format);
    let command = command_name(&cli.command);

    match run(cli) {
        Ok(()) => output::flush_unemitted(command),
        Err(error) => {
            output::emit_failure(command, &format!("{}", error));
            std::process::exit(1);
        }
    }
}

/// Name reported in the JSON envelope, so callers can tell which command spoke.
fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::AwStatus => "aw-status",
        Commands::Workspace { .. } => "workspace",
        Commands::Task { .. } => "task",
        Commands::Build { .. } => "build",
        Commands::BuildStatus { .. } => "build-status",
        Commands::BuildCheck { .. } => "build-check",
        Commands::BuildGate { .. } => "build-gate",
        Commands::BuildProject { .. } => "build-project",
        Commands::Skills { .. } => "skills",
    }
}

fn run(cli: Cli) -> Result<()> {
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    if !matches!(
        cli.command,
        Commands::AwStatus
            | Commands::BuildGate { .. }
            | Commands::Workspace { .. }
            | Commands::Skills { .. }
    ) && !config::Config::exists()
    {
        return Err(error::UdfError::NotConfigured);
    }

    match cli.command {
        Commands::AwStatus => commands::aw_status::run()?,
        Commands::Workspace { action } => match action {
            cli::WorkspaceAction::Init {
                workspace,
                project,
                plugins_root,
                hosts_root,
                engine_path,
                yes,
                skip_skill_install,
            } => commands::init::run(
                workspace,
                project,
                plugins_root,
                hosts_root,
                engine_path,
                yes,
                skip_skill_install,
            )?,
            cli::WorkspaceAction::Add {
                name,
                project,
                hosts_root,
                plugins_root,
                engine_path,
                plugin_path,
                yes,
            } => commands::workspace::add(
                name,
                project,
                hosts_root,
                plugins_root,
                engine_path,
                plugin_path,
                yes,
            )?,
            cli::WorkspaceAction::List => commands::workspace::list()?,
            cli::WorkspaceAction::Doctor { name, deep } => {
                commands::workspace::doctor(&name, deep)?
            }
            cli::WorkspaceAction::Remove { name, yes } => commands::workspace::remove(&name, yes)?,
            cli::WorkspaceAction::Status => commands::status::run()?,
        },
        Commands::Task { action } => match action {
            cli::TaskAction::Create {
                description,
                id,
                branch,
                base_ref,
                prompt,
                workspace,
                primary,
                override_dep,
                yes,
            } => commands::create::run(
                &description,
                id,
                branch,
                base_ref,
                // A task without its original prompt loses the reason it exists,
                // so the description stands in when none is given.
                prompt.or_else(|| Some(description.clone())),
                workspace,
                primary,
                override_dep,
                yes,
            )?,
            cli::TaskAction::List { workspace } => commands::list::run(workspace)?,
            cli::TaskAction::Next { task_ref } => commands::simple::next(task_ref)?,
            cli::TaskAction::Switch {
                task_ref,
                project,
                force,
                skip_regen_project_files,
            } => commands::switch::run(&task_ref, project, force, skip_regen_project_files)?,
            cli::TaskAction::Merge {
                task_ref,
                strategy,
                plugin,
                all,
                force,
                yes,
                dry_run,
            } => commands::merge::run(&task_ref, &strategy, plugin, all, force, yes, dry_run)?,
            cli::TaskAction::Finish {
                task_ref,
                strategy,
                all,
                plugin,
                yes,
            } => commands::finish::run(task_ref, strategy, all, plugin, yes)?,
            cli::TaskAction::Cleanup {
                task_ref,
                force,
                yes,
            } => commands::cleanup::run(&task_ref, force, yes)?,
            cli::TaskAction::Delete {
                task_ref,
                force,
                yes,
                dry_run,
            } => commands::delete::run(&task_ref, force, yes, dry_run)?,
        },
        Commands::Build {
            task_ref,
            background,
            mutex,
            validator,
            primary_only,
            profile,
            build_log_dir,
        } => commands::build::run(
            &task_ref,
            background,
            mutex,
            validator,
            primary_only,
            profile,
            build_log_dir,
        )?,
        Commands::BuildStatus { task_ref } => commands::build_status::run(&task_ref)?,
        Commands::BuildCheck {
            task_ref,
            workspace,
            profile,
            target,
            build_command,
        } => commands::build_policy::check(task_ref, workspace, profile, target, build_command)?,
        Commands::BuildGate { command } => commands::build_policy::gate(command)?,
        Commands::BuildProject {
            workspace,
            profile,
            target,
        } => commands::build_policy::project(workspace, profile, target)?,
        Commands::Skills { action } => match action {
            cli::SkillsAction::Install { global, project } => {
                commands::skills::install(global, project)?
            }
            cli::SkillsAction::List { project } => commands::skills::list(project)?,
            cli::SkillsAction::Remove { global, project } => {
                commands::skills::remove(global, project)?
            }
        },
    }

    Ok(())
}
