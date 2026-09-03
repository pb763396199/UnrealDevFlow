---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01M1DXZWBW3073G1TNNW48AG5S
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-01T07:35:00Z
producer: aes-brainstorm
result: superseded
supersedes: ar_01M1B9A2M0VRC0N808JQA3KRKR
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1DY0G9HF11ZFAJKST8W2VAK
      digest: sha256:d0ee41db2a0ed9005e050cedc2b4f12728496a248f8395098f506890becde033
      locator: research/ue-native-packaging-v2-research.md
---

# 原生 UE 打包兼容设计 v3

## 设计目标

让 `udf package project` 默认复现 Unreal Editor 的 Package Project 语义。项目自己的 Packaging、Cooker、地图和平台设置是基线；UDF 只负责来源隔离、最终输出、命名、插件排除和用户明确保存的覆盖。工具不再把一套固定的 UAT 参数当成项目配置。

## 两个可行方案

### 方案 A：项目配置基线加受控覆盖，推荐

`configure` 读取项目配置中的可识别设置，保存规范化快照和摘要。常用 profile 只保存配置、容器、输出、包名和禁用插件；未指定的字段从项目基线继承。`plan` 展示项目值、UDF 覆盖和最终 argv。执行时把项目配置复制到隔离副本，让 UAT 继续解析 UE 自己的完整配置层级。

优点是默认行为贴近原生路径，UE 新增设置不会被 UDF 重新实现。代价是 UDF 只能对项目明确配置做有证据的摘要，不能假装完全解析 Engine Default。

### 方案 B：UDF 完整重建所有 UE 默认配置

UDF 自己合并 Engine、Platform、Project 和 User 配置，再把所有最终设置转换成 argv。

不采用。UE 配置层级和字段会随引擎版本变化，UDF 重建一套解释器容易与 Editor 分叉，也会把每个 UE 设置变成新 CLI 参数。

## 固定决策

1. 项目配置基线

   解析 `ProjectPackagingSettings`、`CookerSettings`、`GameMapsSettings`、Windows 平台配置和项目 `.uproject` 插件状态。保存原始来源文件、规范化键值和摘要。未识别的项目设置保留在快照摘要中，不擅自转换成命令行。

2. 常用 profile

   保留现有 `configuration`、`container`、`output`、`name`、`disabled_plugins`。首次配置时，configuration 和 container 默认从 Windows 项目设置推导；用户显式传入时才覆盖。output 和 name 始终由 UDF 管理，因为它们属于交付路径和冲突避免。

3. UAT 参数

   `project_package_commands` 根据项目快照和显式 profile 覆盖生成 `-clientconfig`、`-pak/-iostore`、`-compressed`、`-prereqs`、`-build/-skipbuild` 等参数。`-stage`、`-archive`、`-package` 是执行 Package Project 所需的动作。没有项目或用户依据时不加入 `-allmaps`、`-skipcookingeditorcontent`、`-iterate`。

   项目配置已经表达的值可以出现在最终 argv 中，但计划必须标出它们的来源是 project，而不是 UDF default。项目配置与显式 profile 发生冲突时，以 profile 覆盖为准，并记录旧值、新值和原因。

4. Cook 模式

   默认 `full`，对应 UE By-the-book Package Project。高级候选配置可以声明 `cook.mode = "iterate"`，但只有持久化 Cook 状态、项目设置摘要、来源摘要、引擎版本、平台、配置和容器全部匹配时才传 `-iterate`。任一摘要不匹配就回退 full，并在计划和执行记录中说明原因。

   `iterate` 使用每个 profile 独立的持久化 Cook workspace。第一次没有可复用状态时执行 full，成功后保存状态；不能把一次新的临时目录标记为可复用。

5. 配置变化

   已保存 profile 运行前重新计算项目设置摘要。摘要变化时，`plan/check/project` 报 `package_profile_stale` 并要求 `package configure --reason ...` 显式接纳变化。普通代码提交不作废 profile；项目设置、目标、引擎和插件来源变化会影响快照或来源摘要。

6. 诊断

   执行记录保存项目设置摘要、UDF 覆盖、最终 argv、Cook 模式、复用判定、源摘要和实际 UAT 退出码。日志解析把 UE warning 单列，不把未知自定义版本、EditorDomain cycle、Shader 编译回退误报成失败。

## 运行场景

| 场景 | 行为 |
| --- | --- |
| 首次配置 | 读取项目基线，应用用户明确选项，保存固定 profile 和设置摘要 |
| 默认重新打包 | 使用当前固定 profile，默认 full Cook，生成新的执行快照 |
| 项目设置已变 | 阻止执行，要求带 reason 的 configure 更新 profile |
| 高级 iterate 且摘要匹配 | 复用 profile 专属 Cook workspace，传 `-iterate` |
| 高级 iterate 但没有缓存或摘要漂移 | 自动回退 full，输出回退原因，不伪造复用 |
| 显式禁用 MCP | 只修改隔离副本 `.uproject` 的 Enabled，不改开发项目 |

## 接口收敛

不新增 `config show`、`retry`、`resume`、自由 `uat_args` 或逐项包装 UE 设置的 CLI。`plan` 是配置和有效 argv 的唯一人类可读查询入口；高级 `--file` 承载 `cook.mode` 和不适合日常命令的类型化字段。常用命令仍然是 `configure`、`plan`、`check`、`project`、`status`、`clean`、`recover`。

## 影响面和边界

生产代码主要修改 `src/package_profile.rs`、`src/project_packaging.rs`、`src/ue_commands.rs`、`src/commands/package.rs`、`src/execution.rs` 和对应 package 测试。旧 profile 没有项目快照时迁移为 legacy，只允许原有行为并在 plan 中提示；新 profile 必须有快照。首批继续限定 UE 5.5、Windows、Win64 Game。

不修改 UGA、AesWorld、UE 安装或 DEV Junction。工具只在隔离副本中修改插件启用状态；最终输出继续使用独立命名目录。
