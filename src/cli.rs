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

#[derive(Subcommand)]
pub enum Commands {
    /// First-time configuration (Hosts path, plugin path, engine path)
    Configure,

    /// Create a new task workspace (worktree + Host)
    Create {
        /// Task description (used to suggest task ID)
        description: String,

        /// Custom task ID (overrides auto-suggestion)
        #[arg(long)]
        id: Option<String>,
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
    },

    /// List all tasks
    List,

    /// Show current Junction status and active task
    Status,

    /// Merge a task into main repo and clean up
    Merge {
        /// Task ID to merge
        task_id: String,

        /// Force merge even if there are conflicts
        #[arg(long)]
        force: bool,
    },

    /// Delete a task without merging
    Delete {
        /// Task ID to delete
        task_id: String,

        /// Force delete without confirmation
        #[arg(long)]
        force: bool,
    },
}
