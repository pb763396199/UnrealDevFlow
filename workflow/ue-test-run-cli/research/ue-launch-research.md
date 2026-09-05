---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M13QAFD65F1AQ966G9Y67WPX
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-08-28T08:26:35.664Z
producer: aes-research
result: complete
topic: "UDF 能否可靠解析虚幻项目测试启动命令"
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts: []
---

# 虚幻项目测试启动命令调查

## 调查问题

UDF 现有的 workspace、task 和执行记录能否替代手工拼接，稳定确定 UE 可执行文件、
`.uproject`、关卡、运行方式、测试目标和完成证据？调查在找到真实失败样本、当前可复用代码、
Epic 官方命令契约和仍需实现时验证的未知项后停止。

## 看过的材料

| 材料 | 查阅时间 | 适用范围 |
| --- | --- | --- |
| Claude Code 会话 `5774c354-0333-4e12-9244-fcac041f0d1a` 的 JSONL 日志 | 2026-08-28 | Windows、Git Bash、UE 5.5、AesWorld 真机与自动化测试 |
| `src/cli.rs`、`src/main.rs`、`src/config.rs`、`src/host/mod.rs`、`src/source_context.rs`、`src/execution.rs`、`src/editor.rs`、`src/ue_commands.rs` | 2026-08-28，版本 `07b7c4b` | 当前 UDF `dev` 分支 |
| [Command-Line Arguments](https://dev.epicgames.com/documentation/unreal-engine/command-line-arguments-in-unreal-engine) | 2026-08-28 | Epic 当前公开文档，通用 UE 5 命令格式 |
| [Running Unreal Engine](https://dev.epicgames.com/documentation/unreal-engine/running-unreal-engine) | 2026-08-28 | 编辑器、未烘焙游戏和启动关卡 |
| [Run Automation Tests](https://dev.epicgames.com/documentation/unreal-engine/run-automation-tests-in-unreal-engine) | 2026-08-28 | Automation 命令行和 Session Frontend |
| [UCommandlet](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/UCommandlet) | 2026-08-28 | Commandlet 的运行环境 |
| `uwf kb read "Unreal Editor 命令行启动与自动化测试" --scope engine/5-5-4 --json` | 2026-08-28 | 引擎 5.5.4 本机知识库，零命中 |

## 来源直接支持的事实

### 会话 A 的失败样本

| 日志位置 | 原命令或行为 | 可见结果 | 修正证据 |
| --- | --- | --- | --- |
| JSONL 第 362、380 行 | `UnrealEditor.exe UGA.uproject` 后直接执行 Python，没有关卡参数 | 采集世界为 `/Temp/Untitled_0.Untitled`，Full 阶段 `valid=0` | 第 393 行把 `/Game/Maps/UGA_local/aes6_sh_sz_q1` 加到启动参数 |
| JSONL 第 18934、18965 行 | Git Bash 直接传 `/Game/Maps/UGA_local/aes6_sh_sz_q1` | 引擎收到错误路径，运行提前退出 | 第 18966 行加 `MSYS_NO_PATHCONV=1`，第 18975 行日志显示正确 `/Game/...` |
| JSONL 第 23514、23547、23551 行 | 8 月 28 日再次漏掉 `MSYS_NO_PATHCONV=1` | 用户看到黑屏和找不到地图；`Browse` 实际收到 `C:/Program Files/Git/Game/...` | 第 23575 行重启；第 23590 行 `Browse Started` 收到正确 `/Game/...` |
| JSONL 第 625、18117 行 | Automation 有时使用主项目 `UGA.uproject`，有时使用任务 Host `.uproject` | 两条命令的插件和项目内容来源不同，手工命令没有说明选择依据 | UDF 当前 `SourceContext` 已能区分 `workspace` 与 `task`，仍把运行项目和代码来源放在同一对象里 |
| JSONL 第 23483、23488 行 | 启动真机采集前机器已有一个 `UnrealEditor.exe` | 现有实例占用约 11 GB 内存，命令行指向另一份 `UGA.uproject` | 会话临时查进程后才决定是否继续，UDF 只在 `task switch` 前做粗粒度 Editor 检查 |
| JSONL 第 23730 行附近的会话复盘 | `-game -fullscreen` 没有指定 `-ResX/-ResY` | 新运行使用 1920×1080，历史基线是 1366×1024，像素数相差约 48% | 后续模板要求性能对比显式固定窗口模式和分辨率 |

同一个 Git Bash 关卡转换错误在三天内重复出现两次。文档里记住一条环境变量不足以防止复发，
启动器需要直接用参数数组创建子进程，并在启动后核对引擎日志里的实际 `Browse` 目标。

### Epic 官方命令契约

| 事实 | 官方依据 | 对设计的限制 |
| --- | --- | --- |
| 通用格式是 `<EXECUTABLE> [URL_PARAMETERS] [ARGUMENTS]` | Command-Line Arguments | 关卡属于 URL 参数，不能和普通 `-Flag` 混成一段自由文本 |
| 未烘焙游戏需要 `.uproject` 和 `-game`，启动关卡可以跟在项目后面 | Running Unreal Engine | `editor` 与 `game` 要做成不同动作，不能靠调用方自己补 `-game` |
| `-ExecCmds="Automation RunTest ...;Quit"` 可以从命令行运行 Automation | Run Automation Tests | 测试过滤条件要单独建模；UDF 生成 `ExecCmds`，不接收整条 shell 命令 |
| Automation 也可以在已运行的 Editor 或 Client 上由 Session Frontend调度 | Run Automation Tests | 已有会话可以列为一种情况；官方材料没有给任意控制台命令注入接口 |
| Commandlet 环境不加载游戏、客户端、关卡和 Actor | UCommandlet | 需要世界或渲染的任务不能降级成 Commandlet |

官方当前页面使用 `Automation RunTest`。来源会话一直使用 `Automation RunTests`，且 UE 5.5 日志
显示测试能够运行。UDF 实现前要对目标引擎执行一次发现探针，记录该版本接受的命令拼写；设计不把
会话里的拼写当作跨版本常量。

Epic 官方把关卡 URL 作为启动参数。来源会话也用 `MAP LOAD FILE=...` 绕过过 Git Bash 转换。
两条路都能加载关卡，启动参数能让 UE 从第一帧就进入目标世界；`MAP LOAD` 需要额外处理命令执行顺序。
UDF 直接创建子进程后不再经过 Git Bash，所以设计采用官方的关卡 URL 位置，并保留日志核对。

### 当前 UDF 可复用部分

| 能力 | 代码入口 | 结论 |
| --- | --- | --- |
| 顶层命令和全局 JSON 输出 | `src/cli.rs:106`、`src/output.rs:21` | 当前有 `workspace`、`task`、`build`、`package`、`skill` 五组，新命令必须走统一输出信封 |
| workspace 唯一解析 | `src/config.rs:174` | 多 workspace 时已经拒绝猜测 |
| task、Host 和 `.udf-meta.json` | `src/host/mod.rs:106`、`src/host/mod.rs:489` | 能冻结任务代码来源、主项目、引擎和插件版本 |
| 规范 Host `.uproject` | `src/host/mod.rs:206` | 多个 Host 项目时已经拒绝取第一个 |
| 共享执行契约 | `src/execution.rs:146`、`src/execution.rs:167`、`src/execution.rs:203` | `check`、`plan`、执行记录和 `status` 可以扩展到运行与测试 |
| UE 命令参数数组 | `src/ue_commands.rs:81` | 可以绕开 Git Bash 对子进程参数的再次解释 |
| Editor 进程检测 | `src/editor.rs:6` | 目前只知道有没有进程，需要补 PID、项目路径和命令行归属 |

当前 UDF 没有关卡枚举、Automation 目标发现、`UnrealEditor.exe` 路径解析、运行日志证据和已有会话复用策略。

## 由证据推得的设计要求

1. 代码来源和运行项目要分开记录。任务 Host 适合插件自动化，主项目才拥有真实业务关卡；
   `--task` 不能暗中决定使用哪一份 `.uproject`。
2. UDF 应接收关卡、测试过滤条件和控制台命令的结构化参数，再用参数数组启动 UE。
   调用方不再提供一整条 shell 字符串。
3. 启动前的存在性检查只能证明文件或测试候选存在。启动后的日志还要证明实际项目、实际关卡、
   测试数量和完成状态符合计划。
4. 自定义控制台命令是否完成无法由 UDF 猜出。调用方或预设要声明日志、文件、进程退出或人工查看中的一种完成证据。
5. 已有 Editor 会话缺少通用的官方命令注入接口。第一版应返回可复制的控制台命令和匹配的 PID，
   只有明确注册了桥接能力的会话才允许自动发送。
6. 真机性能运行要把窗口模式、分辨率和并发 UE 进程写入计划与结果。缺少这些字段的两次运行不能自动比较。

## 仍未知的内容

| 未知项 | 为什么现在不能下结论 | 实现前怎么查 |
| --- | --- | --- |
| UE 5.5 对 `Automation RunTest` 与 `RunTests` 的精确兼容关系 | 官方当前文档和来源会话不同 | 用目标引擎列举并运行一个无副作用测试，保存命令和日志 |
| 离线扫描 `.umap` 能否覆盖所有插件挂载名、重定向和虚拟资产 | 文件路径不等于最终 long package name | 先做文件扫描，遇到重定向或多个候选时启动只读资产发现探针 |
| 任意已运行 Editor 的稳定命令注入方式 | Session Frontend 文档只覆盖 Automation 调度 | 第一版只检测和给人工命令；另开设计评估 MCP、Remote Control 或自带桥接插件 |
| `-TestExit` 在 UE 5.5 到后续版本的稳定性 | 当前官方 Automation 页面没有把它列为必需参数 | 把它放进版本适配层，用目标引擎探针和回归测试决定是否添加 |

## 交给设计的结论

设计应新增一个统一的 `run` 命令组，复用现有 `check`、`plan`、执行记录和 `status`。
每次计划必须给出以下内容：

- 代码来源与运行项目。
- 引擎、可执行文件、关卡和模式。
- 参数来源、冲突进程和完成证据。
关卡、测试、运行项目或现有会话有歧义时返回候选，不启动进程。
