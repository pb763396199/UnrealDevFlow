---
schema_version: 1
protocol: 1.3.0
id: wi_01KZZ3WP75XMDZYJ25DBT1DF9R
short_id: bdph4ehb
title: 统一 CLI 检查、计划和状态命令的职责
status: done
kind: refactor
created_at: 2026-08-14T03:08:46Z
created_by: Codex
source: user
home_repository: https://github.com/pb763396199/UnrealDevFlow.git
branch_or_pr: refactor/unify-execution-query-semantics
base_revision: 40a81bf7b9b3059bb9f17bca6bff45b7fa55bc5c
---

# 统一 CLI 检查、计划和状态命令的职责

## 原始请求

统一考虑 Package、Build 以及其他一级分类中的 `check`、`plan`、`status`，明确差异、职责和是否需要保留。

## 目标

让同名二级命令在不同一级分类中回答同一种问题，避免检查、计划和真实执行状态互相覆盖。

## 范围

盘点并统一 Build、Package、Workspace、Task 和 Skill 的查询命令；调整命名、输出字段、记录规则和兼容入口。

## 强约束

- 同名二级命令必须使用相同解释和状态语义。
- 查询命令不得修改最近一次真实执行记录。
- 自动化必须能区分命令执行失败和业务结论不允许执行。
- 已发布命令需要明确兼容策略，不能静默改变脚本含义。

## 验收条件

- AC-001: `build check` 与 `package check` 只回答当前能否执行，并使用相同结论字段和状态词。
  Verify: case:build_and_package_check_share_readiness_contract
- AC-002: `build plan` 与 `package plan` 只展示将执行的步骤、参数和输出，不创建 execution ID 或覆盖最近执行。
  Verify: case:plans_do_not_replace_latest_execution
- AC-003: `build status` 与 `package status` 只读取已经启动的执行记录，并使用统一核心字段。
  Verify: case:status_reads_only_real_executions
- AC-004: Workspace、Task 和 Skill 的查询命令按 inspect、doctor、status、list、next 的统一词义完成审计。
  Verify: case:non_execution_queries_follow_the_shared_vocabulary
- AC-005: 旧调用要么保持等价，要么返回明确迁移提示，不得悄悄改成另一种行为。
  Verify: case:legacy_query_invocations_have_explicit_compatibility
