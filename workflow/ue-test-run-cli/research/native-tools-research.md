---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1QPYQE0DJC6XBFBKJQ601FE
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T02:39:24Z
producer: aes-research
result: complete
topic: "UE 原生工具能覆盖哪些启动与退出测试需求"
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts: []
---

# 原生工具复用依据

## 问题与结论

2026-09-05 核查本机 UE 5.5.4 源码及 Epic 5.5 文档。Editor CLI、UAT、Gauntlet 已覆盖启动与测试执行的主体工作。UDF 需要补任务上下文、可复用配置和结果入口；当前没有证据支持重写通用测试运行器。

本机源码根为 `C:\Program Files\Epic Games\UE_5.5\Engine\Source\Programs\AutomationTool\Gauntlet`。
本轮只读源码与文档，没有在目标 Host 运行 Gauntlet，因此下列事实分开标注原生已有和待实测。

## 已核实的原生能力

| 能力 | 本机证据 | 对设计的决定 |
| --- | --- | --- |
| Editor 启动后退出 | `Unreal/Automation/UE.BootTest.cs:195/212` 的 UE.EditorBootTest 配置 Editor 角色并加 QUIT_EDITOR | 优先用它验证现成 Host；不从自建监督器起步 |
| Editor Automation 调度 | `Unreal/Automation/UE.Automation.cs:92/167/379` | 用 UE.EditorAutomation；RunTest 必填，测试清单和报告沿用原生系统 |
| 退出、Fatal 与 Ensure 判断 | `Unreal/Base/Gauntlet.UnrealTestNode.cs:1666` | 复用原生判定，可覆写项目专用规则 |
| 通用崩溃日志解析 | `Unreal/Utils/Gauntlet.UnrealLogParser.cs:1002/1018` | 不在 Rust 重写一套 crash/ensure 正则库 |
| 运行参数 | `Unreal/RunUnreal.cs:27` 起声明 ExecCmds、NullRHI、ResX/ResY、MaxDuration 和 TestIterations | 配置映射原生参数，版本适配必须经过测试 |
| UDF 配置与只读查询 | `src/cli.rs:19/20/277`、`src/package_profile.rs:112/140/581`，基线 2a516b5 | 复用 configure/check/plan 的语言与来源解析 |

官方 [Gauntlet 5.5 概述](https://dev.epicgames.com/documentation/en-us/unreal-engine/gauntlet-automation-framework-overview-in-unreal-engine?application_version=5.5) 提供进程、会话、测试执行及日志处理能力，不要求项目一律加运行时控制插件。
官方 [运行 Gauntlet](https://dev.epicgames.com/documentation/en-us/unreal-engine/running-gauntlet-tests-in-unreal-engine?application_version=5.5) 列出 EditorBootTest 与 EditorAutomation。
[Project Launcher](https://dev.epicgames.com/documentation/en-us/unreal-engine/using-the-project-launcher-in-unreal-engine?application_version=5.5) 已能保存原生启动配置。UDF 不需要另造完整 Launcher 配置体系。

## 正常退出的最小差异

Gauntlet 5.5 的 `GetExitCodeAndReason` 在 1694 行判断 WasKilled，部分非超时的受控终止在 1705 行返回 ExitOk。
1719 行根据 RequestedExit 判断，1723 行也可能给出归一化的成功码。因此 Gauntlet 的归一化 ExitCode 不能直接当作原始 UE 进程退出码。

对于“必须自然退出”的测试，应在 Gauntlet 测试节点内补最小规则：已自然终止、WasKilled=false、原始 UE 退出码为 0，且原生判定无致命失败才通过。原始码非零为 failed，缺失为 unknown；已知 Fatal 或强杀仍为 failed。不能只保存原始码却继续采用归一化成功结果。
这个测试节点可以服务多个项目，但它依然由 Gauntlet 调度，不能再演化成独立生命周期框架。

EarthModeler 的 stat unit + QUIT_EDITOR 案例已经有真实 Host、命令和前后日志，位置见 [前一份调查](lifecycle-and-dev-research.md)。
下一步实施探针先运行原生 EditorBootTest，然后确认退出模式与原命令是否等价。若需要补 stat unit 或严格退出策略，使用最小节点扩展。
Details 状态只有在复现该具体缺陷确实需要时才准备；缺少完整布局快照不能挡住基础退出测试。

## 仍需实施时验证

1. 本机 RunUAT 能否加载 Gauntlet，以及 EditorBootTest 在目标 Host 是否走到同一退出路径。
2. 最小节点程序集的加载路径、日志制品输出与原始 UE 退出状态的取值接口。
3. 从 PowerShell、cmd、Git Bash 调用 UDF 后，最终 UE 收到的关卡及嵌套 ExecCmds 是否正确。
4. 已有项目测试能否提供业务结果；不能时返回仅执行结果，不伪造业务通过。

## 收窄的理由

原始问题是会话重复猜命令与目标。完整输入快照、DLL 哈希门禁和可比性证明，不能直接减少这类错误。
必要上下文是实际工程、引擎、关卡要求、执行参数与原生结果；性能分辨率等前提仅放在相应测试配置中。
前后对照保存并展示差异即可，结论限定为两次观察结果，不升级为自动因果证明。
