//! UnrealDevFlow - Unreal Engine plugin parallel development workflow tool

// The CLI keeps several recovery/inspection helpers and structured error
// variants that are intentionally not wired into every release yet.
#![allow(dead_code)]

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
    if let Err(e) = run() {
        output::print_error(&format!("{}", e));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

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
            | Commands::Configure { .. }
            | Commands::Init { .. }
            | Commands::Workspace { .. }
            | Commands::Skills { .. }
    ) && !config::Config::exists()
    {
        return Err(error::UdfError::NotConfigured);
    }

    match cli.command {
        Commands::AwStatus => commands::aw_status::run()?,
        Commands::Init {
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
        Commands::Start {
            description,
            workspace,
            id,
            primary,
            override_dep,
            yes,
        } => commands::simple::start(&description, workspace, id, primary, override_dep, yes)?,
        Commands::Next { task_ref } => commands::simple::next(task_ref)?,
        Commands::Finish {
            task_ref,
            strategy,
            all,
            plugin,
            yes,
        } => commands::finish::run(task_ref, strategy, all, plugin, yes)?,
        Commands::Workspace { action } => match action {
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
        },
        Commands::Configure {
            hosts_root,
            plugin_path,
            plugins_root,
            default_project,
            engine_path,
        } => commands::configure::run(
            hosts_root,
            plugin_path,
            plugins_root,
            default_project,
            engine_path,
        )?,
        Commands::Create {
            description,
            id,
            prompt,
            workspace,
            primary,
            override_dep,
            yes,
        } => commands::create::run(
            &description,
            id,
            prompt,
            workspace,
            primary,
            override_dep,
            yes,
        )?,
        Commands::Switch {
            task_id,
            project,
            force,
            skip_regen_project_files,
        } => commands::switch::run(&task_id, project, force, skip_regen_project_files)?,
        Commands::Build {
            task_id,
            background,
            mutex,
            validator,
            primary_only,
            profile,
            build_log_dir,
        } => commands::build::run(
            &task_id,
            background,
            mutex,
            validator,
            primary_only,
            profile,
            build_log_dir,
        )?,
        Commands::BuildStatus { task_id } => commands::build_status::run(&task_id)?,
        Commands::List { workspace } => commands::list::run(&cli.format, workspace)?,
        Commands::Status => commands::status::run(&cli.format)?,
        Commands::Merge {
            task_id,
            strategy,
            plugin,
            all,
            force,
            yes,
            dry_run,
        } => commands::merge::run(&task_id, &strategy, plugin, all, force, yes, dry_run)?,
        Commands::Cleanup {
            task_id,
            force,
            yes,
        } => commands::cleanup::run(&task_id, force, yes)?,
        Commands::Delete {
            task_id,
            force,
            yes,
            dry_run,
        } => commands::delete::run(&task_id, force, yes, dry_run)?,
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
