//! Merge command implementation (v2 multi-plugin)

use crate::config::Config;
use crate::error::{Result, UdfError};
use crate::git;
use crate::host::{self, PrimaryPlugin};
use crate::output;
use std::path::{Path, PathBuf};

fn has_uncommitted_changes(worktree_path: &Path) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()?;

    if !output.status.success() {
        return Err(UdfError::Other(format!(
            "Failed to check git status: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(!stdout.trim().is_empty())
}

fn get_uncommitted_changes(worktree_path: &Path) -> Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()?;

    if !output.status.success() {
        return Err(UdfError::Other(format!(
            "Failed to check git status: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}

fn count_commits_to_merge(repo_path: &Path, branch_name: &str) -> Result<usize> {
    let output = std::process::Command::new("git")
        .args(["log", "--oneline", &format!("HEAD..{}", branch_name)])
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        return Ok(0);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().count())
}

/// `task finish` delegates the whole merge here, so it can label the envelope
/// with the command the caller actually typed. One command, one document.
pub fn run(
    task_id: &str,
    strategy: &crate::cli::MergeStrategy,
    plugin: Option<String>,
    all: bool,
    force: bool,
    skip_confirm: bool,
    dry_run: bool,
) -> Result<()> {
    let outcome = run_inner(task_id, strategy, plugin, all, force, skip_confirm, dry_run)?;
    output::emit("task merge", outcome, render_merge);
    Ok(())
}

pub(crate) fn run_inner(
    task_id: &str,
    strategy: &crate::cli::MergeStrategy,
    plugin: Option<String>,
    all: bool,
    force: bool,
    skip_confirm: bool,
    dry_run: bool,
) -> Result<MergeOutcome> {
    let config = Config::load()?;

    let (host_dir, mut meta, _task_context) = host::resolve_task(&config, task_id)?;
    crate::migration::backfill_source_repo(&mut meta, &config);

    if meta.primary_plugins.is_empty() {
        return Err(UdfError::Other(format!(
            "Task '{}' has no primary plugins to merge",
            task_id
        )));
    }

    // === Select target primary plugins ===
    let targets: Vec<PrimaryPlugin> = if all {
        // Reverse order, per AGENTS.md decision: 逆序逐个 merge
        let mut v = meta.primary_plugins.clone();
        v.reverse();
        v
    } else if let Some(name) = plugin {
        let found = meta
            .primary_plugins
            .iter()
            .find(|p| p.name == name)
            .cloned()
            .ok_or_else(|| {
                UdfError::Other(format!("Plugin '{}' not found in task '{}'", name, task_id))
            })?;
        vec![found]
    } else if meta.primary_plugins.len() == 1 {
        vec![meta.primary_plugins[0].clone()]
    } else {
        let names: Vec<String> = meta
            .primary_plugins
            .iter()
            .map(|p| p.name.clone())
            .collect();
        return Err(UdfError::Other(format!(
            "Task '{}' has {} primary plugins. Use --plugin <name> or --all. Available: {}",
            task_id,
            names.len(),
            names.join(", ")
        )));
    };

    let mut all_ok = true;
    let mut reports: Vec<PluginMergeReport> = Vec::new();
    let total = targets.len();
    for (idx, primary) in targets.iter().enumerate() {
        println!();
        output::print_info(&format!(
            "═══ Merge {}/{}: plugin '{}' ═══",
            idx + 1,
            total,
            primary.name
        ));
        let report = merge_single_plugin(
            task_id,
            meta.task_uid.as_deref(),
            &host_dir,
            primary,
            &meta.branch,
            strategy,
            force,
            skip_confirm,
            dry_run,
        )?;
        let ok = report.ok;
        reports.push(report);
        if !ok {
            all_ok = false;
            output::print_warning(&format!(
                "Plugin '{}' merge step did not complete successfully.",
                primary.name
            ));
            if !force {
                output::print_warning(
                    "Aborting remaining --all merges. Use --force to continue past failures.",
                );
                break;
            }
        }
    }

    println!();
    Ok(MergeOutcome {
        task_ref: task_id.to_string(),
        dry_run,
        all_ok,
        plugins: reports,
        cleanup_command: format!("udf task cleanup {}", task_id),
    })
}

/// Merge never deletes anything, so the result always carries the cleanup step.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MergeOutcome {
    task_ref: String,
    dry_run: bool,
    all_ok: bool,
    plugins: Vec<PluginMergeReport>,
    cleanup_command: String,
}

pub(crate) fn render_merge(data: &MergeOutcome) -> String {
    if data.dry_run {
        return format!(
            "Dry run: task '{}' would merge {} plugin(s). Nothing was changed.",
            data.task_ref,
            data.plugins.len()
        );
    }
    let headline = if data.all_ok {
        format!("✓ Task '{}' merge sequence completed.", data.task_ref)
    } else {
        format!(
            "⚠ Task '{}' merge sequence completed with failures.",
            data.task_ref
        )
    };
    format!(
        "{}\n⚠️  Worktrees and branches RETAINED for inspection. Run cleanup after verification:\n  {}",
        headline, data.cleanup_command
    )
}

/// What one plugin's merge step looked at and what came of it. Under
/// `--dry-run` this is the whole answer, so it has to carry the numbers the
/// human preview prints, not just a success flag.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginMergeReport {
    plugin: String,
    branch: String,
    source_repo: String,
    worktree: String,
    commits_to_merge: usize,
    current_ahead: usize,
    task_ahead: usize,
    uncommitted: usize,
    ok: bool,
}

#[allow(clippy::too_many_arguments)]
fn merge_single_plugin(
    task_id: &str,
    task_uid: Option<&str>,
    host_dir: &Path,
    primary: &PrimaryPlugin,
    task_branch: &str,
    strategy: &crate::cli::MergeStrategy,
    force: bool,
    skip_confirm: bool,
    dry_run: bool,
) -> Result<PluginMergeReport> {
    let (_, task_id_only) = host::parse_task_ref(task_id);
    let legacy_expected = format!("task-{}", task_id_only);
    let namespaced_suffix = format!("/{}", task_id_only);
    // 台账那条是新加的：分支名可以完全不含 task-id，只要 git 里记着它属于这个任务。
    // 原有三条一条不删，老任务照走。
    //
    // 比的是完整的任务引用。用后缀比会让 `ws2/save-bug` 在合并 `ws1/save-bug` 时
    // 也算数——那正是台账存完整引用要防的事。老元数据没有 task_uid，才退回宽松判定。
    let ledger_owner =
        git::branch_ledger::task_of(&primary.source_repo, &primary.branch).unwrap_or_default();
    let ledger_says_ours = match (ledger_owner.as_deref(), task_uid) {
        (Some(owner), Some(uid)) => owner == uid,
        (Some(owner), None) => owner == task_id || owner.ends_with(&namespaced_suffix),
        (None, _) => false,
    };
    let branch_matches = primary.branch == task_branch
        || primary.branch == legacy_expected
        || (primary.branch.starts_with("task/") && primary.branch.ends_with(&namespaced_suffix))
        || ledger_says_ours;
    if !branch_matches {
        return Err(UdfError::Other(format!(
            "Branch name '{}' for plugin '{}' does not match expected task id '{}'. Aborting to prevent accidental merge.",
            primary.branch, primary.name, task_id_only
        )));
    }

    let source_repo = if primary.source_repo.as_os_str().is_empty() {
        return Err(UdfError::Other(format!(
            "Plugin '{}' has no source_repo recorded in metadata",
            primary.name
        )));
    } else {
        primary.source_repo.clone()
    };

    let worktree_path = host_dir.join(&primary.worktree);
    let has_uncommitted = if worktree_path.exists() {
        has_uncommitted_changes(&worktree_path)?
    } else {
        false
    };
    let uncommitted_files = if has_uncommitted {
        get_uncommitted_changes(&worktree_path)?
    } else {
        vec![]
    };

    let commit_count = count_commits_to_merge(&source_repo, &primary.branch)?;
    let commits_ahead = git::get_commits_ahead(&source_repo, &primary.branch).unwrap_or_default();
    let commits_behind = git::get_commits_behind(&source_repo, &primary.branch).unwrap_or_default();

    let report = |ok: bool| PluginMergeReport {
        plugin: primary.name.clone(),
        branch: primary.branch.clone(),
        source_repo: source_repo.display().to_string(),
        worktree: worktree_path.display().to_string(),
        commits_to_merge: commit_count,
        current_ahead: commits_ahead.len(),
        task_ahead: commits_behind.len(),
        uncommitted: uncommitted_files.len(),
        ok,
    };

    if dry_run {
        output::print_info(&format!("Dry run: would merge plugin '{}'", primary.name));
        output::print_info(&format!("  Strategy: {:?}", strategy));
        output::print_info(&format!("  Repo: {:?}", source_repo));
        output::print_info(&format!(
            "  Branch: {} ({} commit(s) to merge)",
            primary.branch, commit_count
        ));
        output::print_info(&format!("  Worktree: {:?}", worktree_path));
        if !commits_ahead.is_empty() {
            output::print_warning(&format!(
                "  Current branch is {} commit(s) ahead of task branch.",
                commits_ahead.len()
            ));
        }
        if !commits_behind.is_empty() {
            output::print_warning(&format!(
                "  Task branch is {} commit(s) ahead of current branch.",
                commits_behind.len()
            ));
        }
        if has_uncommitted {
            output::print_warning(&format!(
                "  ⚠ Worktree has {} uncommitted change(s) that will be lost",
                uncommitted_files.len()
            ));
        }
        return Ok(report(true));
    }

    println!();
    output::print_info(&format!("Merge plugin '{}'", primary.name));
    println!("─────────────────────────────────────────────────────────────");
    output::print_info(&format!("  Strategy: {:?}", strategy));
    output::print_info(&format!("  Repo:     {:?}", source_repo));
    output::print_info(&format!(
        "  Branch:   {} ({} commit(s) to merge)",
        primary.branch, commit_count
    ));
    output::print_info(&format!("  Worktree: {:?}", worktree_path));
    if !commits_ahead.is_empty() {
        println!();
        output::print_warning(&format!(
            "Current branch is {} commit(s) ahead of '{}':",
            commits_ahead.len(),
            primary.branch
        ));
        for c in &commits_ahead {
            output::print_info(&format!("  ↑ {}", c));
        }
    }
    if !commits_behind.is_empty() {
        println!();
        output::print_warning(&format!(
            "Task branch '{}' is {} commit(s) ahead of current:",
            primary.branch,
            commits_behind.len()
        ));
        for c in &commits_behind {
            output::print_info(&format!("  ↓ {}", c));
        }
    }
    if has_uncommitted {
        println!();
        output::print_warning(&format!(
            "⚠ WARNING: Worktree has {} uncommitted change(s):",
            uncommitted_files.len()
        ));
        for f in &uncommitted_files {
            output::print_warning(&format!("  {}", f));
        }
        output::print_warning("  These changes will be permanently lost after merge!");
    }
    println!("─────────────────────────────────────────────────────────────");

    if !skip_confirm {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Merge plugin '{}' using {:?} strategy?",
                primary.name, strategy
            ))
            .default(true)
            .interact()
            .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
        if !confirmed {
            output::print_info("Merge cancelled for this plugin.");
            return Ok(report(false));
        }
        if has_uncommitted {
            let double_confirmed = dialoguer::Confirm::new()
                .with_prompt("Uncommitted changes will be lost. Continue?")
                .default(false)
                .interact()
                .map_err(|e| UdfError::Other(format!("Dialog error: {}", e)))?;
            if !double_confirmed {
                output::print_info("Merge cancelled for this plugin.");
                return Ok(report(false));
            }
        }
    }

    // === Fetch latest from origin before merge ===
    if let Err(e) = git::fetch_origin(&source_repo) {
        output::print_warning(&format!("Failed to fetch from origin: {}", e));
        output::print_warning(
            "Proceeding with local state only. Remote changes may not be detected.",
        );
    }
    git::fast_forward_upstream(&source_repo)?;

    output::print_info(&format!(
        "Merging plugin '{}' using {:?} strategy...",
        primary.name, strategy
    ));

    let success = match strategy {
        crate::cli::MergeStrategy::Rebase => {
            match git::rebase_branch(&source_repo, &primary.branch, &primary.based_on) {
                Ok(_) => {
                    output::print_success(&format!(
                        "Branch '{}' rebased successfully",
                        primary.branch
                    ));
                    true
                }
                Err(e) => {
                    output::print_error(&format!("Rebase failed: {}", e));
                    if force {
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        crate::cli::MergeStrategy::Merge => {
            // 只为在路径不是仓库时给出准确错误，句柄本身不再需要。
            let _ = git::open_repo(&source_repo)?;
            match git::merge_branch(&source_repo, &primary.branch) {
                Ok(_) => {
                    output::print_success(&format!(
                        "Branch '{}' merged successfully",
                        primary.branch
                    ));
                    true
                }
                Err(e) => {
                    if force {
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        crate::cli::MergeStrategy::Squash => {
            match git::squash_branch(&source_repo, &primary.branch) {
                Ok(_) => {
                    output::print_success(&format!(
                        "Branch '{}' squashed successfully",
                        primary.branch
                    ));
                    true
                }
                Err(e) => {
                    output::print_error(&format!("Squash failed: {}", e));
                    if force {
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        crate::cli::MergeStrategy::FfOnly => {
            match git::ff_only_merge(&source_repo, &primary.branch) {
                Ok(_) => {
                    output::print_success(&format!(
                        "Branch '{}' fast-forward merged successfully",
                        primary.branch
                    ));
                    true
                }
                Err(e) => {
                    if force {
                        false
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    };

    Ok(report(success))
}

#[allow(dead_code)]
fn delete_with_retry(path: &PathBuf, max_retries: u32) -> Result<()> {
    for attempt in 0..max_retries {
        match std::fs::remove_dir_all(path) {
            Ok(_) => return Ok(()),
            Err(e) if attempt < max_retries - 1 => {
                output::print_warning(&format!(
                    "Delete attempt {} failed (file locked?), retrying in 2s: {}",
                    attempt + 1,
                    e
                ));
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!()
}
