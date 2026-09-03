---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1GCKPYPHYGFPR60YHRY9K99
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-02T03:00:00Z
producer: aes-research
result: complete
topic: 会话中的项目包范围、非项目包入口和产物生命周期
supersedes: null
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts: []
---

# 项目包范围调查

## 调查对象

调查了用户提供的 Codex 会话记录、该会话关联的 UDF execution 记录，以及当前任务分支中的 CLI、设计和文档。会话原始任务是“调研运行时属性覆盖范围”，内容包括运行时设置面板、材质属性、Geo Referencing 和 Data Descriptor，没有项目包、插件包或引擎 Installed Build 的用户要求。

## 直接证据

| 证据 | 结果 |
| --- | --- |
| 会话原始用户请求 | 目标是运行时属性覆盖范围和真实 UE 验证，没有引擎包或插件包要求 |
| 会话中的用户消息 | 明确出现“赶紧打啊”“打出来的包根本没有新字段”“不要每次都完整 Cook，加快时间”，没有要求打插件包或引擎包 |
| 关联 execution 记录 | 18 次真实打包：项目包 11 次、插件包 4 次、引擎包 3 次 |
| 项目包结果 | 4 次成功，4 次失败，3 次中断或清理；失败包括 DEV 断链、UnrealGame 编译错误和一次未收口的任务包流程 |
| 插件包结果 | `package-plugin-20260901T013628Z-194`、`014517Z-833` 处于 running，`014724Z-643` 以退出码 6 清理，`082811Z` 失败 |
| 引擎包结果 | `package-engine-20260901T020205Z-179`、`034239Z-460`、`034359Z-867` 均以退出码 1 结束；日志指向安装版引擎缺少 `Engine/Build/InstalledEngineBuild.xml` |
| 当前 CLI 公开面 | `README.md` 和 `src/cli.rs` 将 `package project`、`package plugin`、`package engine` 作为同级入口，并在示例中同时展示 |
| 旧设计决策 | `design/package-design-v2.md` 明确写了“保留既有 package plugin/engine/run/clean” |

## 为什么会扩展到引擎包和插件包

直接证据能证明入口存在，也能证明用户没有提出这两个目标。会话记录没有保存一条明确的 AI 说明，解释它为何决定启动 `package engine` 或 `package plugin`，因此具体心理动机属于未知。

可以确定的原因是工具和设计没有表达“普通任务只打项目包”。同一组 `package` 命令同时暴露项目、插件和引擎三个目标，旧设计又要求保留这些入口。AI 在执行“验证打包工具”的过程中把同级入口当成了待覆盖范围。这是基于命令表和执行结果的推断，置信度高，但不是会话中的原话。

## 产物和磁盘证据

会话关联的项目包多次使用同一个 `C:\Package\UGA-earth-runtime-settings-property-coverage-Win64-Development` 输出目录，执行记录能看到 full Cook、Pak 和临时 Archive。当前机器现场另有 5 个同一任务族的版本目录，每个约 10.67 GB，`C:\Package` 下合计约 64.04 GB；`C:\Users\YUMEI\AppData\Local\Temp\UDF` 有 6 个目录，合计约 309.33 GB。这些 2026-09-02 的目录不计入提供的 2026-09-01 会话次数，但证明现有生命周期设计会让版本对比和失败重试快速消耗磁盘。

当前 `publish_directory` 会创建已有输出目录并逐文件复制，不会按上一份 manifest 删除已经不存在的新旧文件。当前实现有输出锁和 Junction 保护，但没有输出血缘、空间估算、保留策略或成功后的自动临时目录清理。

## 结论

1. 用户需求边界是项目包，插件包和引擎包不是这次任务的默认验证范围。
2. 非项目包被触发的根因是公开命令分组和设计文本没有建立项目包默认边界，AI 扩展了验证范围。
3. 项目包的固定 profile、full/iterate 和命名设计已经解决了部分复现问题，但没有解决输出版本登记、容量预警、临时目录回收和旧文件残留。
4. 新设计应把项目包设为普通任务唯一入口，把插件包和引擎 Installed Build 移到明确的高级工具链入口；同时把容量和产物生命周期纳入 `plan`、`project`、`status`、`clean` 的同一套模型。

## 未知项

- 会话中 AI 选择引擎包的唯一原始理由没有保留在可读历史里，只能依据入口设计和执行顺序推断。
- 2026-09-02 的 5 个版本目录由哪个后续会话创建，当前提供的 JSONL 没有覆盖；它们只作为现场风险证据，不并入历史次数。
