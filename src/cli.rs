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
    /// First-time configuration (Hosts path, plugin path, engine path)
    Configure {
        /// Hosts root directory (skip prompt if provided)
        #[arg(long)]
        hosts_root: Option<String>,

        /// Plugin main repository path (skip prompt if provided)
        #[arg(long)]
        plugin_path: Option<String>,

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

        /// Skip mutex (for parallel builds)
        #[arg(long)]
        no_mutex: bool,

        /// Force WaitMutex (safe mode for engine intermediate conflicts)
        #[arg(long)]
        safe: bool,
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
