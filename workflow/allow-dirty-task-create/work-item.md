---
schema_version: 1
protocol: 1.3.0
id: wi_01M1XFY0XRRPCQKBVK083X2P2Q
short_id: 3zxrq0ks
title: 允许从有普通修改的主检出创建任务
status: done
created_at: 2026-09-07
home_repository: https://github.com/pb763396199/UnrealDevFlow.git
kind: fix
branch_or_pr: fix/allow-dirty-task-create
base_revision: 284525b422739bd6b2689bf1e7aca9c08fc1835a
---
# 允许从有普通修改的主检出创建任务

## 原始请求

修复 `udf task create` 无法从带有已跟踪修改或已暂存修改的主检出创建任务的问题。主插件仍须位于 `dev`，任务工作区严格从当前 `HEAD` 创建，不带入主检出的未提交修改，也不能改动主检出状态；未合并冲突等不安全状态仍须拒绝。补齐四类回归测试，统一 README、发布说明、AI skill 与命令文案，并按 AES Workflow 完成提交和收口。

## 目标

让用户保留主检出的普通未提交修改，同时可靠地从当前 `HEAD` 创建隔离任务。

## 范围

修改任务创建前的仓库状态检查、回归测试和面向用户的说明。已有且不属于本任务的修改不纳入提交。

## 强约束

主插件必须位于 `dev`。任务工作区只包含当前 `HEAD` 的已提交内容。创建过程不能改变主检出的工作区或暂存区。未合并冲突继续阻断创建。

## 验收条件

- AC-001: 主检出只有未跟踪文件时，`udf task create` 成功，任务工作区不包含该文件，且主检出状态不变。
  Verify: case:task_create_allows_untracked_main_checkout_changes
- AC-002: 主检出有未暂存的已跟踪修改时，`udf task create` 成功，任务工作区读取到 `HEAD` 版本，且主检出状态不变。
  Verify: case:task_create_allows_unstaged_tracked_main_checkout_changes
- AC-003: 主检出有已暂存的已跟踪修改时，`udf task create` 成功，任务工作区读取到 `HEAD` 版本，且主检出状态不变。
  Verify: case:task_create_allows_staged_tracked_main_checkout_changes
- AC-004: 主检出存在未合并冲突时，`udf task create` 拒绝并给出可执行的原因。
  Verify: case:task_create_rejects_unmerged_conflicts
- AC-005: README、发布说明、AI skill 与命令输出一致说明普通修改允许保留，未合并冲突仍会阻断。
  Verify: case:task_create_dirty_checkout_docs_are_consistent
