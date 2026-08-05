---
schema_version: 1
protocol: 1.3.0
id: wi_01KZ921GEY99YYYE1S9GVNC501
short_id: vy0x6agh
home_repository: "https://github.com/pb763396199/UnrealDevFlow.git"
title: "udf 把主插件仓库的暂存区搞脏"
status: done
created_at: 2026-08-05T13:35:00Z
created_by: claude
kind: fix
base_revision: b4df27af5433ed52df9396edbe030874642fdda6
---

# udf 把主插件仓库的暂存区搞脏

## 原始请求

> 这就是我最讨厌你设计的这个工具的地方啊！那个暂存其实是你的工具导致的，你的之前的某些命令
> 会导致让主仓库的分支去拿下某些东西，然后变成 stage 进行污染。我已经发现好多次了。
> 你把这个东西当成一个新任务，赶紧做完。

用户说这不是第一次，是反复出现的。

## 目标

udf 跑完之后，主插件仓库的暂存区跟跑之前一样。

## 已经拿到的证据

在 `F:\ShanghaiP4\neon\Plugins\AesWorld` 上取的，全程只读。

**症状**：`git status --porcelain` 报 7 个 `M ` 文件（已暂存、工作区与暂存区一致），
`git diff --cached --stat` 显示 7 个文件、18 增 268 删。用户没有暂存过这些文件。

直接后果：`task create` 的干净工作区检查把这些当成用户的在制品，拒绝建任务。这就是这次
在真实 AesWorld 上跑不动的原因。

**HEAD 的 reflog 跟 HEAD 对不上**：

| 项 | 值 |
| --- | --- |
| `git rev-parse HEAD` | `b0830c935` |
| `git reflog -1`（HEAD 的 reflog） | `e57ea72cd  commit: fix(build): 修复 AesWorld UE5.5 Runtime 构建依赖` |
| `git reflog show dev -1` | `b0830c935` |
| `e57ea72cd` 是 HEAD 的祖先吗 | 是 |

也就是说 `dev` 被推进过，`dev` 的 reflog 记了，**HEAD 的 reflog 没记**。用普通 git 命令
移动当前分支一定会写 HEAD 的 reflog，写不上说明这一步不是普通 git 命令干的。

**HEAD 的 reflog 里有 udf 的指纹**：

```
HEAD@{2}  merge refs/remotes/origin/dev: Fast-forward
HEAD@{3}  checkout: moving from new_vege_editor_clean to dev
HEAD@{4}  checkout: moving from dev to new_vege_editor_clean
HEAD@{9}  checkout: moving from dev to new_vege_editor_clean
HEAD@{10} reset: moving to 55a598213
HEAD@{11} cherry-pick: feat(vegetation): ...
```

`reset` 加 `cherry-pick` 加来回 `checkout` 正是 `task merge --strategy rebase` 的三步法，
它跑在**主插件仓库**里而不是任务 worktree 里。

**症状在排查途中自行消失**：几条只读命令之后 `git status` 只剩 `?? workflow/`，
`git diff --cached` 空，索引树等于 HEAD 树。`.git/index` 的 mtime 是当天。所以现在不可
复现，得从代码路径倒推。

## 范围

做：找出 udf 哪条命令会让主插件仓库的 HEAD、索引或工作区处于不一致状态；修掉；加回归测试。

不做：不改用户仓库里已有的任何东西；不放宽 `task create` 的干净工作区检查——那道检查是对的，
它这次正确地拦住了。

## 强约束

- **只读排查用户的真实仓库。** 复现要在临时仓库里做。
- 不能靠放宽检查来绕过。
- 修完不能破坏 merge 的三种策略（rebase / merge / squash / ff-only）现有行为。

## 验收条件

- AC-001: 找到并写清根因——哪条命令、哪一行、在什么条件下让主仓库的索引与 HEAD 脱节。
  证据是能在临时仓库里稳定复现的一串命令。
- AC-002: 修复之后，同一串复现命令跑完，主仓库 `git status --porcelain` 与跑之前逐字节相同。
- AC-003: 有回归测试守着，并且做过负向验证——回退修复后测试失败，装回修复后通过。
- AC-004: merge 的四种策略行为不变，现有 17 个 merge/cleanup 用例继续全过。
- AC-005: 排查和修复全程没有改动 `F:\ShanghaiP4\neon\Plugins` 下任何仓库的内容。
- AC-006: `cargo fmt --check`、
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、
  `cargo test` 三条退出码都是 0。
