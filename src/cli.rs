//! CLI command definitions for UnrealDevFlow

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "unrealdevflow")]
#[command(about = "Unreal Engine plugin parallel development workflow tool")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format (json or human)
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
    /// First-time configuration (Hosts path, plugins root, engine path)
    Configure {
        /// Hosts root directory (skip prompt if provided)
        #[arg(long)]
        hosts_root: Option<String>,

        /// v1 legacy single-plugin repository path (use --plugins-root in v2)
        #[arg(long)]
        plugin_path: Option<String>,

        /// v2 plugins root directory (folder containing all project plugin Git repos)
        #[arg(long)]
        plugins_root: Option<String>,

        /// Default UE project path (skip prompt if provided)
        #[arg(long)]
        default_project: Option<String>,

        /// UE engine path (auto-detected from .uproject if not provided)
        #[arg(long)]
        engine_path: Option<String>,
    },

    /// Create a new task workspace (worktree + Host)
    Create {
        /// Task description (used to suggest task ID)
        description: String,

        /// Custom task ID (overrides auto-suggestion)
        #[arg(long)]
        id: Option<String>,

        /// Original prompt/task description to save in metadata
        #[arg(long)]
        prompt: Option<String>,

        /// v2: primary plugin names (comma-separated). If omitted, falls back
        /// to the legacy `plugin_path` configured plugin.
        #[arg(long, value_delimiter = ',')]
        primary: Option<Vec<String>>,

        /// v2: dependency override mappings as `name=engine` / `name=project` /
        /// `name=<absolute-path>`. Repeatable.
        #[arg(long = "override-dep", value_parser = parse_dep_override)]
        override_dep: Vec<DepOverride>,

        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Switch Junction to a specific task (affects next Editor launch)
    Switch {
        /// Task ID to switch to (use "main" to switch back to main repo)
        task_id: String,

        /// UE project path(s) to update (comma-separated for multiple)
        #[arg(long, value_delimiter = ',')]
        project: Option<Vec<PathBuf>>,

        /// Force switch even if Editor is running
        #[arg(long)]
        force: bool,
    },

    /// Build a task's Host project
    Build {
        /// Task ID to build
        task_id: String,

        /// Build in background
        #[arg(long)]
        background: bool,

        /// Mutex mode: auto (default) / wait / nomutex
        ///
        /// auto  : -WaitMutex if engine Intermediate/Build/Shared is missing,
        ///         -NoMutex once shared PCH is ready. Validator hint biases
        ///         to -NoMutex.
        /// wait  : always -WaitMutex (queue if another build is running)
        /// nomutex: always -NoMutex (parallel; fastest when PCH cached)
        #[arg(long, value_enum, default_value = "auto", alias = "safe", alias = "no-mutex")]
        mutex: crate::build_profile::MutexMode,

        /// Hint: build is invoked by a validator / CI (not an IDE).
        /// When `--mutex auto`, prefers -NoMutex so the validator can
        /// detect contention instead of queueing.
        #[arg(long)]
        validator: bool,

        /// v2: only compile primary plugin modules (passes `-Module=` per primary)
        #[arg(long)]
        primary_only: bool,

        /// Build strictness profile. Default: light.
        ///   light  : -FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity
        ///   medium : light + -WarningsAsErrors
        ///   heavy  : -ForceHeaderGeneration -Rebuild -DisableUnity -NoSharedPCH -WarningsAsErrors
        ///           (still keeps -FailIfGeneratedCodeChanges)
        #[arg(long, value_enum, default_value = "light")]
        profile: crate::build_profile::BuildProfile,

        /// Override the UBT log directory. Default: <host>/Logs/UBT/
        #[arg(long)]
        build_log_dir: Option<std::path::PathBuf>,
    },

    /// Check build status of a task
    BuildStatus {
        /// Task ID to check
        task_id: String,
    },

    /// List all tasks
    List,

    /// Show current Junction status and active task
    Status,

    /// Merge a task into main repo
    ///
    /// ⚠️ THIS COMMAND ONLY MERGES - IT NEVER DELETES ANYTHING
    /// After merge, you MUST manually run `cleanup <task-id>` to remove worktree/branch
    ///
    /// v2: tasks may have multiple primary plugins. Use --plugin to merge a
    /// specific plugin, or --all to iterate in reverse order with independent
    /// confirmations per plugin.
    ///
    /// Commit message format (Chinese required):
    ///   Task#[number] [content]
    ///
    ///   修改内容：
    ///   - [change 1]
    ///
    ///   过程反思：
    ///   - [lesson learned]
    ///
    ///   后续注意：
    ///   - [avoid in future]
    Merge {
        /// Task ID to merge
        task_id: String,

        /// Merge strategy (required - agent must ask user)
        /// Options: rebase, merge, squash, ff-only
        #[arg(long, value_enum)]
        strategy: MergeStrategy,

        /// v2: target a specific primary plugin (required when task has >1
        /// primary plugin unless --all is set)
        #[arg(long)]
        plugin: Option<String>,

        /// v2: merge every primary plugin in reverse declaration order, with
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

    /// Cleanup worktree and branch after merge verification
    Cleanup {
        /// Task ID to cleanup
        task_id: String,

        /// Force cleanup without confirmation
        #[arg(long)]
        force: bool,

        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Delete a task without merging
    Delete {
        /// Task ID to delete
        task_id: String,

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
