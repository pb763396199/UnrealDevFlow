---
schema_version: 1
artifact: research
artifact_id: ar_01KZ3F1ACH4KYZ3NAGCXQFJWM5
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
attempt_id: null
created_at: 2026-08-03T09:24:48.657764Z
producer: aes-validate
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts: []
---

## 要回答什么

AC-001 要的是一张能一一对应的差集表：UWF DevFlow 的 15 个动作，和 `unrealdevflow` 的顶层命令，谁对得上谁，各自还独有什么。

已有的两份调查各覆盖了一半。`devflow-capabilities-research.md` 列出了 15 个动作在做什么，
`devflow-fork-drift-research.md` 列出了两边各自独有的命令。缺的是把两个层面对起来的那张表。
这份记录只补这一件事，不重复前两份已经查清的结论。

## 15 个动作对到哪个命令

动作清单取自 `UnrealWorkflow\Source\DevFlow\src\lib.rs:37-52` 的 `ACTIONS` 常量，UWF 版本 `29adecc`。
命令一侧取自本仓库 `src/cli.rs` 的 `Commands` 枚举，版本 `c704c8c`。

| UWF 动作 | `unrealdevflow` 命令 | 对应关系 |
| --- | --- | --- |
| `read-status` | `status` | 一一对应。都读各 UE 项目当前挂着哪个任务、Junction 是否有效 |
| `list-tasks` | `list` | 一一对应。两边都会列出损坏任务，本次合并后对齐 |
| `read-workspaces` | `workspace list` | 对应到子命令 |
| `configure-workspace` | `workspace add` | 对应到子命令。`unrealdevflow` 另有 `init` 和 `configure` 两个入口做同一件事，是 UWF 没有的外壳 |
| `create-task` | `create` | 一一对应。`unrealdevflow` 另有 `start` 作为保存原始需求的小白封装 |
| `next` | `next` | 一一对应 |
| `switch` | `switch` | 一一对应。本次合并后行为也对齐（任务绑定项目、多 Junction 一次切完） |
| `build-task` | `build` | 一一对应。名字不同，语义相同：编任务宿主工程 |
| `build-status` | `build-status` | 一一对应。本次合并后都会对后台构建退出对账 |
| `build-project` | `build-project` | 本次新增，对上了 |
| `build-check` | `build-check` | 本次新增，对上了 |
| `merge` | `merge` | 一一对应，四种策略相同 |
| `finish` | `finish` | 一一对应，都是 merge 的交互式包装 |
| `cleanup` | `cleanup` | 一一对应 |
| `delete` | `delete` | 一一对应 |

15 个动作全部有对应命令，没有落空的。

## UWF 命令层里没有对应的部分

UWF 除了 15 个动作，还有一层「命令」（`lib.rs:78-98`）。这一层是模块契约的产物，不是功能：

| UWF 命令 | `unrealdevflow` 里对应什么 | 判断 |
| --- | --- | --- |
| `build-gate` | `build-gate` | 本次新增，对上了 |
| `dry-run` + `execute` 加确认令牌 | `--dry-run` 与 `-y` | 不照搬。命令行里这两个参数已经承担同样的职责，再加一层 `UnrealWorkflow.DevFlow.<action>.v2` 令牌只会让人多敲一遍 |
| `capabilities` / `commands` / `schema` | `--help` | 不照搬。这三个是给 UWF 宿主读模块契约用的，命令行的等价物是 clap 自带的帮助 |
| `history` / `artifacts` | 无 | 未决。它们读 UWF 的任务产物目录，独立工具里对应什么还没想清楚，设计里列为未决问题 |

## `unrealdevflow` 独有、UWF 完全没有的

| 命令 | 做什么 | 为什么 UWF 没有 |
| --- | --- | --- |
| `init` | 探测并注册 workspace，顺带装 AI skill | 整合进 UWF 时外壳被砍掉了 |
| `configure` | 交互式配置 Hosts 根、插件根、项目、引擎 | 同上 |
| `start` | `create` 的小白封装，强制保存原始需求 | 同上 |
| `workspace doctor` | 校验 workspace 目录，`--deep` 递归查每个插件源 | 同上 |
| `workspace remove` | 注销 workspace | 同上 |
| `skills install/list/remove` | 把 skill 装到 codex、claude、opencode、copilot | 同上 |
| `aw-status` | AgentWatcher 只读探针，隐藏命令 | UnrealDevFlow 侧独有的集成 |

外加整个产品外壳：`cli.rs` 本身、`--format`、`--verbose`、GitHub Release、一行安装器、写 PATH。
UWF 侧的 `cli.rs` 只剩 28 行枚举定义。

## 结论

合并之后，两边的功能差集是这样：

- **UWF 独有的能力全部回流**，15 个动作加 `build-gate` 都有对应命令。
- **仍然只有 UWF 有的**：`dry-run` 确认令牌（有意不做）、`capabilities`/`commands`/`schema`（有意不做）、`history`/`artifacts`（未决）。
- **仍然只有 `unrealdevflow` 有的**：7 个命令加整个发布与安装链路。这些正是它作为独立工具存在的理由。

`unrealdevflow` 顶层命令从 17 个变成 20 个（隐藏的 `aw-status` 计入，`help` 不计入）。

## 证据来源

- `UnrealWorkflow\Source\DevFlow\src\lib.rs:37-52`（`ACTIONS` 常量）与 `78-98`（命令层 `spec` 声明），UWF 版本 `29adecc`，只读未改。
- 本仓库 `src/cli.rs` 的 `Commands` 与 `WorkspaceAction`、`SkillsAction` 枚举，版本 `c704c8c`。
- `unrealdevflow --help` 实测输出，20 行命令（19 个可见命令加 `help`，`aw-status` 因 `hide = true` 不显示）。

## 不知道的

`history` 和 `artifacts` 读的是 UWF 任务产物目录，那个目录在独立工具里没有对等物。要不要做、做成什么样，
取决于有没有人真的要这个能力，本次不做决定。
