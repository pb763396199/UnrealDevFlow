//! UnrealDevFlow - Unreal Engine plugin parallel development workflow tool

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

    if !matches!(cli.command, Commands::Configure { .. }) && !config::Config::exists() {
        return Err(error::UdfError::NotConfigured);
    }

    match cli.command {
        Commands::Configure {
            hosts_root,
            plugin_path,
            plugins_root,
            default_project,
            engine_path,
        } => commands::configure::run(hosts_root, plugin_path, plugins_root, default_project, engine_path)?,
        Commands::Create {
            description,
            id,
            prompt,
            primary,
            override_dep,
            yes,
        } => commands::create::run(&description, id, prompt, primary, override_dep, yes)?,
        Commands::Switch { task_id, project, force } => {
            commands::switch::run(&task_id, project, force)?
        }
        Commands::Build {
            task_id,
            background,
            no_mutex,
            safe,
            primary_only,
        } => commands::build::run(&task_id, background, no_mutex, safe, primary_only)?,
        Commands::BuildStatus { task_id } => commands::build_status::run(&task_id)?,
        Commands::List => commands::list::run(&cli.format)?,
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
    }

    Ok(())
}
