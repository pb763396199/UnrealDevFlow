---
schema_version: 1
protocol: 1.3.0
artifact: debug
artifact_id: ar_01KZ93EG4WSRVP1ZYPD0KEECXB
work_item_id: wi_01KZ921GEY99YYYE1S9GVNC501
created_at: 2026-08-05T14:10:00Z
producer: aes-debug
result: fixed
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8e8cfc19cd5d95fa0a6d13766dc6d2bbe7ec238051e1bd0772d5fa2863d4c2ba
  artifacts: []
  subject:
    kind: change_set
    digest: sha256:26788b3cfc575084abeaf11f862be4bff843241f32b204f404c98b759581a953
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: b4df27af5433ed52df9396edbe030874642fdda6
    revision: 699652f6ba165c5c4a9aa91370d56558c42aac78
    tree: ba8f734bdcf1936b8c9e6b6c828d75c3313deb72
    content_digest: sha256:26788b3cfc575084abeaf11f862be4bff843241f32b204f404c98b759581a953
    branch_or_pr: fix/main-repo-index-pollution
    workflow_excluded: true
---

## 现象

用户在 `F:\ShanghaiP4\neon\Plugins\AesWorld` 看到自己没暂存过的改动，说这不是第一次。
直接后果是 `task create` 的干净工作区检查把它们当成在制品，拒绝建任务。

取证时拿到的（全程只读）：`git status --porcelain` 报 7 个 `M ` 文件，
`git diff --cached --stat` 显示 18 增 268 删。HEAD 的 reflog 停在 `e57ea72cd`，而
`git rev-parse HEAD` 是 `b0830c935`，`e57ea72cd` 是它的祖先——**分支被推进过，HEAD 的
reflog 却没记**。普通 git 命令移动当前分支一定会写 HEAD 的 reflog，写不上说明这一步不是
普通 git 命令干的。

症状在排查途中自行消失（有并行会话在动那个仓库），所以从代码路径倒推，在临时仓库里重建。

## 最小复现

```
建任务 → 在 worktree 里提交一个新文件 → udf task merge <ref> --strategy merge

合并前： status 干净，index tree == HEAD tree
合并后： D  Source/AesWorld/feature.cpp
        diff --cached: 1 deletion
        工作区里根本没有 feature.cpp —— 但 HEAD 说该有
```

## 怀疑过什么，怎么排除的

| 假设 | 证据 | 结论 |
| --- | --- | --- |
| `fast_forward_upstream` 移动了分支 | 它跑的是 `git merge --ff-only`，真 git 命令，reflog 里也确实留了 `merge refs/remotes/origin/dev: Fast-forward` | 排除 |
| `rebase` 的 `reset` + `cherry-pick` 留下残骸 | 它先查工作区是否干净、建备份 ref、失败时 `cherry-pick --abort` 加 `reset --hard` 回滚 | 排除 |
| `task create` 弄脏主仓库 | `git worktree add` 不碰主工作区的索引和 HEAD；submodule 初始化跑在 worktree 里。补了测试实测 | 排除 |
| `merge` 策略用 git2 只移 ref | 见下 | **成立** |

## 根因

`src/git/mod.rs` 的 `merge_branch`：

```rust
let mut index = repo.merge_commits(&head_commit, &branch_commit, None)?;  // 内存里的 index
let tree_id = index.write_tree_to(repo)?;
repo.commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&head_commit, &branch_commit])?;
```

`merge_commits` 算出的是一个**内存中的** index，不是仓库的 index。`repo.commit(Some("HEAD"), ..)`
只移动分支引用。索引和工作区一个都没碰。

于是 HEAD 领先于索引和工作区：合并带进来的每个文件都显示成一条没人做过的暂存改动，
`git diff --cached` 全是删除。四种策略里只有 `merge` 这么写，`squash` 和 `ff-only` 早就是
shell 出去调真 git。

这也解释了 reflog 那条矛盾：libgit2 更新 `refs/heads/dev` 及其 reflog，HEAD 自己的 reflog
没跟上。

## 顺着查出来的第二处

`squash_branch` 有同类的两个洞。

**冲突判定读错了流。** 代码只查 stderr，但实测 `git merge --squash` 把 `CONFLICT` 写在 stdout：

```
--- stdout ---
CONFLICT (content): Merge conflict in f.txt
--- stderr ---
（空）
```

所以冲突永远判不出来，不调 `merge --abort`，主仓库留在带冲突标记的半合并状态，暂存区里是
未合并条目。

**commit 失败时不回滚。** `git merge --squash` 按设计只暂存不提交。后面那句 `git commit`
一旦失败（钩子拒绝、没配 user.email、签名失败），整个合并结果就留在用户的暂存区里。

## 怎么修的

`merge_branch` 换成 `git merge --no-ff --no-edit`，跟另外三种策略一致，索引、工作区、引用和
reflog 由 git 一起管。失败时先 `git merge --abort`，免得把仓库留在 MERGING 状态；冲突仍然按
原来的 `MergeConflict` 错误往上报，判定同时看 stdout 和 stderr。

`squash_branch` 跟 `rebase` 对齐：开工前要求工作区干净——这样回滚才证明得了安全，因为之后
暂存的一定是我们自己弄的；冲突判定同时看两个流；任何失败路径都 `merge --abort` 加
`reset --hard HEAD`。

## 加了什么回归测试

单独一条守 `merge` 策略的索引同步，另外三条是审计：

```rust
fn assert_source_repo_undisturbed(source_repo: &Path, what: &str) {
    // status 为空、无未合并条目、不留 MERGE_HEAD 和 CHERRY_PICK_HEAD
}
```

覆盖四种合并策略、`create`、`delete`，外加一条故意制造冲突的 squash。

**这个 bug 能发出去，是因为没有任何测试问过「这条命令有没有弄脏用户的仓库」。** 补的是这个
缺口，不只是两个具体的洞。

负向验证：

| 测试 | 回退修复后 | 装回修复后 |
| --- | --- | --- |
| `merge_strategy_leaves_the_source_repo_index_matching_head` | FAILED，`left: "D  feature.txt"` | 通过 |
| `a_conflicting_squash_does_not_strand_the_source_repo` | FAILED，`left: "AA probe.txt"` | 通过 |

90 个用例全过（基线 86，新增 4），三条门禁退出码都是 0。

## 改了哪些代码

| 文件 | 改了什么 |
| --- | --- |
| `src/git/mod.rs` | `merge_branch` 换成真 git 命令；`squash_branch` 加干净前置、双流冲突判定、失败回滚 |
| `src/commands/merge.rs` | 调用点跟着改签名 |
| `tests/merge_yes.rs` | 1 条索引同步测试 + 3 条审计测试 + 1 条未跟踪文件测试 + 两个辅助函数 |

## 评审又打回一条

我给 squash 新加的干净前置用了 `git status --porcelain`，它默认包含未跟踪文件。用户的
`AesWorld` 长期躺着一个 `?? workflow/`，这一改之后 `--strategy squash` 在那台机器上会被
无条件拒绝。`rebase` 有同一个写法（老代码），用户上一个任务的冒烟里已经被它挡过一次。

这份严格换不到安全性：`git merge --squash` 本身不介意未跟踪文件，回滚用的 `reset --hard`
也不碰它们。两处都改成 `--untracked-files=no`，并补了一条覆盖 squash 和 rebase 的回归测试，
同样做过负向验证。

## 还剩什么风险

**没在用户的真实仓库上验过。** 全部在临时仓库里做，这是任务强约束要求的。

**审计只覆盖了会动插件仓库的命令。** `switch` 动的是 Junction 和 `state.json`，`build` 动的是
Host 目录，都不在这次的审计范围里。

**`git commit` 失败的那条路没有直接测试。** 要造这个场景得装一个会拒绝的 pre-commit 钩子，
这次只做了代码走查加冲突路径的实测。
