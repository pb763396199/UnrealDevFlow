//! Friendly finish wrapper around merge.

use crate::cli::MergeStrategy;
use crate::config::Config;
use crate::error::{Result, UdfError};

pub fn run(
    task_ref: Option<String>,
    strategy: Option<MergeStrategy>,
    all: bool,
    plugin: Option<String>,
    skip_confirm: bool,
) -> Result<()> {
    let config = Config::load()?;
    let task_ref = match task_ref {
        Some(value) => value,
        None => crate::commands::simple::latest_task_ref(&config)?,
    };
    let strategy = match strategy {
        Some(value) => value,
        None => ask_strategy()?,
    };
    crate::commands::merge::run(
        &task_ref,
        &strategy,
        plugin,
        all,
        false,
        skip_confirm,
        false,
    )
}

fn ask_strategy() -> Result<MergeStrategy> {
    let items = [
        "rebase（推荐，线性历史）",
        "merge（保留任务历史）",
        "squash（压缩成一个提交）",
        "ff-only（仅快进）",
    ];
    let selected = dialoguer::Select::new()
        .with_prompt("请选择合并策略")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
    Ok(match selected {
        0 => MergeStrategy::Rebase,
        1 => MergeStrategy::Merge,
        2 => MergeStrategy::Squash,
        _ => MergeStrategy::FfOnly,
    })
}
