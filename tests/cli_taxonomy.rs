mod build_profile {
    #[derive(Clone, Copy, Debug, clap::ValueEnum, PartialEq)]
    pub enum MutexMode {
        Auto,
        Wait,
        NoMutex,
    }

    #[derive(Clone, Copy, Debug, clap::ValueEnum, PartialEq)]
    pub enum BuildProfile {
        Light,
        Medium,
        Heavy,
    }
}

#[path = "../src/cli.rs"]
mod cli;

use clap::{CommandFactory, Parser};
use cli::{
    BuildAction, Cli, Commands, PackageAction, PackageAdvancedAction, RunAction, WorkspaceAction,
};

fn help_for(path: &[&str]) -> String {
    let mut command = Cli::command();
    for name in path {
        command = command
            .find_subcommand_mut(name)
            .unwrap_or_else(|| panic!("missing subcommand {name}"))
            .clone();
    }
    command.render_long_help().to_string()
}

fn subcommand_about(help: &str, name: &str) -> String {
    help.lines()
        .find_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix(name)
                .map(|rest| rest.trim_start().to_string())
        })
        .unwrap_or_else(|| panic!("missing help line for {name}\n{help}"))
}

#[test]
fn top_level_contains_package() {
    let parsed = Cli::parse_from(["udf", "package", "status"]);
    assert!(matches!(
        parsed.command,
        Commands::Package {
            action: PackageAction::Status { .. }
        }
    ));

    let help = help_for(&[]);
    assert!(help.contains("package"));
    assert!(help.contains("生成可保存、传递或发布的制品"));
}

#[test]
fn top_level_contains_native_run_and_requires_an_explicit_scope() {
    let parsed = Cli::parse_from(["udf", "run", "plan", "editor", "--workspace", "neon-dev"]);
    assert!(matches!(
        parsed.command,
        Commands::Run {
            action: RunAction::Plan { .. }
        }
    ));
    let help = help_for(&[]);
    assert!(help.contains("run"));
    assert!(help_for(&["run"]).contains("原生 Editor"));
    assert!(Cli::try_parse_from(["udf", "run", "list"]).is_err());
}

#[test]
fn package_contains_configure() {
    let parsed = Cli::parse_from(["udf", "package", "configure", "--task", "test/task"]);
    assert!(matches!(
        parsed.command,
        Commands::Package {
            action: PackageAction::Configure { .. }
        }
    ));
}

#[test]
fn package_help_describes_cook_modes() {
    let configure_help = help_for(&["package", "configure"]);
    assert!(configure_help.contains("--cook-mode <MODE>"));
    assert!(configure_help.contains("日常开发"));
    assert!(configure_help.contains("完整发布"));

    let project_help = help_for(&["package", "project"]);
    assert!(project_help.contains("iterate"));
    assert!(project_help.contains("configure --cook-mode full"));
}

#[test]
fn package_contains_recover() {
    let parsed = Cli::parse_from(["udf", "package", "recover", "package-project-test"]);
    assert!(matches!(
        parsed.command,
        Commands::Package {
            action: PackageAction::Recover { .. }
        }
    ));
}

#[test]
fn package_separates_normal_and_advanced_targets() {
    let parsed = Cli::parse_from(["udf", "package", "advanced", "plugin", "AesWorld", "--plan"]);
    assert!(matches!(
        parsed.command,
        Commands::Package {
            action: PackageAction::Advanced {
                action: PackageAdvancedAction::Plugin { plan: true, .. }
            }
        }
    ));
    let help = help_for(&["package"]);
    assert!(help.contains("advanced"));
    assert!(!help.contains("package plugin"));
    assert!(!help.contains("package engine"));
}

#[test]
fn legacy_query_invocations_have_explicit_compatibility() {
    let parsed = Cli::parse_from(["udf", "package", "plugin", "AesWorld", "--task", "neon/fix"]);
    assert!(matches!(
        parsed.command,
        Commands::Package {
            action: PackageAction::Plugin {
                plugins,
                task,
                workspace: None,
                ..
            }
        } if plugins == vec!["AesWorld".to_string()] && task.as_deref() == Some("neon/fix")
    ));

    for args in [
        ["udf", "package", "project"].as_slice(),
        ["udf", "package", "engine"].as_slice(),
        ["udf", "package", "check"].as_slice(),
        ["udf", "package", "plan"].as_slice(),
        ["udf", "package", "status"].as_slice(),
        ["udf", "package", "clean", "package-project-1"].as_slice(),
    ] {
        Cli::try_parse_from(args).unwrap_or_else(|error| panic!("{args:?} failed: {error}"));
    }
}

#[test]
fn removed_package_run_entrypoint_cannot_be_selected() {
    assert!(Cli::try_parse_from(["udf", "package", "run"]).is_err());
    assert!(!help_for(&["package"]).contains("package run"));
}

#[test]
fn non_execution_queries_follow_the_shared_vocabulary() {
    let build_engine = Cli::parse_from(["udf", "build", "engine", "--workspace", "neon-dev"]);
    assert!(matches!(
        build_engine.command,
        Commands::Build {
            action: BuildAction::Engine { workspace, .. }
        } if workspace.as_deref() == Some("neon-dev")
    ));

    let build_plan = Cli::parse_from(["udf", "build", "plan", "neon-dev/task-a"]);
    assert!(matches!(
        build_plan.command,
        Commands::Build {
            action: BuildAction::Plan { task_ref, .. }
        } if task_ref.as_deref() == Some("neon-dev/task-a")
    ));

    let workspace_inspect = Cli::parse_from(["udf", "workspace", "inspect", "neon-dev"]);
    assert!(matches!(
        workspace_inspect.command,
        Commands::Workspace {
            action: WorkspaceAction::Inspect { name, .. }
        } if name.as_deref() == Some("neon-dev")
    ));
}

#[test]
fn same_named_secondary_commands_share_the_same_help_text() {
    let build_help = help_for(&["build"]);
    let package_help = help_for(&["package"]);

    assert_eq!(
        subcommand_about(&build_help, "check"),
        subcommand_about(&package_help, "check")
    );
    assert_eq!(
        subcommand_about(&build_help, "plan"),
        subcommand_about(&package_help, "plan")
    );
    assert_eq!(
        subcommand_about(&build_help, "status"),
        subcommand_about(&package_help, "status")
    );
}
