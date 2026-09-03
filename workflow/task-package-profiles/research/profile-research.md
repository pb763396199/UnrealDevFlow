---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1B7ZA1ZDR02TGP7GHJ4M0YH
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-08-31T06:28:00Z
producer: aes-research
result: complete
topic: 配置保存与更新
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts: []
---

# 配置保存与更新调查

## 问题与停止条件

确认现有身份与存储是否可复用，以及哪些行为必须新增。调查截至 2026-08-31，源码基线 07b7c4b，适用 UnrealDevFlow 0.4.1。

## 看过的地方

| 来源 | 可核对事实 |
| --- | --- |
| src/config.rs:37、100 | workspace 是本机注册配置，config_dir 支持环境变量隔离 |
| src/host/mod.rs:107 | TaskMeta 保存 created、task_uid、context |
| src/commands/create.rs:373、385 | created 为创建时间，task_uid 仅由 workspace/task-id 组成 |
| src/commands/package.rs:645 | 未从 task 读取打包配置 |
| src/commands/cleanup.rs 与 delete.rs | Host 清理必须纳入配置生命周期考虑 |

## 查到的

task_uid 不是每次创建都不同的随机标识。同名任务删除再建时不能仅按它找旧配置。
当前 package 调用把平台、配置和输出位置写死，Config 也没有任务打包配方字段。

## 推断及设计输入

任务当前配置适合放 Host 根目录的 .udf-package.toml，避免写入任何主插件仓库或复写整个 .udf-meta.json。
绑定 task_uid、created 和规范化 Host 路径，缺失 created 的旧任务不能猜实例，应显式初始化。

配置修订历史及每次执行快照应另存 config_dir 下，任务 Host 被清理后仍能解释旧执行。
当前配置只有一个正本，历史副本不可回写为另一套默认值。配置导入必须重新绑定目标 task。

首次初始化冻结已解析的值和项目 Packaging 设置，不在每次执行时重新继承可变 workspace 默认。
用户或 AI 可以显式更新，记录 reason、旧摘要、新摘要。运行中只使用启动前的快照，不追读 current 文件。

并发更新应比较预期摘要并原子替换。直接手工改文件也要被识别为待导入修改，不能静默丢掉历史。
过时检查分别报告项目/引擎/依赖漂移与普通代码更新；普通提交不等同配方过时。

## 未知与风险

同名迁移、损坏元数据、配置历史写入与 current 替换之间崩溃，需要实现阶段的故障注入。
保存 SHA 类摘要的算法和现有依赖取舍由计划阶段确认，本轮不加依赖。
