---
schema_version: 1
protocol: 1.3.0
id: wi_01M07CNY9RM7B36W3AK11ZVZ7F
short_id: 2b054ckt
home_repository: "https://github.com/pb763396199/UnrealDevFlow.git"
title: "创建任务时忽略未跟踪临时文件"
status: done
created_at: 2026-08-17T08:16:18Z
created_by: codex
kind: feature
branch_or_pr: feature/create-with-untracked-files
base_revision: b605b56e93f2214ec04736f9e0428ae77cd401a4
---

# 创建任务时忽略未跟踪临时文件

## 原始请求

> [$aes-go] feature 当前工具经常遇到创建网格区的时候，被主仓库的各种临时文件给挡住，导致不能正确创建网格区的问题。请给出一个合理且简洁优雅高效的解决方案。

图片中的旧对话作为背景参考：主仓库存在 `workflow/` 等非本次任务产生的未跟踪文件，`udf task create` 因“工作区不干净”被拒绝。

## 目标

让 `udf task create` 在主插件仓库只有未跟踪临时文件时仍能创建独立任务 worktree，同时继续保护会影响基线的已跟踪改动、暂存改动和冲突状态。

## 范围

- 只调整创建任务时的主插件基线检查。
- 为“创建检查只关注已跟踪状态”的语义增加可读的 Git 辅助函数和回归测试。
- 保留未跟踪文件在主仓库原位，不自动 stash、删除、移动或复制到任务 worktree。

## 强约束

- 不放宽主插件必须位于真实主 checkout、当前分支必须为 `dev` 等既有保护。
- 不改变 merge、squash、rebase、ff-only 的行为。
- 失败时不能遗留 Host、worktree、分支或依赖 Junction。
- 只在临时测试仓库中验证，不触碰用户的真实插件仓库。

## 验收条件

- AC-001: 主插件存在未跟踪文件或目录时，`udf task create` 成功；任务 worktree 不包含这些未跟踪内容，主仓库未跟踪内容保持原样。
  Verify: case:create_allows_untracked_files_in_primary_source_without_copying_them
- AC-002: 主插件存在已跟踪修改、暂存修改或未合并冲突时，`udf task create` 仍拒绝，并且不留下创建残骸。
  Verify: case:create_still_rejects_tracked_changes_when_untracked_files_are_present
- AC-003: 创建任务的现有主流程和路径保护回归测试全部通过。
  Verify: case:create_sources_regression_suite
- AC-004: `cargo fmt --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test` 全部通过。
  Verify: case:rust_quality_gates_pass
- AC-005: 变更保持在 `feature/create-with-untracked-files`，不修改 `F:\ShanghaiP4\neon\Plugins` 下的任何真实仓库。
  Verify: manual:核对本任务执行期间真实插件仓库未发生内容或索引变化
