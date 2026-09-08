---
schema_version: 1
protocol: 1.3.0
id: wi_01M1XH0G01JN3HHH886TS7ZMW4
short_id: ns4dy38b
title: 修正项目打包缓存的记账、清理和复用规则
status: done
created_at: 2026-09-07T16:50:57+08:00
home_repository: https://github.com/pb763396199/UnrealDevFlow.git
kind: fix
branch_or_pr: fix/package-cache-lifecycle
base_revision: 284525b422739bd6b2689bf1e7aca9c08fc1835a
---

# 修正项目打包缓存的记账、清理和复用规则

## 原始请求

用户要求查清 `C:\Users\YUMEI\AppData\Local\Temp\UDF` 和
`C:\Users\YUMEI\.unrealdevflow\package` 的用途、任务归属与遗留原因，检查是否还漏了别的目录，
再给出完整的清理与优化方案。

## 目标

让 UDF 准确报告自己占用的磁盘空间，安全清除失效材料，并限制项目打包缓存长期增长。

## 范围

- 盘点项目打包、插件打包、交付备份、执行日志、最终包和旧版本目录。
- 明确 workspace、UDF task、AES Work Item 和 package execution 的归属关系。
- 设计现有机器的一次性清理方案，以及后续自动清理、容量限制和复用规则。
- 补齐命令输出、迁移、失败恢复和测试计划。
- 收缩插件打包 staging：成功交付后立即回收私有构建目录，避免成功任务继续占用 `%TEMP%\UDF`。
- 插件 staging 只保留私有 `Intermediate`、`Binaries` 和 `Saved`；源码、Content 与其他只读目录不再整棵复制。
- 不在调查和设计阶段删除现有目录，也不改写执行记录。

## 强约束

- 最终包和用户未授权的文件不能成为自动清理目标。
- 清理必须先盘点，再按可核对的归属证据删除，不能根据目录名猜测。
- 正在运行的 UAT、UBT、Editor 或 package execution 使用的目录不能删除。
- 旧执行记录缺字段时必须保守处理，并提供显式迁移或隔离办法。
- Windows 路径长度、Junction 和跨磁盘输出仍要受现有保护。

## 验收条件

- AC-001: 一份调查记录列出所有 UDF package 存储位置、当前字节数、用途、归属字段和现有清理入口，并给出可复算命令。
  Verify: manual:按 research/storage-lifecycle-research.md 的 PowerShell 命令复算两处根目录、备份、日志和最终包字节数。
- AC-002: 方案能区分当前可复用缓存、失效 revision、失败或中断材料、旧格式材料、交付备份和最终包。
  Verify: case:final_output_is_never_an_automatic_cleanup_target
- AC-003: `package clean --dry-run` 的计划行为能报告全部受管材料，并说明每个目标为什么可清理或为什么必须保留。
  Verify: case:clean_inventory_reports_categories_and_protects_final_outputs
- AC-004: 缓存复用键不因无关的 profile revision 或输出名称变化而新建完整项目副本，真正不兼容的输入仍隔离。
  Verify: case:cache_key_ignores_profile_revision_name_output_and_reason
- AC-005: 缓存具备容量、数量和时间三类限制，清理时保护当前使用项和用户明确保留项。
  Verify: case:global_preflight_uses_historical_cache_size_and_exposes_cap
- AC-006: 项目包和插件包执行记录都能保存 UDF task 归属；AES Work Item 没有直接绑定时明确显示未绑定。
  Verify: manual:检查一次 task project/plugin execution JSON 的 taskRef，并确认 inventory 不生成 AES Work Item 字段。
- AC-007: 旧格式执行记录和孤儿目录有一次性迁移或隔离方案，现有 1 TB 级材料可以先预览再定向清理。
  Verify: case:scoped_stale_cleanup_requires_yes_and_updates_each_record
- AC-008: 计划列出回归测试、故障注入、真实目录 dry-run 和安装版命令验证，且不要求触碰最终包。
  Verify: manual:按 plan.md 的 S8-S11 执行 release、dry-run、进程检查和清理后复核。
- AC-009: 插件包完成交付且最终输出摘要校验通过后，私有 staging 会立即删除；交付失败时 staging 和恢复材料仍保留。
  Verify: case:successful_plugin_delivery_removes_private_stage
- AC-010: 插件打包 staging 不复制 Source、Content、Config、Resources、Shaders 或 ThirdParty 等只读目录；这些目录以 Junction 指向源插件，`Intermediate`、`Binaries`、`Saved` 始终私有。
  Verify: case:plugin_stage_uses_read_only_junctions_and_private_generated_dirs
