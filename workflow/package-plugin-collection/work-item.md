---
schema_version: 1
protocol: 1.3.0
id: wi_01KZZ8HKAQ08JBV7YZ9NG6SF35
short_id: bt0cwcve
title: 支持打包 Host 中的插件集合目录
status: done
kind: fix
created_at: 2026-08-14T04:30:06Z
created_by: Codex
source: user
home_repository: https://github.com/pb763396199/UnrealDevFlow.git
branch_or_pr: fix/package-plugin-collection
base_revision: 8461e00c19dc6a210b80451cd9e3bb9cef1e8c28
---

# 支持打包 Host 中的插件集合目录

## 原始请求

对任务 `neon-dev/unreal-mcp-functional-eval` 的 `Plugins/UnrealMCP` 执行 Package。实际检查发现该目录没有同名描述文件，而是包含多个独立插件。

## 目标

让 `package plugin UnrealMCP --task ...` 能把插件集合目录作为一个发布目标，递归发现并规划其中的独立插件。

## 范围

修改插件发现、依赖闭包、暂存路径、命令生成和输出组织；不改变普通单插件命令语义。

## 强约束

- check 和 plan 继续无执行记录。
- 集合内重名插件必须明确拒绝。
- 插件隔离编译继续全部使用 `-NoMutex`。

## 验收条件

- AC-001: 集合目录能展开嵌套 `.uplugin` 并生成每插件三阶段计划。
  Verify: case:plugin_collection_expands_nested_uplugins_without_an_execution
- AC-002: 普通单插件与 UEB 命令矩阵测试继续通过。
  Verify: case:isolated_plugin_matrix_never_takes_engine_global_mutex
- AC-003: UnrealMCP 真实 Host 的 check 为 ready，plan 输出 78 个无 Mutex 阶段且无 execution ID。
  Verify: case:real_unreal_mcp_collection_is_ready_and_plans_78_no_mutex_steps
