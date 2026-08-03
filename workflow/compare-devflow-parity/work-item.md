---
schema_version: 1
id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
short_id: 47axmb9j
home_repository: "https://github.com/pb763396199/UnrealDevFlow.git"
title: "把 DevFlow 从 UnrealWorkflow 拆回独立工具"
status: done
created_at: 2026-08-03T06:44:37.804286Z
created_by: codex
selected_attempt: at_01KZ35WCEAJTX3MNDKH02BCYDS
---

# 把 DevFlow 从 UnrealWorkflow 拆回独立工具

## 原始请求

> 将当前插件的功能与 Unreal Workflow （F:\AiProject\UnrealWorkflow）中的 devflow 部分的所有功能进行一个一一对比，看当前工程与 Unreal Workflow 相差多少功能，有何异同？该如何推进功能上的完全对齐甚至超越？

澄清后的方向：

> 应该先对齐的是命令行的数量。因为本来 UWF 的 Devflow 就是基于 Unreal Devflow 整合进去的，我其实本质上最想要的是把 Devflow 重新提出来，变成一个独立的工具，跟 UWF 进行解耦。

## 目标

让 `unrealdevflow` 这个独立命令行工具重新拿回 UWF 里的全部能力，不再依赖 UWF 才能用。

## 范围

做：清点两边命令与能力的差集；定出以哪边为主干；把缺的能力补齐；切断对 `uwf-core` 的依赖。

不做：改 UWF 的 AgentHub、KnowledgeBase、UnrealMaster 三个模块；不做 UWF 侧的兼容层设计；
不在本仓库实现任何虚幻相关功能。

## 强约束

- `unrealdevflow` 独立后不能依赖 `uwf-core` 或 UWF 工作区里的任何 crate。
- 已发布的命令行接口不能无故改名，用户装的是 GitHub Release 版本。
- 两边同源代码已各自演进，合并必须双向，不能单向覆盖。

## 验收条件

- AC-001: 列出一张差集表，逐条写明 UWF DevFlow 的 15 个动作与 `unrealdevflow` 的 17 个顶层命令
  各自独有哪些能力，能一一对应的写明对应关系。
- AC-002: 独立后的 `unrealdevflow` 能执行 UWF DevFlow 独有的受控构建检查，即 `build-check`
  和 `build-gate` 对应的能力，且不引入 `uwf-core` 依赖。
- AC-003: `cargo build` 在 `UnrealDevFlow` 仓库单独执行成功，依赖树里没有任何 `uwf-*` crate。
- AC-004: UWF 侧领先的引擎能力全部回流，即 `host/mod.rs`、`commands/switch.rs`、
  `commands/build_status.rs` 三处的增强在独立工具里可用。
- AC-005: `unrealdevflow` 侧领先的两个提交不丢失，即 Task#032 与 Task#033 的行为在合并后仍成立。
