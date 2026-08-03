---
schema_version: 1
artifact: research
artifact_id: ar_01KZ36R5TZZ1F9D5EPPD75J3HW
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
attempt_id: null
created_at: 2026-08-03T07:00:00.479809Z
producer: aes-research
result: partial
supersedes: ar_01KZ365CX59VZF7ZCNHNNZH0YN
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts: []
---

## 这份调查只回答了半个问题

用户后来澄清：要对比的是 `unrealdevflow` 这个独立工具和 UWF 里的 DevFlow 模块，目标是把
DevFlow 拆回独立工具。本记录写在澄清之前，对比对象选错了，把本仓库当成了对比的一方。

仍然有效的部分：DevFlow 的 15 个动作清单、确认边界机制、受控构建、全局配置路径，这些都是
直接从源码读出来的事实。失效的部分：所有涉及本仓库的对比和推进建议。

真正的对比见 [devflow-fork-drift-research.md](devflow-fork-drift-research.md)。

## 要回答什么

用户要求把本仓库和 UnrealWorkflow 的 DevFlow 逐项对比。这份调查回答三个能被证伪的问题：

1. DevFlow 对外提供哪些功能？以源码里的命令表和 action 表为准，不以文档描述为准。
2. 本仓库对外提供哪些功能？以工具子命令表和 Skill 清单为准。
3. 两边哪些功能能一一对应，哪些只有一边有？

查到每个 action 和每个子命令都能说清它做什么就停。不评估代码质量，不评估性能。

## 看了什么

- `F:\AiProject\UnrealWorkflow\Source\DevFlow\`，9627 行 Rust，2026-08-03 读取。
- `F:\AiProject\UnrealWorkflow\Source\Cli\src\main.rs`、`CONTEXT.md`、`Docs/adr/`，同日读取。
- 本仓库 `skills/engineering/`、`docs/adr/`、`tests/`，同日读取，HEAD 为 `adb5dd5`。

UnrealWorkflow 工作区版本 `0.1.2`。DevFlow 契约 schema 为 `uwf.module.devflow.v2`，任务元数据 schema 为 3。

## DevFlow 有什么

DevFlow 是 UnrealWorkflow 里的一个 Rust 模块，不是独立程序。它对外暴露 11 个命令和 15 个动作。

命令这一层管的是协议：`status`、`capabilities`、`commands`、`schema` 读取自身契约，`dry-run`
预演，`execute` 执行，`history` 和 `artifacts` 读任务历史与产物，`build-status`、`build-check`、
`build-gate` 管构建。

动作这一层才是真功能，共 15 个：

| 动作 | 做什么 |
| --- | --- |
| `read-status` | 读各 UE 项目当前挂着哪个任务，junction 是否有效 |
| `list-tasks` | 列出所有工作区下的任务，含损坏任务 |
| `read-workspaces` | 读已注册的工作区 |
| `configure-workspace` | 注册一个工作区：UE 项目、引擎、Hosts 根、Plugins 根 |
| `create-task` | 建执行实例：给每个主插件开 worktree 和分支，生成宿主工程和 junction |
| `next` | 提示这个任务下一步该干什么 |
| `switch` | 把 UE 项目的 junction 切到指定任务，可重生成工程文件 |
| `build-task` | 编译任务宿主工程，支持后台、互斥模式、只编主插件 |
| `build-status` | 读构建状态，后台进程退出时对账 |
| `build-project` | 编译主项目而不是任务宿主 |
| `build-check` | 只检查构建策略，不启动编译 |
| `merge` | 把任务分支并回目标插件仓库，4 种策略 |
| `finish` | merge 的友好包装，不给策略就交互式问 |
| `cleanup` | 删 worktree 和分支，带重试 |
| `delete` | 删整个任务，检查分支是否已合并 |

几个只有 DevFlow 才有的机制：

- **确认边界令牌。** 危险动作必须先 `dry-run`，再带上完全匹配的 `UnrealWorkflow.DevFlow.<action>.v2`
  才准执行。令牌不对直接拒。
- **受控构建。** `build_policy.rs` 815 行，解析 `.uproject` 的 `EngineAssociation`，定位
  `Build.bat` 和 UBT，算 UBT 互斥锁名，产出 `ready | needsUserInput | blocked | deferred`
  四种结论。`build-gate` 专门拦截绕过受控构建的调用。
- **多插件任务。** 一个任务可以有多个主插件，每个主插件一个 worktree 一个分支。依赖插件
  分引擎依赖和项目依赖：引擎依赖改 `.uproject` 开关，项目依赖建 junction。
- **后台构建跟踪。** 记 pid、日志路径、开始和结束时间、退出码。进程没了但拿不到退出码就标
  `unknown`，要求重新受控构建来对账。
- **全局配置和全局状态。** `~/.unrealworkflow/devflow/config.toml` 存工作区注册表，
  `state.json` 存各项目当前挂哪个任务。两个文件都在用户目录，不在仓库里。

## 本仓库有什么

本仓库是 Skill 包加一个 2416 行的 Python 工具，没有编译产物，没有常驻进程。

13 个 Skill：`aes-using-workflow` 是入口，另外 10 个 engineering Skill 覆盖 brainstorm、
research、plan、execute、debug、review、validate、finish、retrospect、writing-skills，
外加 2 个 productivity Skill。

工具 23 个子命令，按用途分四组：

| 组 | 子命令 | 做什么 |
| --- | --- | --- |
| 算摘要 | `id`、`short-id`、`digest`、`raw-digest`、`contract-digest`、`change-digest`、`snapshot`、`check-snapshot` | 生成 ID，算规范化摘要，取代码快照 |
| 找对象 | `discover`、`resolve`、`select-attempt`、`status` | 找任务和路线，判断每份记录还算不算数 |
| 写记录 | `create-work-item`、`create-attempt`、`create-artifact`、`revise-artifact`、`refresh-digests`、`cas-write` | 建对象，改记录，级联刷新指针，并发安全写 |
| 管门禁 | `finish-check`、`validate`、`commit-message`、`collect-metrics`、`record-event` | 收口检查，结构校验，生成提交信息，记度量 |

只有本仓库才有的机制：

- **三层对象模型。** Work Item 是要交付的事，Attempt 是一条路子，Artifact 是路上的记录。
  记录分 10 类，每类的头部字段是封闭的，多一个字段就拒。
- **摘要链。** 每份记录记下上游记录的 ID 和内容摘要，还记代码变更集的摘要。代码改了、上游改了、
  验收标准改了，下游记录自动作废。算代码摘要时永远跳过 `workflow/`。
- **证据和说明分开。** `refresh-digests` 只刷新设计、调查、计划这类说明性记录。评审、验收、
  交付是证据，过期只能重做，工具碰到它们直接报错。
- **逐条验收。** 每条 `AC-###` 要有方法和证据两个机器字段，正文里说过不算。缺一个就报
  `acceptance_without_evidence`。
- **两步收口。** 先写 `ready_to_land` 把任务转 `in_review`，落地之后再写 `delivered` 并标 `done`。
- **代码与记录分离提交。** `commit-message` 看暂存区，两者混在一起直接报错。
- **触发保证。** `forward-test.ps1` 在临时仓库里跑一次真实的无头会话，验证新会话会不会自己
  进入工作流。这是对「技能躺在磁盘上不等于会被调用」的直接回应。
- **90 个自动化测试。** `test_workflow.py` 1731 行。

## 两边定位差在哪

这是本次调查最重要的结论：**两者不是同一类工具，功能清单几乎不重叠。**

| | DevFlow | 本仓库 |
| --- | --- | --- |
| 管的东西 | 物理开发环境 | 流程与证据 |
| 核心对象 | workspace、task（执行实例） | Work Item、Attempt、Artifact |
| 状态放哪 | 用户目录下的全局配置和状态文件 | 仓库里的 `workflow/` 目录 |
| 形态 | 编译型 Rust 模块，需要装 | Skill 加 Python 脚本，复制即用 |
| 绑定 | 只服务虚幻引擎项目 | 与语言、引擎无关 |
| 安全模型 | 预演加确认令牌，防误操作 | 摘要链加门禁，防假证据 |
| 谁来判断 | 工具判断，人确认 | 工具查结构，Skill 做判断 |

一一对应能对上的只有三处：

1. `read-status` / `list-tasks` 对应 `status` / `discover`。都是"现在有什么、做到哪一步"。
   但 DevFlow 答的是 junction 挂在哪，本仓库答的是哪份记录还算数。
2. `create-task` 对应 `create-work-item` 加 `create-attempt`。都建一个工作单元并绑一个分支。
   DevFlow 同时把物理环境建出来，本仓库只写一份记录。
3. `finish` 对应 `finish-check` 加 delivery 记录。都表示收尾。DevFlow 的 finish 是把分支并回去，
   本仓库的收口是核对证据还算不算数。

**只有 DevFlow 有的：** worktree 创建与销毁、junction 管理、宿主工程生成、引擎路径探测、
受控编译、UBT 互斥、后台构建跟踪、构建日志、合并策略、分支合并检查、工作区注册表、
损坏任务识别。这些全是虚幻项目的物理环境编排，本仓库一件都没有，也从没打算做。

**只有本仓库有的：** 设计、调查、计划、评审、验收、复盘这六类记录，摘要链与作废判定，
逐条验收证据，两步收口，评审不可改快照，度量账本，写作规范检查，Skill 触发的前向测试。
DevFlow 一件都没有。

## 查到的、推断的、不知道的

**来源直接这么说的：**

- DevFlow 的 15 个 action 写在 `lib.rs:37-53` 的 `ACTIONS` 常量里。
- 确认边界格式写在 `lib.rs:240-242`。
- 全局配置路径写在 `config.rs:99-109`，schema 里也重复声明为 `%USERPROFILE%/.unrealworkflow/devflow`。
- 本仓库明确不做 CLI 产品，写在 `docs/adr/0003`：四个条件出现之前不做，免得命令行提前变成
  中央状态机。
- 本仓库明确不做外部 Tracker 适配，写在 `README.md:64-67`。

**我据此推断的：**

- DevFlow 的「规划子域」没有落地。`CONTEXT.md:8` 声明 DevFlow 内含规划和执行两个子域，
  规划子域负责工作项、依赖关系、优先级和排期。但在 `Source/` 下用 `work_item`、`WorkItem`、
  `work-item` 三个词搜遍所有 `.rs` 文件，一个命中都没有。所以规划子域目前只存在于词汇表，
  代码里只有执行子域。
- `Adapters/DevFlow` 是空目录，说明适配层还没开始写。
- 因此用户提的「功能对齐」，实际要对齐的是 DevFlow **已实现**的那半边，也就是执行环境编排。
  DevFlow 计划做但没做的规划子域，恰好是本仓库已经做完的部分。

**还不知道的：**

- DevFlow 的 15 个 action 有没有跑通的端到端证据。`Tests/` 下只有 `contracts` 和 `fixtures`
  两个目录，`Source/Cli/tests/devflow_native.rs` 的内容没读。
- UnrealWorkflow 另外三个模块（AgentHub、KnowledgeBase、UnrealMaster）是否承担了流程与证据
  职责。本次调查按用户要求只看 DevFlow，没有展开。这会影响"超越"该怎么定义。
- 本仓库的使用者里有多少人在做虚幻项目。这决定了补齐环境编排值不值。

## 有矛盾的地方

`CONTEXT.md` 说 DevFlow 是「规划与执行」两个子域的完整模块，源码只支持「执行」这一半。
两边证据都留着：文档是设计意图，源码是当前事实。做对比时按源码算，做路线判断时要考虑
DevFlow 可能正朝规划子域走，那样两边会在半年内出现真正的重叠。

## 这份调查怎么用

给接下来的设计三个已经定下来的前提：

1. **「功能对齐」不等于把 15 个 action 都实现一遍。** 两者管的是不同层。照抄
   `create-task` 和 `build-task` 意味着本仓库要开始管虚幻引擎路径和 UBT 互斥锁，这跟
   `docs/adr/0003` 直接冲突。
2. **真正值得对齐的是三件事：** 预演加确认的安全模型、动作的机器可读契约、执行环境与流程记录
   的挂接方式。这三件事与虚幻无关，本仓库现在都没有。
3. **「超越」的可能位置在 DevFlow 的空白处。** DevFlow 没有设计、评审、验收、复盘记录，没有
   摘要链，没有作废判定。本仓库已经有了。把这套东西做得能被 DevFlow 这类执行器调用，比反过来
   抄它的 worktree 管理更有价值。

设计阶段要回答的第一个取舍是：本仓库是要长出环境编排能力，还是定义一份让 DevFlow 这类工具
接进来的契约。这两条路的代价差一个数量级。
