# 用户要求与打包事故证据

## 已确认的要求

| 要求 | 处理方式 |
| --- | --- |
| MCP 插件无法打包，用户要求关闭 | 支持配置级排除插件；不把这次操作记作 Agent 擅自修改 |
| C:\Package\Windows 是用户手动对照实验 | 用来对比参数和结果；不记作 AI 交付路径错误 |
| task 首次需要打包时固定配置 | 后续复用；用户或 AI 判断过时后才更新，保留原因和差异 |
| 没有需要讨论的即可写设计 | 允许完成调查后进入 aes-brainstorm 写 proposed 设计；不表示允许实施或已通过设计评审 |

## 来源

- 会话 A：claude:a915ac74-ad25-424e-a863-4c9ceb13e12f。
- 原文：C:\Users\YUMEI\.claude\projects\f--ShanghaiP4-neon-Plugins-AesWorld\a915ac74-ad25-424e-a863-4c9ceb13e12f.jsonl。
- 本轮取证基于截至约 2528 行的记录。历史中的执行授权不带入本任务。
- 工具基线：07b7c4b72b2612db3ad0a12ac05fcf89985a8e08，Cargo.toml 版本 0.4.1。
- 引擎证据：C:\Program Files\Epic Games\UE_5.5，5.5.4。查询日期：2026-08-31。
- 以下日志目录为 F:\ShanghaiP4\neon\UGA\DEV\Saved\Logs。

## 实测尝试

| 尝试 | 证据文件或会话行 | 结果 |
| --- | --- | --- |
| 首轮 BuildPlugin | JSONL 1375、1384、1391 | Game Development 暴露 UCLASS 宏及缺少 UScriptStruct 定义 |
| 拆模块后补编译 | JSONL 1469、1473 | 4 个 LNK2019，随后补导出宏 |
| 第二轮 BuildPlugin | JSONL 1522、1531、1536 | 三目标编译成功，31 分 53 秒 |
| 首次项目打包 | dev_buildcookrun_20260828_runtime_panel.log | 25 分 47 秒，34 errors，退出 25 |
| 切回任务后重试 | dev_buildcookrun_20260828_runtime_panel_retry.log | 21 分 53 秒，34 errors，退出 25 |
| 去掉 allmaps | dev_buildcookrun_20260828_no_allmaps.log | 96 分 28 秒，37 errors，退出 25 |
| 用户手动打包 | UGA-backup-2026.08.31-05.45.18.log:1640、1946、16695、16696 | 独立 Cook commandlet、FULL COOK、223 秒、退出 0 |

三次失败总计 8648 秒。首两次依赖去重为 11 个源包、4 个目标包、17 条边；第三次为 11、5、18。
对 Source package 与随后 Target package 成对提取，再按源和目标去重，不能数日志中重复打印的 error 行代替依赖数量。

## 问题与更正

| 问题 | 证据与边界 |
| --- | --- |
| skipcookingeditorcontent 排除被引用资源 | CookCommand.Automation.cs:60 转成 skipeditorcontent；CookOnTheFlyServer.cpp:6458 拒绝保存 Engine Editor 路径资源 |
| 资源并非永远不可 Cook | 成功日志 2097、10332、10430、10498 行记录四种 Engine 目标资源均被 Cook |
| FULL COOK 被误判成增量 | 成功日志 1946 行明确否定；不能从运行时间猜 Cook 模式 |
| Shader GetFeatureID 未定义 | 成功日志 12051 至 12054 行也存在并回退默认材质；属于质量警告，不能用来解释 UAT 退出 25 |
| 第三次新增 T_Point | 失败日志 358 行先报找不到包，后出现 M_EarthTerrain_V2 对它的引用；具体原因未知 |
| 第三次新增 Base_Bridge001 | 失败日志 9922 行 package summary invalid；不能只改参数就宣称已解决 |
| 来源变化 | JSONL 2152 当时的 Junction 指向 nanite 任务；切换时点与操作者未经证明 |
| 后台监控丢失 | JSONL 2106 至 2117；监控命令退出 0 不能替代 UAT 的退出码 |
| 旧 exe 存留 | JSONL 2238，仍为 8 月 26 日；存在文件不证明本次成功 |
| 临时排除 MCP | JSONL 2039、2184，用户明确要求；恢复必须避免覆盖并发修改 |
| 固定交付及外部数据 | JSONL 1929、2039，目标为 AesEarthDevStageCurrent_20260819，保留已有 _AesEarthData_ Junction |
| 打包后 RoadClasses 展开缺陷 | JSONL 2473；属于业务运行时问题，来源片段不足以确认最终修复 |
| 图标分发与 include | JSONL 119、216 是早期实现及历史总结；不能当成本轮新增失败或泛化为所有图标必须 NonUFS |
| 工具环境告警 | JSONL 2176 含 MSVC 非首选版本、Installed Engine 不支持 Program target、可选 Insights 程序缺失 |
| 入口与配方缺失 | 会话手拼 BuildCookRun；当前 package CLI 没有配置/平台/输出选项，技能缺少 package 指南 |

成功命令也通过 RunUAT 执行。其 target 为 UGA，启用 pak/archive/package，输出 C:/Package，
没有 skipcookingeditorcontent 或 allmaps。不要照抄 EditorIOPort、自动更新 SDK 等会话临时参数。
本轮没有重跑完整 Cook，不能声称单参数修改已在所有失败场景中实测通过。
