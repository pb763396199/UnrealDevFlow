//! Error types for UnrealDevFlow

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UdfError {
    #[error("UnrealDevFlow 未配置。请先运行 `unrealdevflow configure`")]
    NotConfigured,

    #[error("配置文件不存在：{0}")]
    ConfigNotFound(PathBuf),

    #[error("配置无效：{0}")]
    InvalidConfig(String),

    #[error("Junction 错误：{0}")]
    Junction(#[from] JunctionError),

    #[error("Git 错误：{0}")]
    Git(#[from] GitError),

    #[error("Host 错误：{0}")]
    Host(#[from] HostError),

    #[error("编译错误：{0}")]
    Build(#[from] BuildError),

    #[error("编辑器正在运行：{0}")]
    EditorRunning(PathBuf),

    #[error("任务不存在：{0}")]
    TaskNotFound(String),

    #[error("任务已存在：{0}")]
    TaskAlreadyExists(String),

    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误：{0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML 错误：{0}")]
    Toml(#[from] toml::de::Error),

    #[error("{0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum JunctionError {
    #[error("Junction 不存在：{0}")]
    NotExists(PathBuf),

    #[error("Junction 目标不存在：{0}")]
    TargetNotFound(PathBuf),

    #[error("路径不是 Junction：{0}")]
    NotAJunction(PathBuf),

    #[error("创建 Junction 失败：{0}")]
    CreateFailed(String),

    #[error("删除 Junction 失败：{0}")]
    DeleteFailed(String),

    #[error("Junction crate 错误：{0}")]
    JunctionCrate(String),
}

#[derive(Error, Debug)]
pub enum GitError {
    #[error("不是 Git 仓库：{0}")]
    NotARepo(PathBuf),

    #[error("Git2 错误：{0}")]
    Git2(#[from] git2::Error),

    #[error("Worktree 错误：{0}")]
    Worktree(String),

    #[error("分支错误：{0}")]
    Branch(String),

    #[error("合并冲突：{0}")]
    MergeConflict(String),

    #[error("Git 命令失败：{0}")]
    CommandFailed(String),
}

impl From<git2::Error> for UdfError {
    fn from(err: git2::Error) -> Self {
        UdfError::Git(GitError::Git2(err))
    }
}

#[derive(Error, Debug)]
pub enum HostError {
    #[error("Host 目录不存在：{0}")]
    NotExists(PathBuf),

    #[error("Host 已存在：{0}")]
    AlreadyExists(PathBuf),

    #[error(".uproject 无效：{0}")]
    InvalidUproject(String),

    #[error(".udf-meta.json 无效：{0}")]
    InvalidMeta(String),
}

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("引擎未找到：{0}")]
    EngineNotFound(String),

    #[error("Build.bat 不存在：{0}")]
    BuildBatNotFound(PathBuf),

    #[error("编译失败，退出码：{0}")]
    BuildFailed(i32),

    #[error("DLL 未生成：{0}")]
    DllNotProduced(PathBuf),

    #[error("DLL 过时（旧于源码）：{0}")]
    DllStale(PathBuf),
}

pub type Result<T> = std::result::Result<T, UdfError>;
