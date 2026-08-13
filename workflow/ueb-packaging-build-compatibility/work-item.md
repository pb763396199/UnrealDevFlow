---
schema_version: 1
protocol: 1.3.0
id: wi_01KZWHP5WGDDBVMW0WVRJTAJZE
short_id: 0ym709a
title: 分析 UEB 打包发布编译能力兼容性
status: done
kind: research
created_at: 2026-08-13T00:00:00Z
created_by: Codex
source: user
home_repository: https://github.com/pb763396199/UnrealDevFlow.git
branch_or_pr: research/ueb-packaging-build-compatibility
base_revision: 5138373f7a15d01c95a06022c153f73c8be7b3e6
---

# 分析 UEB 打包发布编译能力兼容性

## 原始请求

分析 `F:\ShanghaiP4\neon\Plugins\UEB` 提供的全部编译能力，判断 UnrealDevFlow 的编译体系能否兼容这套面向打包发布的编译命令。

## 目标

给出有源码证据的能力清单、逐项映射、兼容边界和后续设计输入。

## 范围

只读分析 UEB 与 UnrealDevFlow 当前实现。覆盖命令入口、参数、阶段、产物、缓存、平台、错误处理和发布用途。本轮不实现兼容代码。

## 强约束

- 结论必须能回到具体文件、命令或测试。
- 区分直接兼容、需要适配、语义冲突和缺失能力。
- 不把 UEB 的专用实现细节直接当成 UnrealDevFlow 的产品要求。

## 验收条件

- AC-001: 列出 UEB 可发现的全部编译和打包命令及其关键参数，并给出源码位置。
  Verify: case:project_package_argv_matches_ueb_default_buildcookrun
- AC-002: 列出 UnrealDevFlow 当前编译体系的对应能力及约束，并给出源码位置。
  Verify: case:build_and_workspace_gain_planned_taxonomy_actions
- AC-003: 逐项判断直接兼容、适配后兼容、冲突或缺失，并说明依据。
  Verify: case:engine_source_build_argv_matches_ueb_three_steps
- AC-004: 给出可实施的兼容边界、主要风险和需要产品拍板的问题。
  Verify: case:package_clean_refuses_a_recorded_path_outside_managed_artifacts
- AC-005: 形成一级命令分类和统一二级命令词典；同名二级命令保持相同命名、解释、参数习惯、输出字段和状态含义。
  Verify: case:same_named_secondary_commands_share_the_same_help_text
- AC-006: 插件发布使用独立 staging 时不得占用 UE 全局 UBT Mutex，也不得阻塞普通 Host 编译。
  Verify: case:isolated_plugin_matrix_never_takes_engine_global_mutex
