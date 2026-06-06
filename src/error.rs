//! Error types for UnrealDevFlow

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UdfError {
    #[error("UnrealDevFlow not configured. Run `unrealdevflow configure` first.")]
    NotConfigured,

    #[error("Config file not found: {0}")]
    ConfigNotFound(PathBuf),

    #[error("Invalid config: {0}")]
    InvalidConfig(String),

    #[error("Junction error: {0}")]
    Junction(#[from] JunctionError),

    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("Host error: {0}")]
    Host(#[from] HostError),

    #[error("Build error: {0}")]
    Build(#[from] BuildError),

    #[error("Editor is running for project: {0}")]
    EditorRunning(PathBuf),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task already exists: {0}")]
    TaskAlreadyExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum JunctionError {
    #[error("Junction does not exist: {0}")]
    NotExists(PathBuf),

    #[error("Junction target does not exist: {0}")]
    TargetNotFound(PathBuf),

    #[error("Path is not a junction: {0}")]
    NotAJunction(PathBuf),

    #[error("Failed to create junction: {0}")]
    CreateFailed(String),

    #[error("Failed to delete junction: {0}")]
    DeleteFailed(String),

    #[error("Junction crate error: {0}")]
    JunctionCrate(String),
}

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotARepo(PathBuf),

    #[error("Git2 error: {0}")]
    Git2(#[from] git2::Error),

    #[error("Worktree error: {0}")]
    Worktree(String),

    #[error("Branch error: {0}")]
    Branch(String),

    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    #[error("Git command failed: {0}")]
    CommandFailed(String),
}

impl From<git2::Error> for UdfError {
    fn from(err: git2::Error) -> Self {
        UdfError::Git(GitError::Git2(err))
    }
}

#[derive(Error, Debug)]
pub enum HostError {
    #[error("Host directory does not exist: {0}")]
    NotExists(PathBuf),

    #[error("Host already exists: {0}")]
    AlreadyExists(PathBuf),

    #[error("Invalid .uproject: {0}")]
    InvalidUproject(String),

    #[error("Invalid .udf-meta.json: {0}")]
    InvalidMeta(String),
}

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Engine not found: {0}")]
    EngineNotFound(String),

    #[error("Build.bat not found: {0}")]
    BuildBatNotFound(PathBuf),

    #[error("Build failed with exit code: {0}")]
    BuildFailed(i32),

    #[error("DLL not produced: {0}")]
    DllNotProduced(PathBuf),

    #[error("DLL is stale (older than source): {0}")]
    DllStale(PathBuf),
}

pub type Result<T> = std::result::Result<T, UdfError>;
