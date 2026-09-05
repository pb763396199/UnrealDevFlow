---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01M1QM97EAQG8PYJ6ZQESV3P8S
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T01:52:43Z
producer: aes-brainstorm
result: superseded
supersedes: ar_01M13QAFX4ABBWNZWXC9969XYN
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts:
    - artifact_id: ar_01M1QM979QM0FKYH0C0M7NXSDJ
      digest: sha256:b863e640b361325db8ea437ad4be49e51200ad57f41d049ffe25465d5aa103ba
      locator: research/lifecycle-and-dev-research.md
---

# UDF 运行、退出与回归验证设计 v2

历史设计。用户要求原生工具优先并收窄环境快照范围后，本版由 [v3](run-native-tools-design-v3.md) 取代。

## 结论与范围

UDF 管理一次运行的完整过程：解析目标，准备状态，执行业务动作，请求退出，取得进程结果，收齐证据，再判定每条断言。退出验证成为各类场景可组合的能力，公共入口统一为 `udf run`。

本版基于 `dev=2a516b5`、v0.5.0，取代旧设计。下文所有 `run` 命令和数据结构均为拟新增接口。
本轮交付设计与证据，不实现 CLI，不执行来源案例中的 UE 命令。

| 真实需要 | 旧版不足 | 本版解决方式 |
| --- | --- | --- |
| 选对项目和关卡 | 只列出候选，无法判定同名地图哪个适合任务 | 具名场景固定地图、用途和必要场景状态；候选只辅助选择 |
| 命令已返回 `Value=20`，退出却崩溃 | 报告或业务输出可提前使运行成功 | 业务完成与退出结果分开，最后统一判定 |
| 修复后仍有 74 行 Error | “无报错”会把成功退出误判成失败 | clean-exit、业务断言、内容诊断分别输出 |
| 修复前后各跑一次 | 缺少可比条件和失败签名 | 按同一场景的证据比较，基线未复现不能叫已修复 |
| 主项目内容与任务代码组合 | 只能切主项目 Junction 或运行空 Host | 引入显式 isolated 运行项目，提炼 v0.5.0 的输入隔离机制 |
| 多会话、后台、CI | PID 消失、shell 成功可能被当成 UE 成功 | 每次独立监督进程持有句柄，CI 等待最终判定 |

## 方案取舍

| 方案 | 能力与代价 | 决定 |
| --- | --- | --- |
| 公共运行描述 + 生命周期 + 可组合断言 | 启动、Automation、命令与退出回归共用执行记录；需新增监督与断言模块 | 推荐，完整覆盖两组真实案例 |
| 只增加 `--assert-clean-exit`，给旧完成条件再加 grep | 改动较小，但后台退出码、状态准备、缺陷对照仍需脚本 | 不采用为整体方案；该开关可作为场景配置的快捷写法 |
| 所有流程强制经 Gauntlet | 可复用进程测试框架，增加 UAT/测试程序集准备成本 | 留作适配器；首版 Windows 单进程验证不强制引入 |
| 维持手工命令 | 不增加代码，继续依赖每个会话判断 | 不采用，已有重复路径错误与退出误判案例 |

本版收敛旧版的五个动作入口与 profile 子树。运行业务类型存进同一个描述，配置和执行入口各保留一个。当前用户还未接受旧版，因此不需要发布兼容别名。

## 公共 CLI

```text
udf run configure --task TASK --scenario NAME --file FILE [--reason TEXT]
udf run discover maps|tests|sessions --workspace W|--task TASK [--project main|host|isolated] [--refresh]
udf run check --workspace W|--task TASK [--scenario NAME | INLINE_OPTIONS]
udf run plan  --workspace W|--task TASK [--scenario NAME | INLINE_OPTIONS]
udf run start --workspace W|--task TASK [--scenario NAME | INLINE_OPTIONS] [--background] [--plan-digest DIGEST]
udf run status [EXECUTION_ID] [--workspace W|--task TASK]
udf run wait EXECUTION_ID [--timeout SECONDS]
udf run stop EXECUTION_ID [--force]
udf run compare --before EXECUTION_ID --after EXECUTION_ID --expect fixed|no-regression
```

所有叶子支持现有 `--format human|json`。`configure` 也接受 workspace；上面的 TASK 形式展示常用任务配置。
`INLINE_OPTIONS` 包括 `--project`、`--mode editor|game|commandlet`、`--map`、`--rhi real|null` 和工作内容。
工作内容只能选 `--exec`（可重复）、`--tests`、`--commandlet` 中的一种，均不选表示交互启动。
`--assert clean-exit` 是快捷配置，补齐必需的退出与证据要求；未知断言名拒绝。

| 命令 | 是否启动 UE | 持久状态与返回内容 |
| --- | --- | --- |
| configure | 否 | 校验并保存场景 revision、来源身份、修改原因 |
| discover 无 refresh | 否 | 只读文件、进程与已有发现结果；返回覆盖范围和新鲜度 |
| discover tests --refresh | 是 | 明确启动发现进程，保存 purpose=discovery 的独立 execution，不执行测试 |
| discover maps --refresh | 仅需引擎解析时 | 先输出发现计划；在明确 refresh 动作内启动资产查询，保存副作用和发现记录 |
| check | 否 | 只读，返回四态 readiness；不会创建 execution、缓存或 latest |
| plan | 否 | 返回完整运行描述、参数与输出模板；不含运行中状态或动态 readiness |
| start | 是 | 内部重新 check，冻结计划再创建 execution；前台等待最终结果 |
| status | 否 | 查询既有记录；不伪造退出码，不写原始事实 |
| wait | 否 | 等既有执行的最终判定，返回供 CI 使用的退出码 |
| stop | 否 | 取消指定受管执行；尝试已声明的退出方式，强制停止需显式 --force |
| compare | 否 | 比较两份终态记录；不切分支、不编译、不重新运行 |

`check` 遇到发现结果过期返回 `discovery_required` 与完整 refresh 命令。缓存绑定引擎、模块二进制、项目配置和发现器版本。执行命令不从“最近 task”推断目标；status 省略 ID 仅在查询范围内唯一时选取，否则列候选。

## 配置与 v0.5.0 的衔接

场景是有限的运行描述，不是脚本语言。采用严格 JSON，与 `package configure --file` 的使用方式一致。
task 配置落在 `<Host>/.udf/run/scenarios/<name>.json`；workspace 配置落在用户配置目录的 `run/scenarios/<workspace>/<name>.json`。
可共享的候选文件由项目版本控制，configure 将它解析并绑定本机来源。运行时不再叠加一套隐式用户覆盖文件。

必须复用或提炼 package 的来源绑定、未知字段拒绝、revision、摘要与带备份替换机制；run 不复用 Cook/container 字段。
更新已保存配置要有 reason 和期望 revision，防止覆盖另一个会话的修改。CLI 显式覆盖只影响本次计划，保存前后差异，不暗改场景。

分清三个摘要：`scenarioDigest` 表示场景和断言；`inputFingerprint` 表示项目、配置及二进制；`planDigest` 覆盖前两者与实际 argv、模式、退出策略和解释器版本。临时 execution ID 不参与稳定计划摘要，输出目录在计划中表示为模板。

旧代码 `PlanReport` 的摘要没有覆盖 source，本次 run 不能直接把该摘要当成完整运行许可。新增 run 描述与摘要函数，不改变现有 build/package JSON 的含义。

## 项目、代码和二进制分别记录

| 选择 | 业务内容 | 实际进程项目 | 限制 |
| --- | --- | --- | --- |
| workspace/main | workspace 主项目 | 主项目 .uproject | 显示当前实际插件绑定，不能默认为主插件 dev |
| task/host | task Host 和其插件内容 | 从元数据找到规范 Host .uproject | 默认不拥有主项目 /Game 关卡 |
| task/main | workspace 主项目 | 现有主项目 .uproject | Junction 必须已绑定目标 task；不自动 switch |
| task/isolated | workspace 主项目的明确输入快照 | UDF 创建的独立项目副本 | 覆盖 task primary 插件，使用独立 Saved/Config 用户状态 |

task 执行必须显式给 project 或由具名场景保存；不在 Host 缺图时悄悄换成 main。
引擎沿用 task 冻结 context 或 workspace 配置的解析顺序，记录所有来源。环境变量仅用于缺值回退。
路径文本不同先规范化比较；明确 EngineAssociation 与实际引擎不兼容才阻止，不把低优先级环境变量当成同等配置冲突。

isolated 提炼 package 的受管输入准备代码，不能直接调用打包 staging 后顺便启动。
先规划所需 Content/Config、插件二进制与外部数据的闭包，评估复制空间。可写内容复制，独立 Saved 和日志；引擎/DDC 的共享策略明确记入计划。
同一个副本不得在进程仍加载 DLL 时被下一次构建覆盖。需要绝对路径、网络数据或特定用户布局的项目，先列缺口与准备方法，不能声称复制目录就完整复现。

每次记录 source revision 与工作区内容摘要、build execution ID、引擎 BuildId、必需模块的实际路径与 DLL SHA-256。
快照不足或无法核验加载身份时，退出本身可以给局部观察结论，严格修复证明返回 `build_provenance_missing`。
不能用 mtime 指纹或源码 HEAD 替代 DLL 来源。主项目数据发生变化时，重复运行与对照必须重新取快照。

## 关卡、模式与触发状态

`UnrealEditor-Cmd.exe` 是进程外壳，不意味着 Commandlet 或 NullRHI。Editor、Game、Commandlet 模式与 real/null RHI 分开选择。
这次退出案例是 Editor 模式 + Cmd 外壳 + NullRHI；仍然能经过 Slate/Details 和 Editor Subsystem 的退出路径。

地图要求为 `none|optional|required`。交互默认地图可参考 EditorStartupMap/GameDefaultMap，但查询必须注明读取的配置层和未覆盖的 UE 配置规则；解析不完整时要求显式指定或 refresh。
文件枚举是候选，实际挂载名与 world 加载结果由引擎发现或运行证据确认。插件内容、重定向和同名关卡不能取第一项。

跨 shell 推荐已保存的 scenario 或不以斜杠开头的 map ID。旧版 `asset:/Game/...` 未经 Git Bash 验证，取消“保证安全”的承诺。
UDF 发现输入被转换成 Git 安装目录时报 `shell_path_mangled` 并给场景配置入口，不自动猜改。
内部使用 argv 数组，加 Windows 与 UE 自有命令行解析测试；数组本身不能证明嵌套 ExecCmds 引号已经正确。

| 场景 | 正向前提与业务断言 | 退出断言 |
| --- | --- | --- |
| 打开 Editor | 项目身份、期望地图或明确允许空地图；人工查看 | 用户未退出时 phase=ready、outcome=pending |
| Launch Game / 性能 | 实际地图；相机、分辨率与 RHI；业务完成报告 | 报告完成后继续核验进程退出 |
| Automation | 完整测试选择清单非空，所需模块加载；报告数量与结果 | 报告通过后还要 clean-exit |
| Settings.Get/Set | 对应命令的返回值；负向用例的预期错误；保存与重启读回分开 | clean-exit |
| EarthModeler 最小退出 | Editor/PropertyEditor/EarthModeler 已加载；允许空图；已请求退出 | 最小启动退出的 clean-exit |
| EarthModeler 完整回归 | 额外准备并确认 DetailsView/Actor 选择状态；重复同一触发动作 | 同一退出故障签名先复现，再验证修复 |
| Commandlet | 确认具体 -run 入口与结果，按该 Commandlet 声明 world/渲染能力 | clean-exit；不假定任意 Commandlet 天生不加载 world |
| 已有 Editor | 核验 PID、项目与能力；返回人工命令或已有适配器操作计划 | 观察能力不足就报告 coverage gap |

第一版只支持有限准备操作和稳定证据类型。DetailsView/Actor 准备若依赖不存在的控制接口，完整回归 check 返回 `unsupported_precondition`。
最小退出 smoke 仍可运行，但必须显示较窄覆盖。不能用干净启动时没有选中对象这一条件伪造“旧崩溃已修复”。

## 完整运行生命周期

```mermaid
flowchart LR
  A[冻结来源和计划] --> B[启动进程]
  B --> C[确认项目与准备状态]
  C --> D[执行业务并确认结果]
  D --> E[请求正常退出]
  E --> F[取得真实进程终止状态]
  F --> G[收齐日志及崩溃证据]
  G --> H[逐条断言与总体判定]
```

持久字段分为 `phase` 和 `outcome`。phase 为 preparing/starting/ready/workload/exitRequested/processExited/collecting/finished；outcome 为 pending/passed/failed/inconclusive/cancelled。
另存 `actionResult`、`exitResult`、`diagnostics` 和 `coverage`。业务成功、退出失败是合法且必要的组合。
任何阶段崩溃都先保存事实再进入 collecting；不会因为业务断言已通过而停止观察。

| 部分 | 必须行为 |
| --- | --- |
| 监督进程 | 每次执行启动一个 UDF 子进程，持有 UE 句柄，持续读取 stdout/stderr 并保存退出码；不依赖聊天会话存活 |
| 身份 | execution ID + PID + 创建时间 + executable + 项目；监督器和 UE 均记录，防止 PID 复用 |
| 后台启动 | 监督器已写入 phase=starting、outcome=pending 和进程身份后才回后台应答；不能只返回 shell PID |
| 日志 | execution 独立目录，UE 用绝对 abslog；stdout/stderr 独立记录，支持 UTF-8/UTF-16 与 BOM |
| 原子状态 | 原子替换快照，追加有序事件；崩溃/重启时检测半行和缺口，保留原始文件 |
| 超时 | startup/workload/shutdown/collection 四个期限，任何期限触发都保留阶段与诊断 |
| 观察者丢失 | PID 消失不推断成功；无句柄终止证据时 outcome=inconclusive、observer_lost |
| 取消 | stop 标记取消；有已声明的退出适配器才尝试正常退出；force 为明确强制清理，不能变成 clean-exit passed |
| 恢复查询 | status/wait 只读既有事实；有存活监督器就继续等；丢失观察者不能事后恢复一个假退出码 |

资源占用记录由监督器持有，标明项目可写路径、加载的模块路径及共享 GPU 策略。
其他 UDF 构建、switch、cleanup/delete 在会改变这些资源时检查占用并返回 deferred；过期占用只有核验进程身份后释放。
普通 GPU 进程告警，声明 exclusive 的性能场景遇到其他 real-RHI UE 进程则等待。UDF 不结束用户开的其他编辑器。

## 退出请求与异步动作

有限退出策略为 `queued-editor-quit`、`automation-complete`、`self-exit`、`manual`。

- queued-editor-quit 只用于已知同步或启动 smoke，按 UE 版本构造 `..., QUIT_EDITOR`，并验收前置与业务结果。
- automation-complete 使用已验证的 Automation 队列完成机制；测试清单和报告都不完整时不能通过。
- self-exit 适用于插件已有 QuitWhenDone 能力，要求业务完成之后的退出请求证据。
- manual 用于交互，进程存活时保持 pending；CI 要求有限退出时拒绝此配置。

给任意异步命令追加 Quit 不代表顺序等待。需要“文件完成后再发退出命令”时必须有现成、已验证的适配器；缺少时返回 `unsupported_completion_barrier`。本轮不部署 MCP 或自带通用脚本插件。

## 正常退出与日志断言

clean-exit 通过需要同时满足：所要求的准备状态与退出请求有证据；受管 UE 自然终止且退出码在允许集合；未取消或被强杀；本次完整日志收集成功；没有本次致命崩溃证据；该引擎适配器要求的正常关闭标记齐全。
无正向业务要求的 smoke 声明 workload.kind=startup-smoke，触发命令保存在 startupCommands。
该场景的 actionResult=not_applicable；dispatch-only 只记录触发命令被分发，总体通过仅覆盖前提与退出断言。
普通业务场景禁止用 dispatch-only 代替业务完成。

| 观察 | 结果 |
| --- | --- |
| 业务报告成功，退出时 access violation | action passed；exit failed；overall failed |
| 退出 0，日志有本次 Fatal/Crash | exit failed，保存矛盾证据 |
| 退出 0，只有普通 Blueprint Error | clean-exit 可通过；内容诊断保留；开启 no-log-errors 则该断言失败 |
| 非致命 ensure 带调用栈/报告 | 单独 ensure 诊断，按 fail-on-ensure 策略决定；不按调用栈三个字判 crash |
| NoSuchField 是声明的负向测试 | 匹配命令、阶段、预期次数与错误语义后通过该业务断言；不全局屏蔽同类错误 |
| 进程退出 0，但日志缺失或截断 | inconclusive，不证明 clean-exit |
| 超时、强杀或取消后日志关闭 | failed 或 cancelled，正常结束标记不能抵消干预 |
| 历史 CrashContext 未关联本次 PID/时间 | 不用于本次崩溃判定，保留为未关联诊断 |

每次保存 unsigned 32-bit 原始 Windows exit code、十六进制表示和可选解释。UE handler 返回的 `3` 与异常 `0xC0000005` 是两个字段。
CrashContext/minidump 属于补充证据，正常运行不强求存在。扫描范围限定项目与本次关联 CrashGUID/PID/创建时间；来源不能确认时不得挪用别次材料。

日志断言限定 source、phase、literal/受限 pattern、次数与顺序。命令回显只证明分发，不能单独满足 action passed。
文件断言要求独立输出目录、run ID 或等强关联，mtime 只做辅助。只出现半份 JSON、旧报告或别的 UE 更新同文件，均不得通过。
如插件支持 OutputDir，场景必须显式把 UDF output 模板传给它；模板不能自行改变插件输出行为。

## 配置示例与实际用法

完整候选示例见 [earthmodeler-exit-smoke.json](../assets/earthmodeler-exit-smoke.json)。它对应已验证的 Host 最小启动退出。
另一个 business-query 场景可把工作内容改为 Settings.Get，要求 `Type=int32 Value=20`；修改后的场景必须重新取得匹配基线。
真实案例中的 stat unit 两次日志不能用作不同 Settings.Get 场景的严格前后对照。

```powershell
# 以下均为拟新增接口。一次固定场景，后续会话直接复用。
udf run configure --task neon-dev/earthmodeler-exit-crash --scenario exit-smoke --file earthmodeler-exit-smoke.json
udf run check --task neon-dev/earthmodeler-exit-crash --scenario exit-smoke --format json
udf run plan --task neon-dev/earthmodeler-exit-crash --scenario exit-smoke --format json
udf run start --task neon-dev/earthmodeler-exit-crash --scenario exit-smoke --format json

# 原有 Launch Game 场景也走同一入口；长参数可以保存成场景。
udf run plan --task neon-dev1/perf-benchmark-toolkit --project main --mode game --map aes6_sh_sz_q1 --rhi real

# 两次执行分别准备好二进制和输入，再比较已有记录。
udf run compare --before BEFORE_ID --after AFTER_ID --expect fixed --format json
```

场景 JSON 的 `assertions` 与运行产生的 `assertionResults` 分开，机器结果例子见
[business-pass-exit-crash.json](../assets/business-pass-exit-crash.json)。示例是拟定 schema 数据，不是已执行的 UDF 输出。

## 前后回归对照

compare 首先核验完整性与可比性，然后核验失败到通过的变化。保存每边的执行 ID、完整指纹与断言详情。

| 必须一致 | 允许显式变化 |
| --- | --- |
| 场景行为及断言版本、准备方法与触发状态、模式/RHI、逻辑项目及内容/配置摘要、引擎与关键环境 | 被测插件源码/二进制 variant；完整列表由场景声明 |
| 映射后等价的关卡、命令顺序、退出策略 | task ID、临时路径、PID 和执行时间等运行实例字段 |
| 性能比较还要求相机、分辨率、GPU、热身与并发策略 | 仅声明允许变化的性能参数；变化后报告比较口径 |

项目实际绝对路径可以不同，但要绑定同一逻辑项目和内容快照。Host 与 main 不能仅因装了相同插件就互当对照。
配置路径中包含 task ID 的部分可规范化；业务路径、断言和版本不可被宽泛忽略。

`--expect fixed` 必须声明目标 failure signature。EarthModeler 签名由退出阶段、access violation、包含 FEarthModelerModule::OnPreExit 的关键帧组成；不固定内存地址与源代码行号。
签名的 lifecycleStage 取 startup/workload/shutdown，是故障发生阶段的分类，与持久 phase 分开。
shutdown 依据本次退出请求或引擎退出路径的日志时序确认，对应 exitRequested 之后的终止过程；不能把收集期才发现的早期崩溃误归到 shutdown。
缺少发生时序则 lifecycleStage=unknown，严格签名不匹配，返回证据不足；终态 phase=finished 不用于匹配故障阶段。
基线只有一般非零退出码不足以证明同一缺陷复现。candidate 通过必需断言且不再有该签名才返回 fixed。

| 两边结果 | compare 结论 |
| --- | --- |
| 同场景基线复现目标崩溃，候选完整通过 | fixed，附覆盖范围 |
| 两边都通过 | baseline_not_reproduced；no-regression 模式可通过，fixed 模式不通过 |
| 候选仍失败或出现另一致命问题 | not_fixed 或 new_failure |
| 引擎/地图/布局前提不匹配，或二进制来源缺失 | not_comparable 或 inconclusive |
| 只有 Host 候选通过，主项目候选缺失 | 主项目 coverage gap，不能总称工程已修复 |

重复次数由场景声明，保存每次结果，所有要求次数都达标才通过；不自动忽略失败后重试得到的绿灯。
第一版按已有 execution 比较，不替调用方 checkout、rebase 或覆盖 DLL。新旧版本要分别在受管来源中构建，或顺序准备且保存不可变二进制证据。

历史材料可由离线 evidence importer 形成 `provenance=imported` 的记录，必须显式提供日志、process sidecar 与来源声明，不运行脚本。
缺少的退出码、二进制和准备状态保持 unknown。导入能重放分类与对照规则，不会使旧日志获得当年没有记录的证明；不增加任意路径执行入口。

## JSON 与 CI

复用当前信封键 `command/ok/data/error/messages`，添加 `data.schemaVersion`；新字段局限于 run。
status 查询成功可 `ok=true` 且 data.outcome=failed，表示成功读到失败记录；start/wait/compare 以最终验证结果决定返回值。
需要新增“一次输出失败信封且携带 data”的输出入口；现有 emit_failure 只带字符串错误，不能丢掉 executionId 与断言。

| 调用结果 | CLI 退出码 |
| --- | --- |
| start/wait 全部必需断言通过；compare 达到指定期望 | 0 |
| 断言失败、候选未修复或观察到 UE 崩溃 | 1 |
| CLI 语法错误 | 2，保持 clap 行为 |
| 无法启动、配置/来源/能力缺失 | 3 |
| 证据不足、观察者丢失、对照不可比 | 4 |
| 执行阶段超时 | 5 |
| 已取消或被显式强杀 | 6 |
| 后台启动应答 | 0 只表示 accepted，data.terminal=false；CI 必须接 run wait |
| status/check/plan 查询成功 | 0；check 的 readiness 是查询结果，与 build/package 一致 |

wait 自身等待超时返回 5 但不取消 UE，也不写成 execution timeout。stderr 用于有限进度，stdout 只有一份 JSON。
领域错误保持上述 JSON 保证；clap 帮助与语法错误沿用现有行为，若要连参数错误也输出 JSON，需要另行调整全局解析入口。

## 分阶段实现与影响面

| 阶段 | 交付与可检查结果 | 依赖 |
| --- | --- | --- |
| A | 严格场景配置、来源解析、无进程的 check/plan；证据解析器用前后真实日志重放 | 最新 package 配置机制 |
| B | 监督进程、状态存储、独立日志与最终退出码；假进程覆盖晚发 crash、超时和取消 | A |
| C | Host exit smoke 与业务命令场景；argv/UE 引号验证；现有 execution 的前后比较与 CI | B |
| D | 主项目/isolated 内容来源；状态准备、地图发现、Automation 和性能场景；接入资源占用检查 | B、C |
| E | 显式历史证据导入；按场景重复，补齐 main/real-RHI 覆盖 | C、D |

优先让新待办成为 C 阶段的首个端到端验收场景，再推广到地图和性能测试。
不会只交付一个“命令启动成功”的 v1 后才考虑退出问题。

| 生产接入范围 | 当前文件与新增责任 |
| --- | --- |
| 现有必改 4 文件 | `src/cli.rs`、`src/main.rs`、`src/commands/mod.rs`、`src/output.rs` |
| 可复用/提炼 7 文件 | `src/package_profile.rs`、`src/project_packaging.rs`、`src/source_context.rs`、`src/ue_commands.rs`、`src/execution.rs`、`src/editor.rs`、`src/commands/package.rs` |
| 生命周期消费者 5 文件 | `src/commands/build.rs`、`src/commands/build_policy.rs`、`src/commands/switch.rs`、`src/commands/cleanup.rs`、`src/commands/delete.rs` |
| 拟新增 5 责任模块 | `run_profile`、`run_context`、`run_process`、`run_evidence`、`commands/run`；配置存储与监督事件先留在相应模块 |
| 文档入口 4 文件 | README、AGENTS、CLAUDE、安装用 `skills/unrealdevflow/SKILL.md`；仓库 `skill/SKILL.md` 若存在则一并更新 |

以上是 16 个现有生产文件的候选接入面，提炼后才能确定实际改动数。不得沿用旧版“6 改 + 3 新”的已定范围。
实现前用下面命令复算消费者，另核查 task_routes 与 task_junctions 是否需要直接改动：

```powershell
rg -n 'Configure|EXECUTION_CHECK_ABOUT|EXECUTION_PLAN_ABOUT' src/cli.rs
rg -n 'SourceContext|ExecutionRecord|PlanReport|emit_failure' src -g '*.rs'
rg -n 'prepare_project_stage|save_result|source_fingerprint|fn configure' src/commands/package.rs src/package_profile.rs
rg -n 'is_editor_running|resolve_task|cleanup|switch' src/commands src/task_routes.rs src/task_junctions.rs
```

## 验证矩阵与交付界限

| 测试范围 | 必须覆盖 |
| --- | --- |
| CLI/配置 | 与 package 相同的查询词义；非法组合、未知字段、并发 revision、输入变化；check/plan 无进程无记录 |
| 参数与目标 | PowerShell/cmd/Git Bash；空格中文引号；同名地图、默认配置不完整、Host 缺图；不能被原始参数覆盖项目或退出策略 |
| 监督与证据 | 业务通过后崩溃、exit 0 + Fatal、exit 0 + 74 Error、Ensure、负向用例；旧文件、截断日志、PID 复用、观察者丢失、强杀 |
| 对照与 CI | 基线未复现、错误签名、两边通过、不可比、缺 DLL 证据；查询成功和验证失败的退出码差别 |
| 资源与恢复 | 后台断开、加载 DLL 时 build、运行中 cleanup/switch、锁持有者崩溃；保留用户目录与其他 UE 进程 |
| 真机 | Host 最小退出；状态可确认的 Details 回归；主项目关卡；Automation 完成后退出；性能分辨率与 real RHI |

本版覆盖 AC-001 至 AC-012：旧案例路径/地图需求保留；新增 lifecycle、比较和 CI 契约分别对应 AC-008 至 AC-010；版本衔接与独立审查对应 AC-011/012。
记录层校验和独立设计评审只能证明方案自洽，不能替代 C/D/E 阶段的真机实现验收。

未定的引擎适配细节是：5.5.4 上各模式退出命令的精确解析、完整配置/测试发现接口、Details 状态准备方式。
实现计划先用本机引擎与已有接口验证这些点；任何能力未证实就返回明确缺口，不靠固定延时或新增权限默认放行。
