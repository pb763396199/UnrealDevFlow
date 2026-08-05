//! 分支台账：记住一个分支属于哪个任务。
//!
//! 分支名以前必须长成 `task/<workspace>/<task-id>`，不是因为好看，是因为工具没有
//! 别的地方记这件事。Host 目录一丢，元数据跟着没了，孤儿清理只能靠拼名字去找。
//!
//! 把这条信息写进 git 自己的分支配置（`branch.<分支名>.udftask`），名字就不用再
//! 承担身份。这份台账跟分支存在同一个仓库里，Host 删了它还在，`git worktree prune`
//! 也动不到它；而 `git branch -D` 会自己收走整个 `branch.<name>.*` 段，所以不需要
//! 写任何清理代码。

use crate::error::Result;
use std::path::Path;

/// git 把配置键名归一成小写，枚举时按这个匹配。
const LEDGER_KEY: &str = "udftask";

fn config_key(branch: &str) -> String {
    format!("branch.{}.{}", branch, LEDGER_KEY)
}

/// 记下这个分支属于哪个任务。
///
/// `task_ref` 用完整的 `<workspace>/<task-id>`，不是裸 task-id——同名 task-id 落在
/// 不同 workspace 时不能互相认领。
pub fn record(repo_path: &Path, branch: &str, task_ref: &str) -> Result<()> {
    let repo = crate::git::open_repo(repo_path)?;
    let mut config = repo.config()?;
    config.set_str(&config_key(branch), task_ref)?;
    Ok(())
}

/// 这个分支属于哪个任务？没记过就是 `None`。
pub fn task_of(repo_path: &Path, branch: &str) -> Result<Option<String>> {
    let repo = crate::git::open_repo(repo_path)?;
    let config = repo.config()?;
    match config.get_string(&config_key(branch)) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// 这个任务在这个仓库里有哪些分支？
///
/// 只返回仓库里真实存在的分支。用底层命令删分支会留下配置段，那种孤儿条目指向的
/// 分支已经不在了，跟着返回只会让调用方去删一个不存在的东西。
pub fn lookup(repo_path: &Path, task_ref: &str) -> Result<Vec<String>> {
    let repo = crate::git::open_repo(repo_path)?;
    let config = repo.config()?;
    let entries = config.entries(Some(&format!("branch\\..*\\.{}", LEDGER_KEY)))?;

    let mut branches = Vec::new();
    entries.for_each(|entry| {
        let (Some(name), Some(value)) = (entry.name(), entry.value()) else {
            return;
        };
        if value != task_ref {
            return;
        }
        // `branch.<分支名>.udftask`，分支名自己可能带斜杠和点，所以掐头去尾而不是切分。
        let Some(rest) = name.strip_prefix("branch.") else {
            return;
        };
        let Some(branch) = rest.strip_suffix(&format!(".{}", LEDGER_KEY)) else {
            return;
        };
        branches.push(branch.to_string());
    })?;

    branches.retain(|branch| repo.find_branch(branch, git2::BranchType::Local).is_ok());
    branches.sort();
    branches.dedup();
    Ok(branches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .expect("run git");
        assert!(status.success(), "git {:?} failed", args);
    }

    fn repo_with_commit() -> TempDir {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path();
        git(path, &["init", "-q", "-b", "dev", "."]);
        git(path, &["config", "user.email", "t@t"]);
        git(path, &["config", "user.name", "t"]);
        std::fs::write(path.join("a.txt"), "hi").expect("write");
        git(path, &["add", "a.txt"]);
        git(path, &["commit", "-q", "-m", "init"]);
        temp
    }

    #[test]
    fn a_recorded_branch_can_be_found_by_its_task() {
        let temp = repo_with_commit();
        let path = temp.path();
        git(path, &["branch", "feature/save-bug"]);

        record(path, "feature/save-bug", "neon-dev1/save-bug").expect("record");

        assert_eq!(
            lookup(path, "neon-dev1/save-bug").expect("lookup"),
            vec!["feature/save-bug".to_string()]
        );
        assert_eq!(
            task_of(path, "feature/save-bug").expect("task_of"),
            Some("neon-dev1/save-bug".to_string())
        );
    }

    #[test]
    fn a_branch_name_carrying_slashes_and_dots_still_parses_back() {
        let temp = repo_with_commit();
        let path = temp.path();
        // 分支名本身带点，键名解析不能靠切分
        git(path, &["branch", "release/v1.2"]);

        record(path, "release/v1.2", "ws/task").expect("record");

        assert_eq!(
            lookup(path, "ws/task").expect("lookup"),
            vec!["release/v1.2".to_string()]
        );
    }

    #[test]
    fn deleting_the_branch_takes_the_ledger_entry_with_it() {
        let temp = repo_with_commit();
        let path = temp.path();
        git(path, &["branch", "fix/gone"]);
        record(path, "fix/gone", "ws/gone").expect("record");

        git(path, &["branch", "-D", "fix/gone"]);

        assert!(lookup(path, "ws/gone").expect("lookup").is_empty());
        assert_eq!(task_of(path, "fix/gone").expect("task_of"), None);
    }

    #[test]
    fn recording_the_same_branch_twice_overwrites_rather_than_accumulates() {
        let temp = repo_with_commit();
        let path = temp.path();
        git(path, &["branch", "feature/again"]);

        record(path, "feature/again", "ws/first").expect("first");
        record(path, "feature/again", "ws/second").expect("second");

        assert!(lookup(path, "ws/first").expect("lookup").is_empty());
        assert_eq!(
            lookup(path, "ws/second").expect("lookup"),
            vec!["feature/again".to_string()]
        );
    }

    #[test]
    fn a_ledger_entry_left_behind_by_a_low_level_delete_is_ignored() {
        let temp = repo_with_commit();
        let path = temp.path();
        // 分支从来没建过，只有台账条目——底层命令删分支会留下这种孤儿
        record(path, "feature/never-existed", "ws/ghost").expect("record");

        assert!(lookup(path, "ws/ghost").expect("lookup").is_empty());
    }
}
