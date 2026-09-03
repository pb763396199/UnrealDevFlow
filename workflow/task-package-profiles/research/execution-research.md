---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1B7ZQW49AZ2AA11QV6BZ1NE
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-08-31T06:28:00Z
producer: aes-research
result: complete
topic: 状态与诊断
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts: []
---

# 执行状态与失败诊断调查

## 问题与停止条件

明确已有记录能复用什么，以及中断后的状态如何可信。日期 2026-08-31，基线 07b7c4b。

## 看过的地方

| 来源 | 事实 |
| --- | --- |
| src/execution.rs:36、56 | 已有 Running、Unknown 和步骤起止时间、退出码 |
| src/commands/package.rs:316、333、362 | 执行前未保存 running，失败 exit_code=None |
| src/commands/package.rs:200 | execution_id 仅精确到秒 |
| src/commands/switch.rs | 现有切换检查没有打包执行租约 |
| reference/reference.md | 监控退出码误用、日志轮转、错误去重与旧制品问题 |

## 查到的

RunUAT 子进程由 Command::status 等待，stdout/stderr 已写执行日志。
因此缺口是启动前可恢复记录和结构化状态，不能称为完全没有日志或退出码检查。
秒级 ID 存在并发碰撞风险。公共 ExecutionPlan 已经比 PackageResult 更完整。

## 推断及设计输入

采用共享 ExecutionPlan 与 SourceContext；执行启动前落盘配置快照、来源清单及 running。
ID 生成要以独占创建防碰撞。执行进程记录 PID 与开始时间，避免 PID 复用。
命令行返回后台 execution ID 后，专门的本地执行进程负责等待子进程、写状态及日志，不建常驻服务。

日志捕获与解析分离。UAT 总退出码、失败阶段、原始日志行号及分组错误同时保存。
Cook 依赖按 source/target/object 去重并另给 source/target 边数量。
Shader 回退告警单列，支持配置质量门槛，但不能篡改实际 UAT 结果。
新旧执行对比配置、源码、外部数据标识和错误集合；相同错误重试前说明变化依据。

锁分别覆盖任务打包目录、固定交付目录和 UAT 的共享写入；UBT mutex 仍按引擎规则处理。
switch/delete/cleanup 查询对应输入租约。副本完成后不再依赖主项目链接，
共享内容仍需检查来源变化并如实声明外部写入者不受锁约束。

中断后先判断真实进程和执行记录。无法确认结果记 unknown，不从 BUILD SUCCESSFUL 字串猜成功。
retry 创建新 execution 并引用前次，配置默认取前次快照；不能复用不匹配来源的 Cook/Stage。
恢复交付使用恢复日志，不能重新猜一次删除清单。

## 未知与风险

进程树随终端退出的行为、Job Object 及共享 UAT 输出竞争需 Windows 实测。
插件独立打包已有 NoMutex 约定，本轮不修改该策略，不将它套用到项目打包。
