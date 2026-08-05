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

use clap::FromArgMatches;
use cli::{Cli, Commands};
use error::Result;
use tracing_subscriber::EnvFilter;

fn main() {
    // 帮助文本要全中文，clap 的段落标题和自动生成的 `help` 子命令是英文硬编码，
    // derive 宏够不到。build() 先把自动子命令落成真实节点，否则 mut_subcommand
    // 找不到它会 panic。
    let mut command = <Cli as clap::CommandFactory>::command();
    command.build();
    let command = localize(command);
    let cli = match Cli::from_arg_matches(&command.get_matches()) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };
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

/// 把中文骨架套到整棵命令树上。
///
/// derive 宏上的 `help_template` / `next_help_heading` 只作用于它标注的那一层，
/// 22 个叶子逐个标一遍既啰嗦又容易漏。段落标题是挂在每个参数上的，所以已经建好
/// 的命令要靠 `mut_args` 回头改。位置参数和选项分开归类，否则会混成一段。
fn localize(command: clap::Command) -> clap::Command {
    let command = command
        .help_template(cli::HELP_TEMPLATE)
        .subcommand_help_heading("命令")
        .mut_args(|arg| {
            let heading = if arg.is_positional() {
                "参数"
            } else {
                "选项"
            };
            arg.help_heading(heading)
        });
    let names: Vec<String> = command
        .get_subcommands()
        .map(|sub| sub.get_name().to_string())
        .collect();
    names.into_iter().fold(command, |command, name| {
        if name == "help" {
            // 每一层都有一个自动生成的 help，说明文字都要换。
            command.mut_subcommand(name, |sub| sub.about("显示某条命令的帮助"))
        } else {
            command.mut_subcommand(name, localize)
        }
    })
}

/// Name reported in the JSON envelope, so callers can tell which command spoke.
///
/// Resolves down to the leaf: a failure in `task switch` must not come back
/// labelled `task`, or the caller cannot tell which step of a group broke.
fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::AwStatus => "aw-status",
        Commands::Workspace { action } => match action {
            cli::WorkspaceAction::Init { .. } => "workspace init",
            cli::WorkspaceAction::Add { .. } => "workspace add",
            cli::WorkspaceAction::List => "workspace list",
            cli::WorkspaceAction::Doctor { .. } => "workspace doctor",
            cli::WorkspaceAction::Remove { .. } => "workspace remove",
            cli::WorkspaceAction::Status => "workspace status",
        },
        Commands::Task { action } => match action {
            cli::TaskAction::Create { .. } => "task create",
            cli::TaskAction::List { .. } => "task list",
            cli::TaskAction::Next { .. } => "task next",
            cli::TaskAction::Switch { .. } => "task switch",
            cli::TaskAction::Merge { .. } => "task merge",
            cli::TaskAction::Finish { .. } => "task finish",
            cli::TaskAction::Cleanup { .. } => "task cleanup",
            cli::TaskAction::Delete { .. } => "task delete",
        },
        Commands::Build { action } => match action {
            cli::BuildAction::Task { .. } => "build task",
            cli::BuildAction::Project { .. } => "build project",
            cli::BuildAction::Check { .. } => "build check",
            cli::BuildAction::Gate { .. } => "build gate",
            cli::BuildAction::Status { .. } => "build status",
        },
        Commands::Skill { action } => match action {
            cli::SkillAction::Install { .. } => "skill install",
            cli::SkillAction::List { .. } => "skill list",
            cli::SkillAction::Remove { .. } => "skill remove",
        },
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

    // `build gate` only inspects a command string, so it has to keep working on
    // a machine that has never been configured — that is where hooks run it.
    // The rest of the build group genuinely needs a workspace.
    let needs_config = !matches!(
        cli.command,
        Commands::AwStatus
            | Commands::Workspace { .. }
            | Commands::Skill { .. }
            | Commands::Build {
                action: cli::BuildAction::Gate { .. }
            }
    );
    if needs_config && !config::Config::exists() {
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
                change_type,
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
                change_type,
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
        Commands::Build { action } => match action {
            cli::BuildAction::Task {
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
            cli::BuildAction::Project {
                workspace,
                profile,
                target,
            } => commands::build_policy::project(workspace, profile, target)?,
            cli::BuildAction::Check {
                task_ref,
                workspace,
                profile,
                target,
                build_command,
            } => {
                commands::build_policy::check(task_ref, workspace, profile, target, build_command)?
            }
            cli::BuildAction::Gate { command } => commands::build_policy::gate(command)?,
            cli::BuildAction::Status { task_ref } => commands::build_status::run(task_ref)?,
        },
        Commands::Skill { action } => match action {
            cli::SkillAction::Install { global, project } => {
                commands::skills::install(global, project)?
            }
            cli::SkillAction::List { project } => commands::skills::list(project)?,
            cli::SkillAction::Remove { global, project } => {
                commands::skills::remove(global, project)?
            }
        },
    }

    Ok(())
}
