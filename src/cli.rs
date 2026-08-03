//! CLI command definitions for UnrealDevFlow

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "udf")]
#[command(about = "Unreal Engine plugin parallel development workflow tool")]
#[command(version = env!("UDF_VERSION_LONG"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format. Honored by `list`, `status`, `build-check`, `build-gate`
    /// and `build-project`; other commands always print human-readable text.
    #[arg(long, global = true, default_value = "human")]
    pub format: OutputFormat,

    /// Enable verbose logging
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Human,
}

#[derive(Clone, Debug, clap::ValueEnum, PartialEq)]
pub enum MergeStrategy {
    /// Rebase task branch onto dev (default preference)
    Rebase,
    /// Create merge commit, preserve task history
    Merge,
    /// Squash all task commits into one
    Squash,
    /// Fast-forward only, fail if not possible
    FfOnly,
}

#[derive(Subcommand)]
pub enum Commands {
    /// AgentWatcher read-only module status probe.
    #[command(name = "aw-status", hide = true)]
    AwStatus,

    /// Set up and inspect the UE project environments you work in.
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    /// Create, inspect and retire isolated development tasks.
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Compile a task Host or the main project, and decide when that is allowed.
    Build {
        #[command(subcommand)]
        action: BuildAction,
    },

    /// Install/inspect/remove the UnrealDevFlow skill for AI agent providers
    /// (opencode / copilot / codex / claude code).
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
}

#[derive(Subcommand)]
pub enum BuildAction {
    /// Build a task's Host project
    Task {
        /// Task ref, e.g. workspace/task-id.
        task_ref: String,

        /// Build in background
        #[arg(long)]
        background: bool,

        /// Mutex mode: auto (default) / wait / no-mutex
        ///
        /// auto    : -WaitMutex if engine Intermediate/Build/Shared is missing,
        ///           -NoMutex once shared PCH is ready. Validator hint biases
        ///           to -NoMutex.
        /// wait    : always -WaitMutex (queue if another build is running)
        /// no-mutex: always -NoMutex (parallel; fastest when PCH cached)
        #[arg(long, value_enum, default_value = "auto")]
        mutex: crate::build_profile::MutexMode,

        /// Hint: build is invoked by a validator / CI (not an IDE).
        #[arg(long)]
        validator: bool,

        /// Only compile primary plugin modules (passes `-Module=` per primary)
        #[arg(long)]
        primary_only: bool,

        /// Build strictness profile. Default: light.
        #[arg(long, value_enum, default_value = "light")]
        profile: crate::build_profile::BuildProfile,

        /// Override the UBT log directory. Default: <host>/Logs/UBT/
        #[arg(long)]
        build_log_dir: Option<PathBuf>,
    },

    /// Build the workspace's main UE project instead of a task Host
    Project {
        /// Workspace name. Required when more than one workspace exists.
        #[arg(long)]
        workspace: Option<String>,

        /// Build strictness profile. Default: light.
        #[arg(long, value_enum, default_value = "light")]
        profile: crate::build_profile::BuildProfile,

        /// Editor target override. Default: derived from the .uproject.
        #[arg(long)]
        target: Option<String>,
    },

    /// Report whether a controlled UE build may start right now (never builds)
    ///
    /// Prints one of: ready / needsUserInput / blocked / deferred.
    Check {
        /// Task ref, e.g. workspace/task-id. If omitted, checks the workspace's
        /// main project instead of a task Host.
        task_ref: Option<String>,

        /// Workspace name. Used when no task ref is given.
        #[arg(long)]
        workspace: Option<String>,

        /// Build strictness profile. Default: light.
        #[arg(long, value_enum, default_value = "light")]
        profile: crate::build_profile::BuildProfile,

        /// Editor target override. Default: derived from the .uproject.
        #[arg(long)]
        target: Option<String>,

        /// A build command to validate against the resolved project before it runs.
        #[arg(long)]
        build_command: Option<String>,
    },

    /// Check whether a proposed command bypasses the controlled build path
    ///
    /// Exits non-zero when the command is refused.
    Gate {
        /// The command a tool or agent proposes to run.
        command: String,
    },

    /// Check build status of a task
    Status {
        /// Task ref, e.g. workspace/task-id.
        task_ref: String,
    },
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// Create an isolated task: one git worktree per primary plugin, plus a Host project.
    Create {
        /// Task description. Also saved as the original prompt unless --prompt is given.
        description: String,

        /// Custom task ID (overrides auto-suggestion)
        #[arg(long)]
        id: Option<String>,

        /// Explicit task branch name used in every primary repository.
        /// Defaults to task/<workspace>/<task-id>.
        #[arg(long)]
        branch: Option<String>,

        /// Git ref used as the base in every primary repository.
        #[arg(long)]
        base_ref: Option<String>,

        /// Original prompt to save in metadata. Defaults to the description.
        #[arg(long)]
        prompt: Option<String>,

        /// Workspace name. Required when more than one workspace exists.
        #[arg(long)]
        workspace: Option<String>,

        /// Primary plugin names (comma-separated). If omitted, falls back to the
        /// legacy `plugin_path` configured plugin.
        #[arg(long, value_delimiter = ',')]
        primary: Option<Vec<String>>,

        /// Dependency override mappings as `name=engine` / `name=project` /
        /// `name=<absolute-path>`. Repeatable.
        #[arg(long = "override-dep", value_parser = parse_dep_override)]
        override_dep: Vec<DepOverride>,

        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// List all tasks
    List {
        /// Workspace name to filter tasks
        #[arg(long)]
        workspace: Option<String>,
    },

    /// Tell the user the single next step for a task.
    Next {
        /// Task ref, e.g. workspace/task-id. If omitted, uses latest task when unambiguous.
        task_ref: Option<String>,
    },

    /// Point a UE project's Junctions at this task (affects next Editor launch)
    Switch {
        /// Task ref, e.g. workspace/task-id. Use "main" to switch back to the main repos.
        task_ref: String,

        /// UE project path(s) to update (comma-separated for multiple).
        ///
        /// Only "main" may target several projects. A task is bound to the
        /// project frozen in its metadata, so naming any other project is
        /// refused before any Junction changes.
        #[arg(long, value_delimiter = ',')]
        project: Option<Vec<PathBuf>>,

        /// Force switch even if Editor is running
        #[arg(long)]
        force: bool,

        /// Skip the automatic "Generate Visual Studio project files" step.
        #[arg(long)]
        skip_regen_project_files: bool,
    },

    /// Merge a task into its primary repositories
    ///
    /// ⚠️ THIS COMMAND ONLY MERGES - IT NEVER DELETES ANYTHING
    /// After merge, you MUST run `task cleanup` to remove worktree/branch.
    ///
    /// Tasks may have multiple primary plugins. Use --plugin to merge a
    /// specific plugin, or --all to iterate in reverse order with independent
    /// confirmations per plugin.
    Merge {
        /// Task ref, e.g. workspace/task-id.
        task_ref: String,

        /// Merge strategy (required - agent must ask user)
        /// Options: rebase, merge, squash, ff-only
        #[arg(long, value_enum)]
        strategy: MergeStrategy,

        /// Target a specific primary plugin (required when task has >1 primary
        /// plugin unless --all is set)
        #[arg(long)]
        plugin: Option<String>,

        /// Merge every primary plugin in reverse declaration order, with
        /// independent confirmation per plugin
        #[arg(long)]
        all: bool,

        /// Force merge even if there are conflicts
        #[arg(long)]
        force: bool,

        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,

        /// Show what would be merged without actually merging
        #[arg(long)]
        dry_run: bool,
    },

    /// Finish a task by running the merge guide.
    Finish {
        /// Task ref, e.g. workspace/task-id. If omitted, uses latest task when unambiguous.
        task_ref: Option<String>,

        /// Merge strategy. If omitted, asks interactively.
        #[arg(long, value_enum)]
        strategy: Option<MergeStrategy>,

        /// Merge all primary plugins.
        #[arg(long)]
        all: bool,

        /// Target primary plugin.
        #[arg(long)]
        plugin: Option<String>,

        /// Skip confirmation prompts after strategy is known.
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Cleanup worktree and branch after merge verification
    Cleanup {
        /// Task ref, e.g. workspace/task-id.
        task_ref: String,

        /// Force cleanup without confirmation
        #[arg(long)]
        force: bool,

        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Delete a task without merging
    Delete {
        /// Task ref, e.g. workspace/task-id.
        task_ref: String,

        /// Force delete without confirmation
        #[arg(long)]
        force: bool,

        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,

        /// Show what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum WorkspaceAction {
    /// First-run setup: detect and register a workspace, then install the AI skill.
    Init {
        /// Workspace name. If omitted, one is suggested from the project path.
        #[arg(long)]
        workspace: Option<String>,

        /// UE project directory containing a .uproject file.
        #[arg(long)]
        project: Option<PathBuf>,

        /// Project plugins root. If omitted, inferred from project siblings.
        #[arg(long)]
        plugins_root: Option<PathBuf>,

        /// Hosts root. If omitted, defaults to <project-parent>/Hosts.
        #[arg(long)]
        hosts_root: Option<PathBuf>,

        /// UE engine path. If omitted, inferred from .uproject EngineAssociation.
        #[arg(long)]
        engine_path: Option<PathBuf>,

        /// Skip confirmation prompts.
        #[arg(long, short = 'y')]
        yes: bool,

        /// Do not install the AI skill during init.
        #[arg(long)]
        skip_skill_install: bool,
    },

    /// Add or update a named UE project workspace.
    Add {
        name: String,
        /// UE project directory containing a .uproject.
        #[arg(long)]
        project: PathBuf,
        /// Hosts root directory.
        #[arg(long)]
        hosts_root: PathBuf,
        /// Project plugins root.
        #[arg(long)]
        plugins_root: PathBuf,
        /// UE engine path. If omitted, inferred from project.
        #[arg(long)]
        engine_path: Option<PathBuf>,
        /// Legacy default primary plugin path.
        #[arg(long)]
        plugin_path: Option<PathBuf>,
        /// Skip confirmation prompts.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// List registered workspaces.
    List,
    /// Check whether a workspace points at valid directories.
    Doctor {
        name: String,
        /// Recursively validate every plugin source under plugins_root.
        #[arg(long)]
        deep: bool,
    },
    /// Remove a workspace registration.
    Remove {
        name: String,
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Show which task each UE project currently points at.
    Status,
}

#[derive(Subcommand)]
pub enum SkillAction {
    /// Copy skills/<name>/SKILL.md into .codex/ + .agents/ + .claude/ + opencode skill dirs
    Install {
        /// Install to user home (~/.codex/ + ~/.agents/ + ~/.claude/ + opencode) instead of project
        #[arg(long, short = 'g')]
        global: bool,
        /// Project root (default: current dir)
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Show which providers have the skill installed (project + global)
    List {
        /// Project root (default: current dir)
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Remove installed skills
    Remove {
        #[arg(long, short = 'g')]
        global: bool,
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum DepOverrideKind {
    Engine,
    Project,
    CustomPath(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DepOverride {
    pub name: String,
    pub kind: DepOverrideKind,
}

fn parse_dep_override(raw: &str) -> std::result::Result<DepOverride, String> {
    let (name, value) = raw
        .split_once('=')
        .ok_or_else(|| format!("invalid --override-dep '{}', expected NAME=VALUE", raw))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(format!("invalid --override-dep '{}': empty name", raw));
    }
    let value = value.trim();
    let kind = match value {
        "engine" => DepOverrideKind::Engine,
        "project" => DepOverrideKind::Project,
        other => DepOverrideKind::CustomPath(PathBuf::from(other)),
    };
    Ok(DepOverride { name, kind })
}
