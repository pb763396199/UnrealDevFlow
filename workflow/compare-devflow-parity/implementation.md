---
schema_version: 1
protocol: 1.3.0
artifact: implementation
artifact_id: ar_01KZ3ET9D7QP28CP1F3ZT13R5E
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
created_at: 2026-08-03T09:20:58.279345Z
producer: aes-execute
result: complete
supersedes: ar_01KZ3EQ2754G79XGQ9D16HVFS8
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts:
    - artifact_id: ar_01KZ37EABQ94G88X046P6WBBTG
      digest: sha256:a2354c27e6025a821cefbd5fd6fa8f273abe337a770534d3065befa0a028d63b
      locator: plan.md
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

## 做完了什么

`unrealdevflow` 现在装一个命令就够用，不再需要装 UWF。UWF 侧领先的引擎增强和受控构建都搬了回来，依赖树里没有任何 `uwf-*` crate。顶层命令从 17 个变成 20 个。

S1 到 S8 各一个提交（S6 按计划拆成两个），另加计划外的 S9 和按评审修复的 S10，基线 `e5fff60`，落在 `feature/compare-devflow-parity` 上。

| 步骤 | 提交 | 改了什么 |
| --- | --- | --- |
| S1 | `c949d82` | `AGENTS.md` 声明本仓库用 AES Workflow |
| S2 | `2e2cd16` | 新增 `src/build_policy.rs`（815 行来源，切断 `uwf_core`），`Cargo.toml` 加 `ipc-lock`、`md-5` |
| S3 | `0dd7479` | `build-check` / `build-gate` / `build-project` 接线，`host::resolve_host_uproject` 提前搬入 |
| S4 | `aa83bc6` | `host/mod.rs`：元数据原子写加 `.bak`、损坏任务清单 |
| S5 | `2221e84` | `switch.rs`：任务项目范围约束、Junction 建后校验、三条只读诊断 |
| S6 | `2e7dc9b`、`7211755` | `build_status.rs` 后台构建对账；`create.rs` 回滚与隔离，连带 `git/worktree.rs` 签名 |
| S7 | `33d7323` | `state.rs` 原子写、`list.rs` 暴露损坏任务、`migration.rs` 不再假设 AesWorld、`workspace.rs` 去掉过严门禁 |
| S8 | `4e301f9` | 锁语义核对（只读）＋ 16 处 `collapsible_if` 清理 |
| S9 | `ebeefd5` | 五份文档补上新命令说明，修掉两处错的帮助文案 |
| S10 | `9dddf37`、`c704c8c` | 按代码评审修掉两条阻断，处理四条建议，并把 `delete` 的守卫改回可 `--force` 放行 |

## 跑了什么，结果如何

三条发布门禁在末版（含评审修复）全部通过：

- `cargo fmt --check` — 退出码 0
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` — 退出码 0
- `cargo test` — 退出码 0，65 个用例全过（单元 34、`create_sources` 15、`merge_yes` 12、`multi_plugin` 4）

每一步做完 `cargo build` 都能过。测试基线从 40 涨到 65，新增 25 个：`build_policy` 6、`host` 6、`switch` 4、`build_status` 2、`create` 1、`state` 1、`migration` 1、`workspace` 1，加上 S3 提前搬入的 `resolve_host_uproject` 3 个。

对真实任务的实测：

- `unrealdevflow build-check neon-dev1/hier-anchor-rebase --format json` 在机器上正跑着 UE 编译时返回 `deferred` / `ubt_mutex_busy`，编译结束后同一条命令返回 `ready` / `build_policy_resolved`。
- `unrealdevflow build-gate "unrealdevflow build-check ..."` 放行、退出码 0；`build-gate "Build.bat DEVEditor Win64 Development -NoMutex"` 拦截、退出码 1。
- `unrealdevflow list`、`workspace doctor neon-dev1` 仍然正常；没有冻结 `TaskContext` 的老任务 `prefab-web-sync` 仍然能解析。
- `cargo tree | grep -i uwf` 零命中；`grep -rn uwf_core src/` 只剩 `build_policy.rs` 顶部一句说明来源的注释。

## 锁语义核对（S8 的正题）

`ubt_mutex_name` 的 MD5 输入只有一样东西：`UnrealBuildTool.dll` 的绝对路径，转大写后按 UTF-16LE 取字节。里面没有产品名，也没有构建目标。`ipc-lock 0.1.4` 在 Windows 上把名字前缀成 `Global\` 再 `CreateMutexW`，跟 UnrealBuildTool 自己用的命名空间一致。

所以同一个引擎目录下，`unrealdevflow` 和 UWF 算出的锁名相同，两边不会各拿一把锁并发跑 UBT。**不需要改锁名算法。**

但核对过程中发现了一个真实缺陷并当场修掉：`resolve_engine_root` 原来用 `std::fs::canonicalize`，在 Windows 上一定带 `\\?\` 前缀。这个前缀有两处害处——它会漏进给人执行的 `Build.bat` 命令行；更要紧的是它进了 MD5 输入，算出来的锁名根本不是 UBT 会拿的那把。修成 `dunce::canonicalize` 之后，上面那组 busy/available 的实测才成立。修在 S3（`0dd7479`），因为它直接决定 `build-check` 的输出对不对。

顺带记一笔：UWF 侧至今仍用 `std::fs::canonicalize`，所以 UWF 的锁探测是失准的。这不影响两边的实际编译互斥——真正的锁是 UBT 自己在编译时拿的，两边都只是调 `Build.bat`——只影响 UWF 那份「现在能不能编」的判断。

## 与计划不一致的地方

**S5 的描述不准。** 计划把 S5 写成「多个主插件各自有 junction 时一次全部切换」，但本地 `switch.rs` 早就这么做了。448 行差异的真实内容是：任务只能切自己绑定的项目、Junction 建完读回校验、三条跨项目诊断、`state` 从每项目存一次改成整轮存一次。按 AC-004「`switch.rs` 的增强可用」施工，没有回退改计划。

**测试数量对不上。** 计划给的 UWF 侧数字（host 11、`build_policy` 5、switch 5、`build_status` 3、create 4、state 2、migration 2、workspace 2）比实际各多一个。实际是 host 10、`build_policy` 4、switch 4、`build_status` 2、create 3、state 1、migration 1、workspace 1。

**计划没有列全需要动的文件。** `git/worktree.rs`、`cleanup.rs`、`delete.rs` 不在任何步骤里，但 S6 要采用 UWF 的 `cleanup_partial_create` 就必须改 `git::worktree::remove` 的签名，连带两个调用点。改了。

**S8 的范围扩大了。** 计划说 S8 只读。实际它还承担了 S2 埋下的 16 处 `collapsible_if` 清理——`ipc-lock` 要求 Rust 1.89，`rust-version` 从 1.85 提上去之后 clippy 才开始建议 let-chain。这批清理故意压到全部手工合并做完再动，免得在 S4–S7 逐文件对照时混进无关噪音。

**`rust-version` 从 1.85 提到 1.89。** 计划没提，但这是 `ipc-lock 0.1.4` 的硬要求，继续声明 1.85 是假话。CI 用 stable，不受影响。

**加了一个 S10 修评审问题。** 代码评审给出 `changes_requested`，两条阻断：`build-project` 会因为 `resolve_uproject` 向上翻而编错项目；v1 元数据下 `cleanup` 和 `delete` 会静默跳过 worktree 与分支清理。都在 `9dddf37` 修掉，同时处理了四条非阻断建议（`build-project` 改流式执行并落 UBT 日志、`build-gate` 拦下 `--mutex no-mutex`、`.bak` 写失败不再阻断主文件、`switch` 校验失败文案如实说明已切走的 Junction），并删掉没有调用点的 `persist_migration_if_needed`。

复核这个修复时又发现它自己的副作用：给 `delete` 加硬守卫等于堵死坏任务的唯一逃生口，元数据缺主插件身份的任务将连删都删不掉，只剩手工改 `.udf-meta.json`。`c704c8c` 把 `delete` 改成警告门——不给 `--force` 报错并说清后果，给了就警告后继续。`cleanup` 保持硬拦，它是合并之后的常规收尾，没有逃生口语义。

**加了一个计划外的 S9。** 计划到 S8 为止，验证条目只写了「`--help` 数命令」。照做之后清点发现：S3 加的三个命令在 `AGENTS.md`、`CLAUDE.md`、`README.md`、`skill/SKILL.md`、`skills/unrealdevflow/SKILL.md` 五份文档里一次都没出现，而 `init` 会把最后那份装到四个 AI provider 那里。也就是说新能力对使用者根本不存在。这是本次改动自己留下的缺口，不是新范围，所以补掉而不是留给下一个任务。同一步顺手修了两处帮助文案说谎：`build --mutex` 的说明写 `nomutex` 但实际只收 `no-mutex`（照着敲直接报 `invalid value`），全局 `--format` 号称通用但只有 5 个命令读它。

## 明确没有采用的东西

**`host::require_frozen_context`（UWF 侧）。** 它在元数据缺 `TaskContext` 时直接报错，要求走「受控迁移」恢复——但两边都没有这样的迁移命令。本机 `prefab-web-sync` 就是一个没有 `context` 的真实任务，采用它等于把这个任务锁死。计划 S4 也写明「旧版元数据仍能读」。保留按任务所在目录推断 workspace 的现有做法。

**`output.rs` 整个文件。** UWF 那套 thread-local 输出捕获是为了让 DevFlow 作为模块把输出交回宿主，独立命令行没有这个宿主。跟着搬还会把 `✓/✗/⚠/ℹ` 换成 `OK:/ERROR:/WARNING:/INFO:`，并把错误从 stderr 挪到 stdout，对命令行是退步。

**`create::effective_task_branch`。** 它在 UWF 那边已经退化成恒等函数，本地也从来没有改写分支名的逻辑。搬过来只是多一层空壳，外加两个测试恒等函数的用例。

**`simple.rs` 的下一步提示、`create.rs` 的错误与提示文案。** UWF 把它们改成了 `uwf dev execute --action ...`，独立工具里该是 `unrealdevflow` 自己的命令，保持本地版本。

## 一处用户可见的行为收紧

`switch <task> --project <其他项目>` 现在会报错。以前可以把任务插件的 Junction 打到任意项目上，结果两个项目共用一个 Host。任务绑定的项目取自元数据里冻结的 `TaskContext.default_project`；`switch main` 不受这条约束。`cli.rs` 里 `--project` 的帮助文本已经改过来了。

命令名没有改动，已发布的接口没有破坏。

## 还剩什么风险

**`build-project` 没有真跑过一次完整编译。** `build-check`、`build-gate` 都用真实任务验证过，`build-project` 只验证了策略解析和参数拼装，没有等一次真实的主项目编译跑完。跑一次要几十分钟，留给验收。S10 之后它改成流式执行并落一份 UBT 日志到 `<project>/Saved/Logs/UnrealDevFlow/`，但这条路径本身也还没实跑过。

**`switch` 的 Junction 校验失败仍然不回滚。** S10 只把错误文案改成如实说明「本项目已经有几个 Junction 被切走、账本还停在上一个任务」，没有自动把它们切回去。真正的回滚需要先想清楚失败时该退回哪个状态，属于新的设计决定。

**`switch` 的三条诊断只有单元测试覆盖。** 跨项目路由、共享 `Plugins` 目录这两条在真实多项目环境下没有实测过。本机只注册了一个 workspace，构造不出场景。

**元数据 `.bak` 的磁盘占用没有回收策略。** 每次 `write_meta` 都会留一份上一版；文件很小，但没有清理逻辑。

**v1 元数据现在会停在 schema 1。** 迁移不再编造主插件身份，所以真有 v1 任务的话，`switch` 会报「没有可切换的 junction」而不是切一个猜出来的 AesWorld。本机没有 v1 任务，这条没有实测。恢复路径（从 Host/git/uproject 证据重建身份）还没有实现，目前只有报错提示。

**20 个命令里我只实操过 10 个。** 跑过的都是只读命令：`list`、`status`、`next`、`build-check`、`build-gate`、`build-status`、`workspace list`、`workspace doctor`、`skills list`、`aw-status`，外加全部命令的 `--help`。写操作命令（`init`、`start`、`create`、`switch`、`build`、`build-project`、`merge`、`cleanup`、`delete`、`finish`、`configure`、`workspace add/remove`、`skills install/remove`）会真的建 worktree、改 Junction、删分支，没有在用户的真实工作区上跑。这些改动的正确性目前只靠单元测试和代码走查，不靠实操。

## 已知但故意留着的问题

这三条是清点 CLI 时发现的，都会动已发布行为，不适合塞进这条已经声明好变更集的路线，应当另立任务。

**`--format` 覆盖面。** 它是全局参数，但 20 个命令里只有 `list`、`status`、`build-check`、`build-gate`、`build-project` 真的读它，其余 15 个照样打人类文本。S9 只把帮助文案改成不说谎，没有扩大覆盖面。要么给所有命令补 JSON 输出，要么把这个参数从全局降为按命令声明，两种都是产品决策。

**`build` 和 `build-project` 的命名是反的。** `build <task>` 编任务宿主，`build-project` 编主项目。「project」在这个工具的其他地方一律指 UE 项目，所以新人会把 `build-project` 当成通用动作、把 `build` 当成主入口。合理的形态是同一个动词带两个宾语，但改名会破坏已发布接口。

**位置参数命名不一致。** `switch`、`build`、`build-status`、`merge`、`cleanup`、`delete` 写 `<TASK_ID>`，`next`、`finish`、`build-check` 写 `<TASK_REF>`，实际上全都接受 `workspace/task-id`。另外 `build-check [TASK_REF]` 可选而 `build-status <TASK_ID>` 必填，两个都是查任务状态。
