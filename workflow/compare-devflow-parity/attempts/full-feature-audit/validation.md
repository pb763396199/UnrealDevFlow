---
schema_version: 1
artifact: validation
artifact_id: ar_01KZ3F3TNRPR5CTASREF550CFY
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
attempt_id: at_01KZ35WCEAJTX3MNDKH02BCYDS
created_at: 2026-08-03T09:26:10.872230Z
producer: aes-validate
outcome: passed
executed_at: 2026-08-03T09:26:10Z
environment: "Windows 11 Pro 26200; rustc 1.96.1; UE_5.5 at C:/Program Files/Epic Games/UE_5.5; workspace neon-dev1 at F:/ShanghaiP4/neon/UGA/DEV_1"
acceptance:
  - acceptance_id: AC-001
    outcome: passed
    method: "对照三份调查记录与两边源码：动作清单回到 UWF lib.rs:37-52 的 ACTIONS 常量，命令清单回到本仓库 src/cli.rs 的 Commands 枚举，逐条核对对应关系与双方独有项"
    evidence: "workflow/compare-devflow-parity/research/action-command-map-research.md 给出 15 个动作到命令的逐条对应表（15 条全部有对应）、UWF 命令层四项的处置、unrealdevflow 独有的 7 个命令；devflow-capabilities-research.md 给出 15 个动作的职责；devflow-fork-drift-research.md 给出 1956 行引擎差异与双方独有命令"
  - acceptance_id: AC-002
    outcome: passed
    method: "对真实任务跑 build-check，对一正一反两条命令跑 build-gate 并看退出码；再 grep 源码确认没有 uwf_core 引用"
    evidence: "build-check neon-dev1/hier-anchor-rebase --format json => status=ready, reason=build_policy_resolved, mutexStatus=available, exit 0；同一命令在真实编译进行时 => status=deferred, reason=ubt_mutex_busy；build-gate \"Build.bat DEVEditor Win64 Development\" => exit 1 (raw_ue_build_forbidden)；build-gate \"unrealdevflow build-check neon-dev1/x\" => exit 0；grep -rn uwf_core src/ 仅命中 build_policy.rs 顶部注释"
  - acceptance_id: AC-003
    outcome: passed
    method: "在本仓库单独跑 cargo build，再用 cargo tree 全量搜 uwf"
    evidence: "cargo build 退出码 0；cargo tree | grep -i uwf 零命中；Cargo.toml 无 path 依赖，新增依赖仅 ipc-lock 0.1.4 与 md-5 0.10，均为 crates.io 公开包"
  - acceptance_id: AC-004
    outcome: passed
    method: "host/mod.rs、switch.rs、build_status.rs 三处各跑对应单元与集成测试，再对真实数据跑一次只读路径确认无回归"
    evidence: "cargo test host:: => 9 passed（原子写、.bak 备份、空文件与截断回退、双坏报错、损坏任务不被吞）；cargo test switch => 单元 4 passed 加集成 2 passed；cargo test build_status => 2 passed；unrealdevflow list 列出 12 个任务无报错；workspace doctor neon-dev1 返回正常；无 TaskContext 的老任务 prefab-web-sync 仍能被 build-check 解析"
  - acceptance_id: AC-005
    outcome: passed
    method: "先定位两个提交各自的行为保障落在哪些测试上，再跑那些测试，同时用 git diff 核对本次变更有没有动过它们的实现文件"
    evidence: "Task#033 (e5fff60) 新增 tests/multi_plugin.rs 314 行 4 个用例 => cargo test --test multi_plugin 4 passed；Task#032 (979d579) 行为保障为 commands::build::tests 2 个用例 => 2 passed；git diff --stat e5fff60..HEAD -- src/commands/build.rs 输出为空，src/plugin/uplugin.rs 同样未改；cargo test --test create_sources 15 passed"
supersedes: null
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts:
    - artifact_id: ar_01KZ3EW91M84KGH2GHJPBJM4RK
      digest: sha256:201a0e0d22be312e7e992db3bea192a85d44f48b2d10ba75578668442e105eb9
      locator: reviews/code-review.md
  subject:
    kind: change_set
    digest: sha256:f1827294286884456780b511c3b7c8cbfd5dfd73d6a2ecff9226dfc95bd4604a
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: e5fff6055ae6e214e1566ab02bfd011370aa0232
    revision: c704c8c6d1ee4b69420d762126fc4a52cf9e758d
    tree: cf653a656e26d8c7446377d135bdb11fca0dcdd6
    content_digest: sha256:f1827294286884456780b511c3b7c8cbfd5dfd73d6a2ecff9226dfc95bd4604a
    branch_or_pr: feature/compare-devflow-parity
    workflow_excluded: true
---

## 在什么状态上验的

分支 `feature/compare-devflow-parity`，版本 `c704c8c`，基线 `e5fff60`。代码区干净，只有 `workflow/` 未提交。

环境：Windows 11 Pro 26200，rustc 1.96.1，UE_5.5（`C:\Program Files\Epic Games\UE_5.5`），
真实 workspace `neon-dev1`（`F:\ShanghaiP4\neon\UGA\DEV_1`），12 个真实任务。

变更集和实现记录对得上：`git log --oneline e5fff60..HEAD` 列出 11 个提交，与实现记录里的
S1–S10 表格逐条吻合；`git status` 显示代码区无未提交改动。

## 逐条结果

### AC-001 差集表 — passed

**方法**：读三份调查记录，核对是否逐条写明了 UWF 的 15 个动作与 `unrealdevflow` 顶层命令的对应关系，
以及两边各自独有的能力。动作清单回到 `UnrealWorkflow\Source\DevFlow\src\lib.rs:37-52` 的
`ACTIONS` 常量核对，命令清单回到本仓库 `src/cli.rs` 核对。

**证据**：`devflow-capabilities-research.md` 给出 15 个动作各自做什么；
`devflow-fork-drift-research.md` 给出两边各自独有的命令与 1956 行引擎差异；
`action-command-map-research.md` 给出 15 个动作到命令的逐条对应表、UWF 命令层里没有对应的四项
（`build-gate` 已补、`dry-run` 令牌与 `capabilities/commands/schema` 有意不做、`history/artifacts` 未决）、
以及 `unrealdevflow` 独有的 7 个命令。15 个动作全部有对应命令，没有落空。

**说明**：验收时发现前两份调查各覆盖一半，缺的正是「一一对应」那张表，所以本轮补出了
`action-command-map-research.md`。它是任务级调查（`attempt_id: null`），不属于变更集，
补写不影响已通过的代码评审。

### AC-002 独立执行受控构建检查，不引入 uwf-core — passed

**方法**：对真实任务跑 `build-check`，看是否给出四种结论之一；对两条命令跑 `build-gate`，
看放行与拦截的退出码；再确认这条能力不依赖 `uwf-core`。

**证据**：`unrealdevflow build-check neon-dev1/hier-anchor-rebase --format json` 返回
`"status": "ready"`、`"reason": "build_policy_resolved"`、`"mutexStatus": "available"`，退出码 0。
同一条命令在机器上正跑 UE 编译时返回 `"status": "deferred"` / `"ubt_mutex_busy"`，说明结论跟着
真实的 UBT 互斥锁走，不是常量。
`build-gate "Build.bat DEVEditor Win64 Development"` 退出码 1（`raw_ue_build_forbidden`）；
`build-gate "unrealdevflow build-check neon-dev1/x"` 退出码 0。
`grep -rn uwf_core src/` 只命中 `src/build_policy.rs` 顶部一句说明来源的注释，没有代码引用。

### AC-003 单独 cargo build 成功且依赖树无 uwf-* — passed

**方法**：在本仓库单独跑 `cargo build`，再用 `cargo tree` 全量搜 `uwf`。

**证据**：`cargo build` 退出码 0。`cargo tree | grep -i uwf` 零命中。
`Cargo.toml` 的 `[dependencies]` 里没有任何 path 依赖，新增的只有 `ipc-lock 0.1.4` 和 `md-5 0.10`，
两者都是 crates.io 上的公开包。

### AC-004 UWF 侧三处引擎增强可用 — passed

**方法**：三处各跑对应的单元测试，再对真实数据跑一次读路径确认没有回归。

**证据**：
`cargo test host::` — 9 个用例全过，覆盖元数据原子写、`.udf-meta.json.bak` 备份、空文件与截断的回退、
备份与主文件双坏时的报错、损坏任务不被列表吞掉。
`cargo test switch` — 单元 4 个（任务项目范围拒绝、跨项目路由只诊断不改、账本过期时安全接管、
共享 Plugins 目录别名识别与陈旧 state 清理）加集成 2 个（三 Junction 一次切完、冲突时不留半切现场）全过。
`cargo test build_status` — 2 个用例全过，覆盖后台进程退出标 `unknown`、进程还在保持 `building`。
真实数据读路径：`unrealdevflow list` 列出 12 个任务无报错，`workspace doctor neon-dev1` 返回正常，
没有冻结 `TaskContext` 的老任务 `prefab-web-sync` 仍能被 `build-check` 解析。

### AC-005 Task#032 与 Task#033 的行为仍成立 — passed

**方法**：先确认这两个提交各自的行为保障落在哪些测试上，再跑那些测试，同时核对本次变更有没有动过它们的实现文件。

**证据**：
Task#033（`e5fff60`）新增了 `tests/multi_plugin.rs` 314 行，4 个用例：三主插件三 worktree 加共享分支、
多主插件 merge 必须选边且 `--all` 逆序、三 Junction 切换与 delete 全资源回收、switch 冲突原子性。
`cargo test --test multi_plugin` 4 个全过。
Task#032（`979d579`）改的是 `src/commands/build.rs`、`src/cli.rs`、`src/main.rs`、`src/plugin/uplugin.rs`，
行为保障是 `commands::build::tests` 的 2 个用例（`-Module=` 取自 `.uplugin` 而不是插件名、
旧版空 `.uplugin` 回退到插件名）。两个全过。
`git diff --stat e5fff60..HEAD -- src/commands/build.rs` 输出为空，本次没有改动过 Task#032 的主实现文件；
`src/plugin/uplugin.rs` 同样未改。`cli.rs` 与 `main.rs` 只有新增命令变体、接线和帮助文案，没有改动既有变体。
`cargo test --test create_sources` 15 个全过。

## 顺带记下的门禁结果

不属于任何一条 AC，但发布规则要求，在同一版本上跑过：

- `cargo fmt --check` 退出码 0
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 退出码 0
- `cargo test` 退出码 0，65 个用例全过（单元 34、`create_sources` 15、`merge_yes` 12、`multi_plugin` 4）

## 没有覆盖到的

**一次真实的完整 UE 编译没有跑。** `build` 与 `build-project` 的实际编译结果不在本次验收范围内。
五条 AC 都不要求跑通一次完整编译——AC-002 要的是「能执行受控构建检查」，检查本身不启动编译。
但这意味着 `build-project` 从策略解析到 UBT 退出码的整条链路只有代码走查，没有实跑。
这一条已经落进 `manual-test.md` 交给人核对。

**写操作命令没有在真实工作区上跑。** `create`、`switch`、`merge`、`cleanup`、`delete` 会真的建 worktree、
改 Junction、删分支，本次验收只跑了只读命令和自动化测试。这些命令的行为保障来自
`tests/multi_plugin.rs` 与 `tests/create_sources.rs` 的临时工作区 fixture，不是真实环境。
也已落进 `manual-test.md`。
