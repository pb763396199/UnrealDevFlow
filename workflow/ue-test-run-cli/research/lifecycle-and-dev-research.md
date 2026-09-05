---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1QM979QM0FKYH0C0M7NXSDJ
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T01:52:43Z
producer: aes-research
result: complete
topic: "退出崩溃案例与 v0.5.0 运行设计基础"
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts: []
---

# 退出崩溃与最新 UDF 的事实

## 问题与范围

一个命令已经返回正确数据，编辑器随后退出时崩溃，UDF 应该如何给出准确结论？本轮读取待办、原始日志和当前代码，查清前后证据及可复用范围后停止。没有重跑 UE，也没有修 AesWorld。

## 版本核验

2026-09-05 本地 `dev`、`origin/dev` 和 `git ls-remote origin refs/heads/dev` 均指向
`2a516b50dfb8b3d58e5a7962b7c0705c84ba78ce`。安装入口为
`C:\Users\YUMEI\.unrealdevflow\bin\udf.exe`，实际输出
`udf 0.5.0 (git 2a516b50d, built 2026-09-03T08:47:33Z)`。

相对旧基线 `07b7c4b`，`dev` 一侧新增 32 个提交；比较两端有 64 个变化文件，其中生产源码 16 个、Rust 测试文件 8 个。旧分支仍有一个不在 `dev` 历史中的提交，因此保留为 `feature/ue-test-run-cli-initial-design`。AES 工具从当前 `dev` 新建同名任务 worktree，旧设计按原 ID 复制保留。

复算命令：

```powershell
git rev-list --count 07b7c4b..2a516b5
git diff --name-only 07b7c4b 2a516b5
git ls-remote origin refs/heads/dev
udf --version
```

## 案例文件位置

| 简称 | 已读取的真实绝对目录 |
| --- | --- |
| CASE | `F:\ShanghaiP4\neon\Hosts\W-neon-dev\T-earthmodeler-exit-crash_Host\Plugins\AesWorld\workflow\earthmodeler-exit-crash` |
| LOG | `C:\Users\YUMEI\AppData\Local\Temp\claude\F--ShanghaiP4-neon-Plugins-AesWorld\dac1de11-e901-4324-86f6-856b03886093\scratchpad` |
| ORIGINAL | `C:\Users\YUMEI\AppData\Local\Temp\claude\F--ShanghaiP4-neon-Plugins-AesWorld\eb84530e-2ace-4b8a-ad7d-22513b8653de\scratchpad` |
| SESSION | `C:\Users\YUMEI\.claude\projects\f--ShanghaiP4-neon-Plugins-AesWorld\dac1de11-e901-4324-86f6-856b03886093.jsonl` |

待办原文位于主检出 `workflow/todo/exit-crash-regression-verification.md`，ID 为 `td_01M1QK547DGFJPA89ZBWXTAP8P`。下列行号均针对上述文件，来源会话中的命令仅用作证据。

## 实际结果

| 场景 | 已核实证据 | 支持的结论 |
| --- | --- | --- |
| 原业务命令 | `ORIGINAL/get-editor-1.log:1332` 执行 `AesWorld.Settings.Get Tier=Low TerrainSettings.MaxLevel`；1333 返回 `Type=int32 Value=20`；1334 执行 `QUIT_EDITOR`；1345 Critical error；1356 OnPreExit；1370 状态 3 | 业务读取成功，退出失败 |
| Host 修复前 | `LOG/baseline-crash-before-fix.log:28` 完整项目参数；1413 `QUIT_EDITOR`；1423 Critical error；1427 access violation；1429 至 1438 调用链；1448 `RequestExitWithStatus(1,3,...)`；`SESSION:254` 返回 `EXITCODE=3` | 真实 UE 5.5.4 Editor 退出崩溃 |
| Host 修复后 | `LOG/after-fix-clean-exit.log:1390` `QUIT_EDITOR`；1499 Editor shut down；1503 Object subsystem closed；1526 Exiting；1527 log closed；`SESSION:463` 返回 `EXITCODE=0` | 此次 Host 正常退出 |
| 修复后内容诊断 | after 日志仍有 74 行 `Error:`，1407 至 1437 有缺失 WdpCamera 的 Blueprint 错误；stdout 还有 Invalid socket handle | 正常退出不能证明内容没有错误 |
| 主项目修复前 | `ORIGINAL/editor-earth-1.log:29` 指向 `UGA/DEV/UGA.uproject`；1484、1493 加载真实 `aes6_sh_sz_q1`；1593 至 1612 为 Settings.Get/Set 结果；1613 退出；1625、1629、1636 崩溃 | 主项目也曾复现，但本轮没找到主项目修复后证据 |
| 负向业务用例 | 主项目日志 1611 至 1612 的 `NoSuchField` 错误用 Warning 输出；1608 的 Set 为 `Saved=0` | 必须断言具体业务结果；日志级别不能替代断言，读取成功也不等于保存成功 |

两次 Host 命令见 `CASE/debug.md:34` 和 `SESSION:250/462`：

```text
UnrealEditor-Cmd.exe <T_earthmodeler_exit_crash_Host.uproject>
-ExecCmds="stat unit, QUIT_EDITOR" -unattended -nopause -nosplash -nullrhi -log
```

两次引擎均为 `5.5.4-40574608+++UE5+Release-5.5`，世界都是 `/Temp/Untitled_1`。
此案例故意不依赖业务关卡；空地图不应被统一当成错误。
源码记录分别关联 `d8a3ebb5cf3a851f405d275a990b84dd6de798f9` 与
`e70b7bc52dd402b89a582b24a58511b4b4e2a2a9`，见 `CASE/validation.md:24/39`。
原日志未记录加载 DLL 的内容摘要，不能据此证明二进制与 commit 一一对应。

| 原日志 | SHA-256 |
| --- | --- |
| baseline-crash-before-fix.log | `5C11342A30D22258E9F81CF2B71F735E03CFD01D4407456C071DF95D0FE5ED0C` |
| after-fix-clean-exit.log | `3B40853C8C076102E0A77223F98F9A97DA9431E552B865332B641989C43AAF5F` |

还读取了 Host `Saved/Crashes/UECC-Windows-41AC8AD542845E2A3C1FB091DE75FF05_0000/CrashContext.runtime-xml`：
第 10 行是 CrashType，第 14 行 PID 为 67428，第 32 行是 `CommandLineRemoved`，同目录有 minidump。
因此 CrashContext 可能没有完整命令行；正常退出也不会产生一份“无崩溃报告”。

## Automation 的覆盖边界

本机 `C:\Program Files\Epic Games\UE_5.5\Engine\Source` 下：

- `Runtime/Launch/Private/Launch.cpp:188`：退出请求后停止主循环。
- `Runtime/Launch/Private/LaunchEngineLoop.cpp:6042/6054`：Automation controller/worker 由 Tick 驱动。
- 同文件 5064 调用 `GEngine->PreExit()`，5189 才统一卸载模块。
- `Editor/UnrealEd/Private/EditorEngine.cpp:1586/1589`：先清 EditorSubsystemCollection，再进入父类 PreExit。
- `Runtime/Engine/Private/UnrealEngine.cpp:2351`：广播 OnEnginePreExit。

待办中“该窗口 Automation 已卸载”过于绝对。进程内测试可以准备状态并请求退出，完整退出结果需要由进程外观察者判断。Automation 报告通过不能替代后续进程退出验证。

[Epic Crash Reporting](https://dev.epicgames.com/documentation/unreal-engine/crash-reporting-in-unreal-engine?lang=en-US)
区分 Crash、Assert 与非致命 Ensure。因此有调用栈或 CrashReportClient 不等于发生致命崩溃。
官方 [Gauntlet](https://dev.epicgames.com/documentation/en-us/unreal-engine/running-gauntlet-tests-in-unreal-engine)
已有 BootTest 和 EditorBootTest，可作为后续适配器；本轮不声称 v0.5.0 已接入这些能力。
网页查阅时间为 2026-09-05，公开页面当前为 5.8，具体 5.5.4 时序以上述本机源码为准。

## 当前 UDF 能复用什么

| 代码位置（2a516b5） | 已有事实 | 设计边界 |
| --- | --- | --- |
| `src/cli.rs:275`、`tests/cli_taxonomy.rs:139` | `package configure` 固定配置；`package run` 已删除；高级制品显式进入 advanced | run 采用 configure/check/plan，旧 profile list/show/run 不沿用 |
| `src/package_profile.rs:112/320/560/581/693` | 来源身份、配置 revision、摘要、备份替换、原因和原生 INI 漂移 | 提炼这些机制；Cook/container 字段留给 package |
| `src/project_packaging.rs:50/77` | 收集部分 INI 与 GameDefaultMap/MapsToCook | 尚无 EditorStartupMap，更没有完整 UE 配置解释器 |
| `src/commands/package.rs:1337/1382/1627` | 主项目输入的隔离 staging 与 task 插件覆盖 | 可以提炼隔离项目准备；运行目录、可写数据、准备成本需重新约定 |
| `src/commands/package.rs:57/1593` | 快速来源指纹采用路径、大小、mtime 与 Junction | 用于失效检查，不能冒充完整内容或加载 DLL 的证明 |
| `src/ue_commands.rs:102` | executable 与 arguments 分离 | 复用模型，仍需验证 Windows 到 UE 自有解析器的引号语义 |
| `src/execution.rs:14/205` | 共享类型 | 未接成通用运行器 |
| `src/commands/package.rs:278/916/966` | 私有 PackageResult、直接写 execution 文件、同步等待退出 | 还没有通用原子事件记录、监督进程、超时或取消 |
| `src/commands/build_status.rs:165` | PID 消失且无结果时为 unknown | 保留此语义；PID 身份需加创建时间与可执行文件 |
| `src/commands/package.rs:2472` | recover 恢复打包交付事务 | 不等于恢复 UE 会话或重跑测试 |

## 设计必须回答的问题

1. 报告生成后继续观测退出，区分业务结果、退出结果和内容诊断。
2. 监督进程持有真实进程句柄。观察者丢失、日志不完整或进程被强杀时不判通过。
3. 前后对照固定场景和断言，明确允许变化的插件版本，缺陷签名必须先在基线复现。
4. DetailsView/Actor 选择等触发条件未被准备或证实时，要报覆盖不足。
5. 独立日志与制品目录需绑定 execution；正常退出和进程级错误码分开，状态 3 不替换为异常码 `0xC0000005`。
6. Host 无 RHI 的结果只覆盖 Host 退出，不能替代主项目或图形测试。

## 证据缺口

此次历史材料没有每次运行的 DLL 摘要、完整布局/选择状态快照和足够的重复运行统计。
主项目缺少修复后样本。新 CLI 应补齐这些字段；对旧记录导入时显示缺失，不能补写一份假定完整的修复证明。
