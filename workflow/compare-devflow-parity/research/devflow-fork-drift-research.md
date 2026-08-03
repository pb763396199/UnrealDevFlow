---
schema_version: 1
artifact: research
artifact_id: ar_01KZ36QJHA21RAMT1ZYKXEH9PK
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
attempt_id: null
created_at: 2026-08-03T06:59:40.714363Z
producer: aes-research
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts: []
---

## 要回答什么

用户澄清了方向：UWF 的 DevFlow 本来就是从 `unrealdevflow` 整合进去的，现在要把它拆回独立工具。
这份调查回答四个能被证伪的问题：

1. 两边是不是同源？拿文件级证据说话，不看文档说法。
2. 命令面差多少？逐条列出双方独有的命令。
3. 引擎能力差多少？拿行数和文件差异说话。
4. 拆出来要切断多少依赖？

查到能列出完整差集、并说清依赖点在哪就停。不评估代码质量，不做性能测试。

## 看了什么

- `F:\AiProject\UnrealDevFlow`，7759 行 Rust，`Cargo.toml` 声明 `unrealdevflow` v0.1.2，
  edition 2024，2026-08-03 读取。dev 分支 HEAD 为 `e5fff60`。
- `F:\AiProject\UnrealWorkflow\Source\DevFlow`，9627 行 Rust，crate 名 `uwf-devflow`，
  edition 2021，同日读取。UWF 仓库 HEAD 为 `29adecc`。
- 两边的 `git log`、`git remote`、`git branch -a`、`git diff --stat`。

## 两边同源，证据在文件级

两个目录下有 29 个同名 `.rs` 文件。其中 18 个行数差不超过 5 行：

`editor.rs` 79/79、`error.rs` 132/132、`git/mod.rs` 546/546、`host/uproject.rs` 34/34、
`junction/mod.rs` 84/84、`junction/validator.rs` 36/36、`plugin/mod.rs` 35/35、
`plugin/scanner.rs` 151/151、`plugin/uplugin.rs` 130/130、`commands/finish.rs` 53/53、
`commands/status.rs` 86/86、`config.rs` 357/356、`build_profile.rs` 103/101、
`commands/build.rs` 369/365、`commands/cleanup.rs` 350/349、`commands/delete.rs` 430/429、
`commands/merge.rs` 422/425、`git/worktree.rs` 154/149。

**这不是两个工具，是同一个工具的两个分叉。** 共享代码约 6900 行。

git 上两边没有关联。UWF 仓库的 remote 是 `pb763396199/UnrealWorkflow`，UnrealDevFlow 的
remote 是 `pb763396199/UnrealDevFlow`。`Source/DevFlow` 由 UWF 仓库自己跟踪，是一份导入后
独立演进的副本。

有一个容易误认的地方：`UnrealWorkflow\.plugin-worktrees\DevFlow` 的 `.git` 文件写着
`gitdir: F:/AiProject/UnrealDevFlow/.git/worktrees/DevFlow`，它确实是 UnrealDevFlow 仓库的
worktree。但那是 UWF 用 DevFlow 自己的 worktree 机制挂进来的开发视图，跟 `Source/DevFlow`
不是同一份代码。UnrealDevFlow 上的 `unrealworkflow/devflow` 分支也不是整合分支，它从
`e20acc4` 分出，只多一个 AgentWatcher 提交，且落后 dev 两个提交。

## 命令面差多少

`unrealdevflow` 是带 `[[bin]]` 的可执行程序，17 个顶层命令，另有两组子命令。
`uwf-devflow` 是库 crate，对外 11 个命令加 15 个动作。

**只有 `unrealdevflow` 有的：**

| 命令 | 做什么 |
| --- | --- |
| `init` | 首次配置：探测并注册工作区，顺带装 AI skill |
| `configure` | 交互式配置 Hosts 根、插件根、项目路径、引擎路径 |
| `workspace doctor` | 校验工作区目录是否有效，`--deep` 递归校验每个插件源 |
| `workspace remove` | 注销工作区 |
| `skills install/list/remove` | 把 skill 装到 codex、claude、opencode、copilot 四个 provider |
| `aw-status` | AgentWatcher 只读状态探针，隐藏命令 |

还有整个产品外壳：命令行入口本身（`cli.rs` 482 行）、`--format json/human`、`--verbose`、
GitHub Release 发布、一行安装器、写入用户 PATH。UWF 侧的 `cli.rs` 只剩 28 行，是几个枚举定义。

**只有 `uwf-devflow` 有的：**

| 能力 | 做什么 |
| --- | --- |
| `build-check` | 只检查构建策略，不启动编译 |
| `build-gate` | 拦截绕过受控构建的调用 |
| `build-project` | 编译主项目而不是任务宿主 |
| `dry-run` 加确认令牌 | 危险动作必须先预演，再带匹配的 `UnrealWorkflow.DevFlow.<action>.v2` |
| `history`、`artifacts` | 读任务历史与产物 |
| `capabilities`、`commands`、`schema` | 机器可读的自身契约 |

支撑这些的是 `build_policy.rs`，815 行，UnrealDevFlow 侧完全没有这个文件。它解析
`.uproject` 的 `EngineAssociation`，定位 `Build.bat` 和 UBT，算 UBT 互斥锁名，产出
`ready | needsUserInput | blocked | deferred` 四种结论。

## 引擎能力差多少

**UWF 侧领先的部分**，按行数差排序：

| 文件 | UnrealDevFlow | uwf-devflow | 差 | 增强了什么 |
| --- | --- | --- | --- | --- |
| `build_policy.rs` | 无 | 815 | +815 | 受控构建策略 |
| `commands/switch.rs` | 504 | 952 | +448 | 多 junction 切换 |
| `host/mod.rs` | 366 | 708 | +342 | 任务元数据 schema 3、跨工作区解析、损坏任务识别、原子写加备份 |
| `commands/create.rs` | 896 | 995 | +99 | 建任务 |
| `commands/build_status.rs` | 95 | 189 | +94 | 后台构建退出对账 |
| `state.rs` | 103 | 149 | +46 | 多 junction 状态 |
| `migration.rs` | 94 | 125 | +31 | 元数据迁移 |
| `output.rs` | 36 | 65 | +29 | 输出捕获 |
| `commands/list.rs` | 63 | 82 | +19 | 损坏任务列举 |
| `commands/simple.rs` | 89 | 107 | +18 | next 提示 |
| `commands/workspace.rs` | 422 | 437 | +15 | 工作区注册 |

合计约 1956 行增强。

**UnrealDevFlow 侧领先的部分：** `cli.rs` 多 454 行，另有 `init.rs`、`configure.rs`、
`skills.rs`、`aw_status.rs` 四个命令模块 UWF 侧没有。dev 分支还多两个提交：
`979d579` Task#032 修正主插件模块编译与诊断入口，`e5fff60` Task#033 修复多主插件全生命周期一致性。

## 拆出来要切断多少依赖

**很少。** `uwf-devflow` 的 `Cargo.toml` 里只有一条内部依赖 `uwf-core = { path = "../Core" }`。
在 `src/` 下搜 `uwf_core`，只有两个文件命中：

- `lib.rs` 用了 `json_array`、`json_string`、`Capability`、`CommandRequest`、`CommandSpec`、
  `ModuleContract`、`ModuleId`、`SafetyLevel`。
- `build_policy.rs` 用了 `json_array`、`json_option`、`json_string`、`CommandRequest`。

也就是 JSON 序列化辅助函数加命令契约类型。`commands/`、`git/`、`host/`、`junction/`、
`plugin/`、`config.rs`、`state.rs` 这些真正干活的模块一处都没引用。

其余依赖两边基本一致，版本有小差：`dirs` 5 对 6，`sysinfo` 0.32 对 0.33。UWF 侧多了
`ipc-lock 0.1.4` 和 `md-5 0.10`，都是 `build_policy.rs` 用来算互斥锁名和加锁的。

## 查到的、推断的、不知道的

**来源直接这么说的：**

- `unrealdevflow` 的 17 个命令写在 `src/cli.rs` 的 `Commands` 枚举里。
- `uwf-devflow` 的 15 个动作写在 `lib.rs:37-53` 的 `ACTIONS` 常量里。
- 两边共享 6900 行的判断来自逐文件行数比对，命令见调查过程。
- 依赖点只有两个文件，来自对 `Source/DevFlow/src/` 全目录搜索 `uwf_core`。

**我据此推断的：**

- UWF 整合时的做法是：保留引擎，砍掉产品外壳，换成模块契约。证据是 `cli.rs` 从 482 行降到
  28 行，且 `init/configure/skills/aw_status` 四个模块整体消失，而 `git/mod.rs` 546 行两边一字不差。
- 拆回独立工具的主要工作不是搬代码，是**双向合并**。两边都各自往前走了：UWF 加了受控构建和
  多 junction，UnrealDevFlow 加了两个 Task 提交。单向覆盖任一边都会丢东西。
- 以 `unrealdevflow` 为主干比以 `uwf-devflow` 为主干省事。前者已有 CLI 外壳、GitHub Release、
  安装器、PATH 注册和四个 provider 的 skill 安装，这些重建一遍成本高于把 1956 行引擎增强搬过去。

**还不知道的：**

- UWF 侧那 1956 行增强有没有测试覆盖。`Source/Cli/tests/devflow_native.rs` 没读。
- UWF 拆掉 DevFlow 之后自己怎么办。是改为调用独立二进制，还是保留一份精简副本。这属于 UWF
  的事，本次按用户划定的范围没有展开。
- `build_policy.rs` 依赖的 `ipc-lock` 和 `md-5` 在独立工具里是否需要同样的锁语义。

## 有矛盾的地方

`UnrealWorkflow\CONTEXT.md` 说 DevFlow 含「规划」和「执行」两个子域，规划子域负责工作项、
依赖图、优先级和排期。但在 `Source/` 下搜 `work_item`、`WorkItem`、`work-item` 三个词，
所有 `.rs` 文件零命中。两边证据都留着：文档是设计意图，源码是当前事实。拆分时按源码算，
规划子域不存在，不用管。

## 这份调查怎么用

给设计三个已经定下来的前提：

1. **主干选 `unrealdevflow`。** 它有完整产品外壳和发布链路，UWF 侧只有引擎增强。反过来做要
   重建 482 行 CLI 加四个命令模块加发布链路。
2. **要搬的是 1956 行引擎增强，加 815 行受控构建。** 其中 `build_policy.rs` 是唯一需要改写的
   文件，因为它 import 了 `uwf_core`。改写工作量小，替换的是 4 个 JSON 辅助函数和 1 个类型。
3. **命令数量对齐要双向。** `unrealdevflow` 需要新增 `build-check`、`build-gate`、`build-project`
   三个命令，才能覆盖 UWF 独有能力。UWF 独有的 `dry-run` 加确认令牌机制，在 CLI 里已有
   `--dry-run` 和 `-y` 两个标志承担类似职责，是否照搬需要设计阶段判断。

设计阶段要回答的第一个取舍是：`build_policy.rs` 的契约层怎么处理。照搬 `ModuleContract` 会
把 UWF 的模块概念带进独立工具，不搬则要重新定义 `build-check` 的输出格式。
