---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01M1E9CXJEVY3JBVAAKDC35J9N
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-01T10:49:27Z
producer: aes-brainstorm
result: superseded
supersedes: ar_01M1DXZWBW3073G1TNNW48AG5S
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1DY0G9HF11ZFAJKST8W2VAK
      digest: sha256:d0ee41db2a0ed9005e050cedc2b4f12728496a248f8395098f506890becde033
      locator: research/ue-native-packaging-v2-research.md
    - artifact_id: ar_01M1DXZWBW3073G1TNNW48AG5S
      digest: sha256:2ff03c5b2f5640b6a61e816613f16a6ce3f3a11b5d3522f59f89fb9af701a06c
      locator: design/package-design-v3.md
---

# 日常增量与完整发布打包设计 v4

## 目标

让使用者一眼知道当前配置会不会复用 Cook。日常任务默认走增量 Cook，完整发布必须明确选择完整模式；已有固定配置保持原模式，不因工具升级悄悄改变。

## 已确认事实

- `bCookAll=False` 只表示不强制 Cook 所有地图，不表示增量复用。
- `FullRebuild=False` 只表示不强制完整重编译，不控制 Cook 或 Pak。
- `UsePakFile=True` 表示项目要求 Pak，Pak 阶段仍可能重建当前包中的所有条目。
- 增量 Cook 需要 `-iterate` 和可验证的持久化 Cook 状态；项目设置没有提供时，UDF 的任务配置才可以明确表达这个开发策略。

## 方案对比

### 方案 A：把 Cook 模式固定在任务 profile，推荐

新增 `--cook-mode full|iterate`，写入任务的固定 profile。新 profile 默认 `iterate`，发布任务显式传 `full` 并记录原因。执行时只有摘要匹配才传 `-iterate`，否则完整 Cook 并说明回退原因。已有没有该字段的 profile 按原来的 `full` 解释，避免静默改行为。

优点是配置和执行一致，日常命令短，发布不会误用增量缓存。代价是切换发布模式需要一次带原因的 `configure`。

### 方案 B：每次 `package project` 临时传模式

把 `--full` 或 `--iterate` 放在执行命令上，不保存到 profile。它看起来直接，但同一个 task 的两次执行可能使用不同语义，执行记录也容易与固定配置脱节，因此不采用。

## 固定决策

1. `CookMode::Iterate` 是新 profile 的默认值，代表日常任务开发。
2. `CookMode::Full` 是显式发布选择。旧 profile 没有 `[cook]` 时按 full 兼容，升级工具不改已有任务行为。
3. CLI help 必须直接给出两条命令：日常开发用 `--cook-mode iterate`，正式发布用 `--cook-mode full`，并说明 `-iterate` 只影响 Cook，不能保证 Pak 不重建。
4. `plan` 和执行记录都显示 `cookMode`、是否复用及原因；用户不需要猜当前是否在全量 Cook。
5. `--file` 仍是高级候选配置入口，不能和 `--cook-mode` 等常用选项混用。

## 失败和边界

- 没有缓存、来源摘要变化、项目设置变化、引擎变化、平台变化或容器变化时，iterate 自动回退 full。
- Pak 项目即使使用 iterate，内容有变化时仍允许 UAT 重建 Pak；工具不能把 Cook 增量误报为 Pak 增量。
- 只改 C++ 代码但不需要包时，使用 `udf build task`；不启动 package 就不会触发 Cook/Pak。

## 验收

- 新 profile 序列化 `[cook] mode = "iterate"`。
- `package configure --cook-mode full` 写入 full，并要求已有配置提供 `--reason`。
- `package configure --help` 与 `package project --help` 清楚说明两种模式。
- 旧 profile 没有 `[cook]` 时仍按 full，plan 不出现 `-iterate`。
- iterate 有匹配缓存时出现 `-iterate`，没有匹配缓存时出现 full 回退原因。
