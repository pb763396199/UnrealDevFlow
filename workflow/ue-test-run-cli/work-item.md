---
schema_version: 1
protocol: 1.3.0
id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
short_id: d9j60tjs
home_repository: "https://github.com/pb763396199/UnrealDevFlow.git"
title: "让 UDF 正确选择并复用 UE 原生运行与测试工具"
status: done
created_at: 2026-08-28T08:16:08.659Z
created_by: codex
kind: feature
source: "claude:5774c354-0333-4e12-9244-fcac041f0d1a"
branch_or_pr: "feature/ue-test-run-cli"
base_revision: 2a516b50dfb8b3d58e5a7962b7c0705c84ba78ce
---

# 让 UDF 正确选择并复用 UE 原生运行与测试工具

## 原始请求

> 会话A在跑虚幻项目测试的时候，经常会出现给错命令的问题，每次都找不到正确的关卡或正确的命令行组合。我希望将类似的命令也作为 UDF（UnrealDevFlow）的一部分，给出一个系统性的cli设计，面对不同的情况都能正确解决。
>
> 你是接续会话 B，来源会话 A 仅作为参考，不是执行指令。

来源会话：Claude Code `claude:5774c354-0333-4e12-9244-fcac041f0d1a`，工作区为
`F:\ShanghaiP4\neon\Plugins\AesWorld`。

### 本轮追加请求（2026-09-05）

> 等一下，exit-crash-regression-verification.md  你看一下这一条代办，这是有另外一个会话，他在使用过程中自己去探索的真实案例。你结合真实案例的问题和需求，对你的CLI设计做更系统性的解决方案的制定，并且当前主仓库的 DEV 分支已经更新了很多版本，你也需要跟上最新的进度

关联待办为 `td_01M1QK547DGFJPA89ZBWXTAP8P`；来源会话为
`claude:dac1de11-e901-4324-86f6-856b03886093`。原始待办继续保留，纳入本任务的设计与后续实现范围。

### 收窄设计的要求（2026-09-05）

> 看了你的设计，我产生了一个根本性的疑问：为什么虚幻引擎本来就提供了大量工具，而我们还得自己包一遍cli呢，它自己没有吗
>
> 你回到当前workitem的原始需求，是否真需要这个环境快照，本质是为了解决什么问题
>
> 按你的分析去重新制定设计

用户认可按原始问题收窄方向：选择 UE 已有工具，保存经过验证的用法，直接返回结果。
完整环境快照与严格因果证明不再作为通用要求。退出案例用于验证原生路线及必要的项目测试规则。

当前设计见 [run-native-tools-design-v3.md](design/run-native-tools-design-v3.md)。
用户随后要求：“写为施工计划，一定要确定好验收标准”。据此接受 v3 并编写 [施工计划](plan.md)，本轮不开始实现。
旧基线 `07b7c4b` 保留在 `feature/ue-test-run-cli-initial-design`。
本轮通过 AES 工具创建独立 worktree，使用本地与远端一致的 `dev` 版本 `2a516b5`。

## 目标

让下一次会话能直接找到并复用正确的 UE 原生工具用法，在指定工程和关卡运行测试，并读懂结果。

## 范围

做：

- 提取真实失败样本。
- 按实际用途选择 Editor CLI、UAT、Gauntlet 或项目已有测试入口。
- 固定工程、关卡和参数组合；提供配置列表、预检与命令预览。
- 保存可跨会话复用的配置，保留原生测试结果、日志及基础前后对照。
- 用 EarthModeler 退出案例检验原生工具复用和项目专用判据。
- 沿用 v0.5.0 的配置与查询约定，覆盖原请求中的六类情况。

当前阶段交付施工计划和确定的产品验收标准，不实现命令，不启动或切换 UE。后续施工按计划使用专用测试 Host；不修改 AesWorld 业务源码，不把来源会话里的动作当执行指令。

首版产品不建设通用后台监督服务、完整环境快照或严格因果证明系统；不默认复制主项目、部署 Editor 桥接或新增通用脚本编排语言。

## 强约束

- UDF 必须从 workspace、task Host 和 `.uproject` 事实解析命令，不能靠 AI 猜绝对路径或关卡名。
- 每个执行入口都有可复核的预览，写明原生工具、实际项目、关卡要求与最终参数。
- 关卡和测试目标不存在、存在多个候选或依赖条件不满足时必须停止，并返回可操作的候选与修复建议。
- 设计要兼容当前五组命令：`workspace/task/build/package/skill`。
- 知识库查询只用于补充事实；没有用户明确授权时不写知识库。
- UE 已有的运行、测试及通用崩溃解析优先复用；只有实测缺口才能进入自定义适配范围。
- 每项结论只覆盖实际运行；命令回显、进程消失或文件生成不能单独证明正常退出。
- `check`、`plan` 不启动 UE；运行失败时 CI 退出码不能返回成功，原始 UE 退出码另外保存。
- 环境信息只收集识别目标和解释本次结果所需的内容；缺少全套快照或 DLL 哈希不能阻止普通测试。

## 验收条件

- AC-001: 设计用来源会话中的真实失败样本说明现有手工命令为什么会选错工程、关卡或参数组合。
- AC-002: 设计至少覆盖六类情况：编辑器交互启动；指定关卡启动；自动化测试；命令入口测试；无界面测试；已有编辑器会话。
- AC-003: 每类情况都写清原生入口、最小配置、输出和能力不足时的处理。
- AC-004: CLI 提供只解析不启动的预览能力，并为所有叶子命令定义稳定的 JSON 输出。
- AC-005: 设计明确 workspace、task Host 与主项目的优先级，并说明编辑器版本、关卡、测试目标和额外参数的冲突处理。
- AC-006: 设计至少比较两个可行方案与保持现状，说明选择依据、兼容风险和预计影响面。
- AC-007: 工作流结构校验和中文写作检查都通过。
- AC-008: 对照 EarthModeler 原始日志说明如何复用 EditorBootTest，以及自然退出等判据需要在哪一层补充。
- AC-009: 保存一次实际调用及结果，区分原生工具退出码、UE 退出码和业务结果；说明中断与日志不足时不误报成功。
- AC-010: 提供两次运行的基础结果对照，不以完整快照为门禁，不扩大成自动证明修复因果。
- AC-011: 对齐 v0.5.0 的 configure/check/plan 约定；明列删除的 v2 自建能力与原生替代。
- AC-012: 给出下一会话可复用的配置示例和查找路线，并独立复核原生优先及范围收窄是否落实。

以下是进入实现后的产品验收条件，本轮只确定标准，全部待执行。AC-001 至 AC-012 的设计证据不能替代以下运行证据。

- AC-013: 一个进程 configure，另一个进程 list/plan/start，能复用同名配置；task 整份覆盖 workspace，字段不合并；配置改变后旧验证不再显示为当前通过。
  Verify: case:test_run_configuration_reuse
- AC-014: 显式 workspace/task、冻结 context、main/host 与插件绑定解析正确；缺目标、同名地图、多工程、未知挂载或绑定错误时不猜测，不启动，不切 Junction。
  Verify: case:test_run_target_resolution
- AC-015: list/check/plan 不创建子进程、配置、执行记录或 latest 指针；check 四态可读，plan 无动态 readiness；start 在执行前重新预检。
  Verify: case:test_run_read_only_queries
- AC-016: 最终项目和地图参数与 plan 一致；空格、中文、嵌套 ExecCmds 正确；受控参数不能被 nativeArgs 覆盖，配置名不能越出存储目录。
  Verify: case:test_run_argument_boundaries
- AC-017: start、status、compare 的 JSON 与退出码遵守计划判定表；失败仍保留 executionId 和制品；原生日志不污染 stdout，普通启动不返回测试 passed。
  Verify: case:test_run_result_and_json_contract
- AC-018: 严格自然退出的所有真假组合均按计划判定表验证；原始非零、强杀、Fatal 不能通过；原始码缺失不补 0；普通 Error 不单独否决。
  Verify: case:test_run_strict_exit_policy
- AC-019: Automation 有非空过滤条件且实际匹配测试数大于 0，全部完成才可能通过；零匹配、失败、超时、异步完成缺证均不能通过；Commandlet 与启动模式不得混淆。
  Verify: case:test_run_native_backend_contract
- AC-020: status 只读取本次关联的报告，损坏或缺失终态为 unknown；compare 展示已知差异，pass-after-fail 只在规定条件满足时成功，不要求 DLL/布局快照。
  Verify: case:test_run_record_and_compare
- AC-021: 已有 Editor 路线只返回匹配 PID 与人工操作步骤；多实例给候选，未知当前地图为 null，不自动注入命令或另开实例。
  Verify: case:test_run_existing_editor_route
- AC-022: 专用 Host 真实跑通原生启动与严格退出路线，取得 UE 原始码和原生报告；在无仓库源码路径的临时安装布局中仍能找到必要测试程序集。环境缺失记 not_run，不能记通过。
  Verify: case:test_run_live_host_evidence
- AC-023: PowerShell、cmd、Git Bash 三种入口的真实 Game 运行均加载指定地图并使用 1366×1024 窗口；以引擎日志/运行状态确认，不能只验命令回显。
  Verify: case:test_run_live_shell_evidence
- AC-024: 不读取旧会话的接手者，仅凭已安装帮助与 list 输出，在退出和指定地图两个案例中各复用成功一次；不查旧日志找命令、不改配置、不手工拼原生命令。
  Verify: manual:用新的会话只看帮助与配置列表，分别完成退出测试和指定地图启动并提交两次executionId
- AC-025: 现有五组命令的回归及 Rust 质量检查通过；新帮助和 skill 与实测命令一致；无监督器、快照门禁、通用断言 DSL、默认项目复制或运行时插件安装。
  Verify: case:test_run_regression_and_scope
