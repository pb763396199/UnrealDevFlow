---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01M07CQNFVDJEE2DGP5Q9YY6EB
work_item_id: wi_01M07CNY9RM7B36W3AK11ZVZ7F
created_at: 2026-08-17T08:18:00Z
producer: aes-brainstorm
result: accepted
supersedes: null
dependencies:
  work_item_contract_digest: sha256:d444f7a30ab66f3cab02a47ea7ae326e65adacd42265fe8feaf42c8b4353500f
  artifacts: []
---

# 设计：创建任务时忽略未跟踪临时文件

## 现状与缺口

`src/commands/create.rs` 的 `prepare_primary_plans` 需要确认主插件位于 `dev` 分支，随后调用通用的 `git::status_porcelain`。该调用默认包含未跟踪文件，因此主仓库里的 `workflow/`、编辑器临时文件或本地脚本会被当成阻断创建的“脏工作区”。

这里真正需要保护的是任务基线不能混入未提交的 tracked/index 内容；未跟踪文件不会进入提交点，也不会被 `git worktree add` 复制到新的 Host worktree。

## 方案对照

| 方案 | 做法 | 取舍 |
| --- | --- | --- |
| A：创建边界忽略未跟踪项（推荐） | 在创建检查使用 `git status --porcelain --untracked-files=no`，仍阻断 tracked、staged 和 conflict 状态 | 一行语义清楚、无文件移动、无恢复状态，兼容现有流程 |
| B：自动 stash | 创建前执行 `git stash -u`，完成后再恢复 | 会改变用户仓库状态，可能遇到 stash 冲突、恢复失败或并发会话，不适合作为默认自动行为 |
| C：临时文件白名单 | 只忽略 `workflow/`、编辑器目录等指定路径 | 规则会持续膨胀，无法覆盖用户自定义临时文件，且容易漏掉真实未跟踪文件 |

## 拍板结果

采用方案 A。新增一个表达意图的 Git 辅助函数，而不是改变通用 `status_porcelain` 的含义；创建命令只在自己的边界选择“已跟踪状态检查”。这样其他未来需要完整状态的调用不会被隐式改变。

错误信息继续显示实际阻断的 tracked/index 状态。未跟踪项不再出现在创建失败信息中，也不做删除、stash、移动或复制。

## 场景与状态流转

1. 主插件在 `dev`，只有未跟踪文件：已跟踪状态为空，继续解析依赖、预览并创建 Host/worktree；未跟踪文件仍留在主仓库。
2. 主插件有 tracked 修改、暂存修改或冲突：已跟踪状态非空，在创建 Host/worktree 前拒绝，现有路径保护不变。
3. 创建中途失败：沿用现有 partial-create 回滚；因为未跟踪文件从未被移动，重试不会产生 stash 或恢复副作用。
4. 中断后重试：主仓库仍保持原状；若 Host 路径已存在，现有任务存在检查继续阻止重复创建，避免覆盖残留。

## 验收映射

- AC-001：新增未跟踪文件成功创建，并断言文件不进入 worktree、源仓库状态保持。
- AC-002：补充 tracked 修改与暂存状态的拒绝断言，确保保护没有被放宽。
- AC-003：运行现有 `create_sources` 测试。
- AC-004：运行格式、Clippy 和全量测试。
- AC-005：代码和 workflow 记录只在当前 UnrealDevFlow 任务分支中产生。
