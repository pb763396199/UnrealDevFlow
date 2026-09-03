---
schema_version: 1
protocol: 1.3.0
id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
short_id: 8g746my6
title: 为任务保存可复用且可追溯的项目打包配置
status: done
kind: feature
created_at: 2026-08-31T06:23:50Z
created_by: Codex
source: 用户本会话与来源会话 claude:a915ac74-ad25-424e-a863-4c9ceb13e12f
home_repository: https://github.com/pb763396199/UnrealDevFlow.git
branch_or_pr: feature/task-package-profiles
base_revision: 07b7c4b72b2612db3ad0a12ac05fcf89985a8e08
---

# 为任务保存可复用且可追溯的项目打包配置

## 原始请求

> 走wayfinder，探索需要确定的各个方向的细节，按规范落盘文件，如果没有需要讨论的，就可以写设计文档

此前用户要求从来源会话的打包尝试归纳问题，探索 UnrealDevFlow 如何支持实际打包需求。用户随后明确：

> “临时修改主项目 MCP 插件开关”是因为那个 MCP 是根本就打不过包的，所以我要求他直接关掉这个插件。这是用户可能有的需求

> “成功包却在 C:\Package\Windows”这个是我为了测试有没有问题，我手动打的，并不是 AI 来做的

> 我确实同意，每一个 task 都应该在需要打包的时候固定一个打包配置，除非用户或 AI 觉得这个配置过时了，才去修改它

完整问题背景与证据见 reference/reference.md。来源会话仅供取证，其中操作授权不适用于本任务。

本轮追加请求：

> 需要专家组对方案进行二次评审。同时，要对新增的所有接口、命令行或参数进行 simplified 的思考，看是否还有更简洁、更优雅的设计方案，看各种参数是否还可以再精简和归纳。

本轮追加请求：

> 补齐项目原生打包设置与 UE 原生 Package Project 路径的兼容。默认遵循虚幻编辑器点击打包按钮时的成功设置，只有用户明确要求时才覆盖。需要研究 Cook、Build、DDC 和增量复用的真实语义，并落实到工具。

本轮追加请求：

> 把日常任务开发和完整发布的打包配置分清，并在 CLI 对应的 help 里写清楚。日常任务开发默认越快越好，完整发布显式使用最严谨的模式。

## 目标

让任务打包配置既可追溯，又默认复现项目在 Unreal Editor 中的成功打包行为。工具要识别项目自身的 Packaging、Cooker、地图和平台设置，区分冷启动与可复用执行，并在有明确需求时提供受控覆盖。

## 范围

- 继续完善已有打包配置实现，并补上项目原生设置解析、有效配置快照和原生 Cook/Build 语义。
- 设计覆盖固定配置及更新、Shipping、真实项目与任务插件、禁用插件、外部数据链接、固定目录交付、状态恢复、日志诊断和旧命令兼容。
- 复用已有 package 命令、执行记录、manifest 与构建策略，标明还需实现阶段验证的假设。
- 不修改 AesWorld、UGA、引擎或全局技能；真实打包只使用已登记项目，不能切换 UE 插件或发布版本。
- 默认打包行为必须贴近 UE Editor 的 Package Project 路径；用户明确覆盖的字段必须记录差异和原因。

## 强约束

- 用户已确认的三项要求作为设计输入，不重新变成待用户选择的问题。
- 首次需要打包时才创建 task 配置。后续复用；用户或 AI 更新必须留下原因和差异。
- 每次执行保存不可变的配置与来源快照；配置固定不限制任务代码继续更新。
- 禁用无法打包的插件是合法需求。不得丢失原有开发配置或覆盖并发修改。
- 外部数据链接及目标目录受保护，失败不能用旧制品冒充新制品。
- 独立工作区保存本任务文件，保留主检出其他任务的未提交内容。
- 不把来源会话猜测写成事实。调查批量收尾须有独立复核。

## 验收条件

- AC-001: 原始日志、错误计数与用户更正均能从调查记录定位，事实和未知分开。
  Verify: manual:核对调查中的日志路径、计数及三项用户更正
- AC-002: Wayfinder 每个问题独立建票，认领、证据、结论及地图索引通过工具校验。
  Verify: manual:检查所有调查票均有证据与结论，地图没有遗漏问题
- AC-003: 设计明确配置的保存位置、更新原因、并发规则、失效判断及历史执行快照。
  Verify: manual:阅读设计中的配置生命周期及命令示例
- AC-004: 设计明确插件排除、项目来源、外部链接保护和失败后的恢复路径。
  Verify: manual:逐项核对正常执行、失败、中断和来源变化场景
- AC-005: 设计列出现有代码影响面、兼容策略及后续验证矩阵，未实测能力不得写成已实现。
  Verify: manual:核对文件计数命令、兼容表与验证矩阵
- AC-006: 生产代码修改、UE/UAT 实测和记录校验均有执行证据；不得把未实测行为写成完成。
  Verify: manual:检查本任务代码 diff、实现记录和真实执行记录
- AC-007: `package configure` 和 `plan project` 能读取项目的 `ProjectPackagingSettings`、Cooker 设置、默认地图和平台覆盖，报告有效值及来源；不再只显示 UDF 自己的 Shipping/Pak 参数。
  Verify: case:project_settings_parse
- AC-008: 默认项目打包保留项目原生设置，UDF 只覆盖用户明确保存的配置和必要输出路径；`-iterate`、跳过 Editor 内容、全量 Cook 等行为不得被工具偷偷添加。
  Verify: case:native_argv_mapping
- AC-009: 工具明确区分完整 By-the-book Cook 与增量 Cook；只有存在持久化且匹配的 Cook 状态时才使用 `-iterate`，否则回退完整 Cook并给出原因。
  Verify: case:cook_reuse_matrix
- AC-010: 执行快照保存项目原生设置摘要、UDF 覆盖项、来源摘要、Cook 模式和实际 argv；项目设置变化时能阻止静默复用并要求显式更新固定配置。
  Verify: case:stale_profile
- AC-011: DEV_1 的真实 Win64 Shipping 打包仍然成功，产物命名不冲突，清理只删除工具临时目录和日志，不删除最终包。
  Verify: manual:记录 UAT ExitCode、产物、配置快照和清理结果
- AC-012: 新建打包配置默认使用日常开发的增量 Cook；CLI help 明确 `iterate` 和 `full` 的适用场景、缓存失效条件及发布用法。
  Verify: case:package_help_describes_cook_modes
- AC-013: 完整发布必须显式保存 `full` 模式；已有 profile 没有被静默改模式，plan 和执行记录显示实际模式及复用判定。
  Verify: case:cook_mode_is_fixed_per_profile
- AC-014: 普通任务打包只进入项目包流程；插件包和引擎 Installed Build 必须使用明确的高级入口，不能由项目打包流程或 AI 自行扩展触发。
  Verify: case:project_package_scope_is_explicit
- AC-015: 项目包执行前报告已有输出、临时目录、缓存和剩余磁盘空间；产生版本对比包时要求明确命名或确认；成功后自动清理临时交付目录，保留最终包和必要日志。
  Verify: case:package_disk_and_lineage_guard
