---
schema_version: 1
protocol: 1.3.0
artifact: plan
artifact_id: ar_01KZ37EABQ94G88X046P6WBBTG
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
created_at: 2026-08-03T07:12:06.007258Z
producer: aes-plan
result: ready
supersedes: null
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts:
    - artifact_id: ar_01KZ37AC6DB46XA48QK5HR0CM6
      digest: sha256:13001a2c9e2042233901c92748be41e2d66bfba29f0b7fabab4bf5d5f730abe6
      locator: design.md
---

## 基线

**主干仓库**：`F:\AiProject\UnrealDevFlow`，分支 `feature/compare-devflow-parity`，基线版本
`e5fff6055ae6e214e1566ab02bfd011370aa0232`。所有代码改动在这里做。

**移植来源**：`F:\AiProject\UnrealWorkflow\Source\DevFlow`，UWF 仓库版本 `29adecc`。只读，不改。

**记录仓库**：跟主干同一个，就是 `UnrealDevFlow\workflow\`。`.gitignore` 不忽略它，可以提交。

**测试基线**：`unrealdevflow` 现有约 40 个测试，`tests/create_sources.rs` 15 个、
`tests/merge_yes.rs` 12 个、`tests/multi_plugin.rs` 4 个，`src/` 下 `build.rs` 2 个、
`init.rs` 5 个、`skills.rs` 2 个。UWF 侧有 48 个，其中 `host/mod.rs` 独有 11 个、
`build_policy.rs` 5 个、`switch.rs` 5 个，这些要跟代码一起搬。

**每步都要过的门**：`cargo build` 成功。最后一步之前还要过 `cargo fmt --check`、
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test`。
这三条来自 `UnrealDevFlow\AGENTS.md` 的发布硬规则，不是本计划加的。

## 步骤

### S1 立住入口和基线

**用到**：`AGENTS.md`、`cargo`。

**改**：`AGENTS.md` 加一段声明本仓库用 AES Workflow，开发请求先走 `aes-using-workflow`。
不动发布硬规则那一节。

**变成什么样**：新会话在 `UnrealDevFlow` 里收到开发请求会先进工作流，不会绕过记录。

**验证**：`python <skill>/scripts/workflow_tool.py discover --repo .` 能列出 `47axmb9j`。
跑一遍 `cargo fmt --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、
`cargo test`，把当前结果记下来当基线。

**证据**：三条命令的退出码和 `cargo test` 的用例统计。

**回退**：`git checkout AGENTS.md`。

### S2 移植受控构建引擎，不带 uwf_core

**用到**：`UnrealWorkflow\Source\DevFlow\src\build_policy.rs` 815 行。

**改**：新增 `src/build_policy.rs`。`Cargo.toml` 加 `ipc-lock = "0.1.4"` 和 `md-5 = "0.10"`。
`src/main.rs` 挂上模块但先不接命令。

**变成什么样**：`build_policy.rs` 里所有 `uwf_core` 引用消失。四个 JSON 辅助函数
`json_array`、`json_string`、`json_option` 和手写 JSON 拼接，改成 `serde` 派生加
`serde_json::to_string`。`CommandRequest` 换成本地 struct，字段按 `build-check` 实际要用的取，
不照搬 UWF 的全量字段。`BuildPolicyReport`、`BuildGateReport`、`BuildExecution` 三个结构
加 `#[derive(Serialize)]`。

**验证**：`cargo build` 过。`grep -rn uwf_core src/` 零命中。UWF 侧 `build_policy.rs` 的
5 个单元测试一起搬过来并通过。

**证据**：`cargo build` 与 `cargo test build_policy` 的输出。`grep` 的空结果。

**回退**：删 `src/build_policy.rs`，撤销 `Cargo.toml` 与 `main.rs` 改动。

### S3 接上三个构建命令

**用到**：S2 的 `build_policy.rs`。

**改**：`src/cli.rs` 加 `BuildCheck`、`BuildGate`、`BuildProject` 三个变体。
`src/commands/mod.rs` 与 `src/main.rs` 接线。

**变成什么样**：`unrealdevflow build-check <task>` 输出 `ready`、`needsUserInput`、
`blocked`、`deferred` 之一。`build-gate` 判断某条命令是否绕过受控构建。
`build-project` 编译主项目而不是任务宿主。三个命令都支持 `--format json`。

**验证**：`unrealdevflow --help` 从 17 个命令变成 20 个。对一个真实任务跑 `build-check`，
拿到四种结论之一。

**证据**：`--help` 输出，`build-check --format json` 的实际输出。

**回退**：撤销这一步的提交，`build_policy.rs` 留着不影响构建。

### S4 合并 host/mod.rs

**用到**：UWF 侧 `host/mod.rs` 708 行，本地 366 行。

**改**：`src/host/mod.rs`。

**变成什么样**：任务元数据 schema 从当前版本升到 3。多出跨工作区任务解析、损坏任务识别、
元数据原子写加 `.udf-meta.json.bak` 备份。旧版元数据仍能读，由 `migration.rs` 负责。

**为什么排在前面**：`switch.rs`、`list.rs`、`build_status.rs`、`create.rs` 都调用 `host`，
它是这批合并的地基。这一步没做完，后面四步都动不了。

**验证**：UWF 侧 `host/mod.rs` 的 11 个单元测试搬过来并通过。用一个现有 v2 元数据的任务
跑 `unrealdevflow list`，确认能读出来不报错。

**证据**：`cargo test host` 输出，`list` 对老任务的输出。

**回退**：`git revert` 这一步。注意元数据一旦被写成 schema 3，回退后要靠 `migration.rs`
往回读；测试时用临时工作区，别拿真实任务试。

### S5 合并 switch.rs

**用到**：UWF 侧 `switch.rs` 952 行，本地 504 行。依赖 S4。

**改**：`src/commands/switch.rs`。

**变成什么样**：一个任务的多个主插件各自有 junction 时，`switch` 一次全部切换，
而不是只切第一个。`state.rs` 里的 `junctions` 数组记录全部 junction。

**验证**：UWF 侧 `switch.rs` 的 5 个单元测试搬过来并通过。在临时工作区建一个双主插件任务，
`switch` 后确认两个 junction 都指向任务 worktree。

**证据**：`cargo test switch` 输出，切换后两个 junction 的 `dir` 输出。

**回退**：`git revert` 这一步。junction 是文件系统状态，回退后手工跑一次 `switch` 复位。

**这是本计划最大的单步改动。** 448 行差异里包含真实的行为变化，不是纯重构。改完先跑测试
再手工验证。

### S6 合并 build_status.rs 与 create.rs

**用到**：UWF 侧 `build_status.rs` 189 行、`create.rs` 995 行。依赖 S4。

**改**：`src/commands/build_status.rs`、`src/commands/create.rs`。

**变成什么样**：后台构建进程退出但拿不到退出码时，状态标成 `unknown` 并提示需要重新受控构建
来对账，而不是留在 `building`。`create` 那 99 行差异需要逐段判断：本地 Task#032 和 Task#033
也改过主插件相关逻辑，冲突时以行为为准。

**验证**：UWF 侧 `build_status.rs` 3 个、`create.rs` 4 个单元测试搬过来并通过。
本地 `tests/create_sources.rs` 15 个和 `tests/multi_plugin.rs` 4 个必须仍然全绿——
它们就是 Task#032 和 Task#033 的行为保障。

**证据**：`cargo test` 全量输出，含新旧两批用例。

**回退**：两个文件各自一个提交，可以单独回退。

### S7 合并六个小文件

**用到**：UWF 侧 `state.rs` 149、`migration.rs` 125、`output.rs` 65、`list.rs` 82、
`simple.rs` 107、`workspace.rs` 437。依赖 S4。

**改**：对应六个文件，合计约 158 行差异。

**变成什么样**：`state.rs` 支持多 junction 状态；`migration.rs` 补 schema 3 迁移；
`output.rs` 支持输出捕获；`list.rs` 列出损坏任务；`simple.rs` 的 `next` 提示更细；
`workspace.rs` 注册逻辑对齐。

**注意**：`workspace.rs` 只合并 15 行差异，**不要动本地的 `doctor` 和 `remove` 两个子命令**，
UWF 侧没有它们，合并时容易误删。

**验证**：UWF 侧 `state.rs` 2 个、`migration.rs` 2 个、`workspace.rs` 2 个单元测试通过。
`unrealdevflow workspace doctor <name> --deep` 仍然可用。

**证据**：`cargo test` 输出，`workspace doctor` 的输出。

**回退**：六个文件各自一个提交。

### S8 验证锁语义，核对命令数量

**用到**：S2 的 `build_policy.rs`，本机若装有 UWF 则一并用上。

**改**：不改代码，除非发现冲突。

**变成什么样**：确认 `ipc-lock` 的锁名算法在独立工具和 UWF 之间是否指向同一把锁。
如果指向同一把，两个工具同时编译会互相排队，这是对的；如果指向不同把，两个 UBT 会并发跑，
那是错的，要改锁名算法让两边一致。

**验证**：读 `build_policy.rs` 里锁名的 md5 输入，确认它只由引擎路径和目标决定，
不含产品名。若含产品名则必须改掉。跑 `unrealdevflow --help` 数命令，应为 20 个。
最后跑完整三件套：`cargo fmt --check`、
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test`。

**证据**：锁名计算的输入清单，`--help` 输出，三条门禁命令的退出码。

**回退**：这一步只读，没有回退需求。发现冲突就开新步骤修。

## 顺序和并行

S1 → S2 → S3 是一条链，S2 不完成 S3 无从接线。

S4 是 S5、S6、S7 三步的共同前提，必须先做完。

S5、S6、S7 之间不写同一个文件，理论上可以并行。但 `git` 合并冲突和测试互相影响的风险大于
并行收益，建议串行做，顺序按风险从高到低：S5、S6、S7。

S8 要等前面全部完成。

## 风险

**最大的一处**：S5 的 448 行差异。它不是重构，是行为变化。如果测试搬过来跑不过，
先停下来查清楚 UWF 那边为什么这么改，不要边改边猜。原因不明的失败转 `aes-debug`。

**第二处**：S6 的 `create.rs`。两边都改过，是唯一需要真正做双向判断的文件。

**第三处**：S8 的锁语义。它是设计里点名的最可能出问题的地方，所以单独占一步，不塞进 S2。

## 不做什么

不搬 UWF 的 `dry-run` 加确认令牌机制。不搬 `history` 和 `artifacts` 两个命令，
它们在设计里是未决问题。不动 `unrealdevflow` 已有的 `init`、`configure`、`skills`、
`aw-status` 四个命令。不改任何已发布命令的名字。
