---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01KZ37AC6DB46XA48QK5HR0CM6
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
created_at: 2026-08-03T07:09:56.813449Z
producer: aes-brainstorm
result: accepted
supersedes: ar_01KZ377N20RN3X16ZTGF3FA276
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts:
    - artifact_id: ar_01KZ36QJHA21RAMT1ZYKXEH9PK
      digest: sha256:a5f61de80993bb8fc8b8731fc1264bd64b4840efe27232a9b70fa406fd35838a
      locator: research/devflow-fork-drift-research.md
---

## 目标

让 `unrealdevflow` 一个命令装完就能用，不用装 UWF。

## 背景

`unrealdevflow` 是一个独立的命令行工具，管虚幻插件的多任务并行开发：每个任务一个 git worktree
加一个宿主工程，用 NTFS Junction 秒级切换，编译隔离，四种合并策略。它有 GitHub Release、
一行安装器、写 PATH、装 AI skill 到四个 provider。

UWF 后来把它整合成了 `uwf-devflow` 模块。整合的做法是保留引擎、砍掉外壳：`cli.rs` 从 482 行
降到 28 行，`init`、`configure`、`skills`、`aw-status` 四个命令模块整体删除，换成 UWF 的模块契约。

此后两边各自演进了一段时间。现在的状态是：

- 共享约 6900 行几乎相同的代码，29 个同名文件里 18 个行数差不超过 5 行。
- UWF 侧多了 1956 行引擎增强，另有 815 行的受控构建 `build_policy.rs`。
- `unrealdevflow` 侧多了 482 行 CLI 加四个命令模块，还领先两个提交。
- git 上两边没有关联，是 fork-and-drift。

事实依据见 [devflow-fork-drift-research.md](research/devflow-fork-drift-research.md)。

## 非目标

- 不改 UWF 的 AgentHub、KnowledgeBase、UnrealMaster。
- 不替 UWF 设计拆分后怎么办。那是 UWF 自己的事，等这边独立跑通再谈。
- 不给 `unrealdevflow` 加任何流程、评审、验收类功能。它只管物理开发环境。
- 不重命名任何已发布的命令。用户装的是 Release 版本。

## 方案对比

**方案一：以 `unrealdevflow` 为主干，把 UWF 的增强搬回来。**
保留现有 CLI、发布链路和四个命令模块，把 1956 行引擎增强加 815 行受控构建移植过去，
移植时把 `uwf_core` 的依赖换成自己的类型。代价是要处理两边同源文件的双向合并，
`switch.rs` 和 `host/mod.rs` 两处改动最大。选它。

**方案二：以 `uwf-devflow` 为主干，重建 CLI 外壳。**
把模块 crate 拆出来单独发布，重新写 482 行 `cli.rs`，重新实现 `init`、`configure`、`skills`、
`aw-status`，重新搭 GitHub Release、安装器和 PATH 注册。引擎侧不用动。没选，因为重建外壳的
工作量大于移植引擎，而且发布链路重建有风险：用户已经在用一行安装器。

**方案三：抽第三个共享 crate，两边都依赖它。**
把 6900 行共享代码抽成 `devflow-core`，`unrealdevflow` 和 `uwf-devflow` 各自作为外壳依赖它。
理论上最干净。没选，因为它没有解决用户要的事：拆出来之后 `unrealdevflow` 仍然要和 UWF 协调
版本，解耦没有真正发生。而且抽 crate 要先让两边收敛到同一份代码，那正是方案一的工作。
方案一做完之后，如果 UWF 还想继续用，再抽 crate 也不迟。

**方案四：什么都不做。**
两边继续各自演进。代价是每次改一处 bug 要改两遍，而且差距只会越来越大。目前已经漂了 1956 行。
没选。

## 选定方案

以 `unrealdevflow` 为主干，分四步把 UWF 的能力搬回来。

**第一步：切断 `uwf_core`。** 只有 `lib.rs` 和 `build_policy.rs` 两个文件引用它，用的是四个
JSON 辅助函数和 `CommandRequest`、`ModuleContract`、`SafetyLevel` 等类型。JSON 辅助函数直接
用 `serde_json` 替掉。类型不照搬：独立工具不需要 UWF 的模块概念，`build-check` 的输出定义成
自己的 struct，用 `serde` 派生序列化。

**第二步：移植受控构建。** `build_policy.rs` 815 行是 UWF 侧最有价值的东西。它解析
`.uproject` 的 `EngineAssociation`，定位 `Build.bat` 和 UBT，算 UBT 互斥锁名，产出
`ready | needsUserInput | blocked | deferred` 四种结论。移植后新增三个命令：
`build-check`、`build-gate`、`build-project`。要一起带的依赖是 `ipc-lock` 和 `md-5`。

**第三步：合并引擎增强。** 按行数差从大到小做：`switch.rs` 加 448 行的多 junction 切换、
`host/mod.rs` 加 342 行的元数据 schema 3 和损坏任务识别、`build_status.rs` 加 94 行的后台
构建对账，然后是 `create.rs`、`state.rs`、`migration.rs` 等六个小改动。每个文件单独一步，
单独验证。

**第四步：对齐命令数量。** 合并后 `unrealdevflow` 有 20 个顶层命令：原有 17 个加新增 3 个。
UWF 的 15 个动作全部有对应命令。UWF 独有的 `dry-run` 加确认令牌机制不照搬，因为 CLI 里
`--dry-run` 和 `-y` 已经承担了同样的职责，再加一层令牌只会让人多敲一遍命令。
`history` 和 `artifacts` 两个读取命令保留在待定，它们读的是 UWF 的任务产物目录，
独立工具里对应什么还没想清楚。

## 边界与失败

**双向合并会遇到真冲突。** 两边都改过 `create.rs`、`delete.rs`、`merge.rs`。UWF 侧改了
99 行，`unrealdevflow` 侧的 Task#032 和 Task#033 也动了主插件相关逻辑。冲突时以行为为准
不以行数为准：先跑两边各自的测试，弄清每处改动要解决什么问题，再决定合并结果。

**受控构建的锁语义可能对不上。** `build_policy.rs` 用 `ipc-lock` 加锁，锁名由 md5 算出。
独立工具里如果同时有 UWF 装着，两个进程会不会抢同一把锁，需要在移植时验证。这是本方案
最可能出问题的地方。

**发布链路不能中断。** 移植过程中 `cargo build` 必须一直能过。每一步做完就能构建，不留
半成品状态。

**回退办法。** 每一步一个提交，独立可回退。第二步和第三步之间是天然的分界：只做完第一、
二步，工具已经能独立构建并且多了受控构建，是一个可交付的中间状态。

## 怎么算做对

- `cargo build` 在 `UnrealDevFlow` 仓库单独跑成功，`cargo tree` 里没有任何 `uwf-*` crate。
- `unrealdevflow build-check` 能对一个真实任务给出四种结论之一。
- `unrealdevflow --help` 列出 20 个命令。
- UWF 侧领先的三处大改动在独立工具里可用：多 junction 切换、损坏任务识别、后台构建对账。
- Task#032 和 Task#033 的行为在合并后仍成立，用两边现有测试验证。

## 未决问题

**一、`history` 和 `artifacts` 两个命令要不要。**
它们在 UWF 里读任务产物目录。独立工具的任务目录结构不同，读什么、给谁看还没想清楚。
选项是照搬、重新定义、或者不做。下一步：等前三步做完，看实际有没有人要这个能力。

**二、UWF 拆掉之后怎么办。**
可能是调用独立二进制，可能是保留精简副本，也可能继续用现在这份。这属于 UWF 的范围，
本任务不做决定。下一步：独立版本能跑通之后再跟 UWF 那边对齐。
