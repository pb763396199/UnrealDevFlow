---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01KZ3EW91M84KGH2GHJPBJM4RK
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
created_at: 2026-08-03T09:22:03.444664Z
producer: aes-review
verdict: approved
supersedes: ar_01KZ3EASR43GRPJEYBKJ2BSKG3
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts:
    - artifact_id: ar_01KZ3ET9D7QP28CP1F3ZT13R5E
      digest: sha256:62db35c4cbaa9149d0360fd75f818b4fda76165efaf021786780a6df7290702e
      locator: ../implementation.md
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

## 审查范围

基线 `e5fff60`，末版 `c704c8c`，分支 `feature/compare-devflow-parity`，11 个提交。

本版是对 `ar_01KZ3EASR43GRPJEYBKJ2BSKG3` 的复审。上一版在 `ebeefd5` 上给出 `changes_requested`，列了两条阻断和五条建议。作者在 `9dddf37` 和 `c704c8c` 两个提交里做了修复，本版重新看了修复本身，以及修复有没有带出新问题。

逐个提交核对过文件清单，没有范围外改动混进来：`workflow/` 一次都没进过提交，`docs/` 和 `scripts/` 没动，已发布命令没有改名。三条发布门禁在末版复跑通过，65 个用例全过。

## 两条阻断都已消除

### 1. `build-project` 编错项目 — 已修，核对通过

`src/commands/build_policy.rs:148-158` 现在用 `workspace::find_uproject` 把 `.uproject` 显式解析出来再交给策略层，找不到就报错并指向 `workspace doctor`。传给 `BuildPolicyRequest::main_project` 的是文件路径而不是目录，`resolve_uproject` 的第一分支（`start.is_file()` 且扩展名匹配）直接返回，走不到向上翻的循环。任务分支和 workspace 分支的安全性现在对齐了。

实测 `build-check --workspace neon-dev1 --format json` 的 `projectPath` 从原来的目录变成了 `F:\ShanghaiP4\neon\UGA\DEV_1\UGA.uproject`。

### 2. `cleanup` / `delete` 静默跳过清理 — 已修，且修得比我提的更准

`src/commands/cleanup.rs:232` 加了硬守卫，`primary_plugins` 为空时拒绝执行并说明要先恢复元数据。

`src/commands/delete.rs:141-155` 没有照搬硬守卫，而是改成了警告门：不给 `--force` 报错并写清后果，给了 `--force` 就打警告后继续。这个区分是对的，我上一版没想到——`delete` 是坏任务的逃生口，一律拦死会让缺主插件身份的任务连删都删不掉，用户只剩手工编辑 `.udf-meta.json` 一条路。`cleanup` 是合并之后的常规收尾，没有逃生口语义，硬拦合适。

两处都不再把不完整的清理报告成成功，这是原问题的实质。

## 建议项的处理情况

- **`build-project` 缓冲整份输出且不落日志 — 已修。** `src/build_policy.rs:517-535` 改用 `status()`，输出直接流到终端；通过 `-Log=` 把 UBT 日志写到 `<project>/Saved/Logs/UnrealDevFlow/BuildProject_<profile>_<stamp>.log`。`BuildExecution` 去掉了 `stdout`/`stderr` 两个字段，换成 `log_path`，所以 `--format json` 不会再把整份编译输出塞进一个字符串。这个改法比我建议的干净。
- **`build-gate` 放行 `--mutex no-mutex` — 已修。** `build_policy.rs` 新增 `mutex_mode_argument`，同时认 `--mutex X` 和 `--mutex=X` 两种写法，命中后按 `no_mutex_forbidden` 拦下。新增用例 `going_through_the_tool_does_not_licence_disabling_the_mutex` 覆盖三种写法，并反向确认 `--mutex wait`、`--mutex auto` 仍然放行。实测退出码 1。文档里那条规则现在有机器约束了。
- **`.bak` 写失败阻断主文件 — 已修。** `src/host/mod.rs` 改成降级警告后继续写主文件。
- **`persist_migration_if_needed` 死代码 — 已删。** 连带清掉四个变成未用的导入。
- **Junction 校验失败留下半切现场 — 部分处理。** `src/commands/switch.rs:165-186` 的错误文案现在会报出本项目已有几个 Junction 被切走、都是哪些插件，并给出「重跑同一条 switch」或「`switch main` 全部复位」两条恢复动作。没有做自动回滚。**我接受这个处理**：自动回滚要先定义失败时该退回哪个状态（上一个任务？main？），那是设计决定不是修 bug，不该在收口阶段临时发明。现场如实说清楚，已经把「静默」这个真问题解决了。

## 本版新看的地方

复审重点看了修复有没有引入新问题：

- `find_uproject` 从私有改成 `pub(crate)`，只多了 `commands/build_policy.rs` 一个调用点，没有扩大到 crate 外。
- `cleanup` 的守卫放在 `resolve_task` 之后、`cleanup_orphaned_branches` 兜底分支之外，所以 Host 不存在时走的仍然是原来的孤儿分支清理路径，没有被误伤。
- `delete` 的 `--force` 语义在这里是「接受不完整清理」，跟它原有的「跳过确认」含义不冲突，但确实是同一个开关承担了两件事。这属于参数语义偏窄，不构成缺陷，记在这里供后续命名整理时一并考虑。
- `ubt_log_path` 用 `SystemTime` 取秒级时间戳，同一秒内连续两次 `build-project` 会写同一个日志文件名。实际不可能——一次 UE 编译按分钟计——不算问题。
- 去掉 `BuildExecution` 的 `stdout`/`stderr` 是 JSON 输出契约的变化，但 `build-project` 是本次新增命令，还没有任何已发布版本包含它，没有兼容性负担。

## 结论

`approved`。两条阻断已消除，五条建议里四条已修、一条（Junction 回滚）以说清现场的方式合理收敛。

## 没有覆盖的范围

- **一次真实的完整编译仍然没有跑过。** `build-project` 改成流式执行加落日志之后，这条路径本身也还没实跑。`-Log=` 参数、`Saved/Logs/UnrealDevFlow` 目录创建、退出码传递都只经过代码走查。这是本次审查最大的未覆盖面。
- **`cleanup` / `delete` 的空主插件守卫没有集成测试。** 两处守卫是从代码路径确认的，没有构造 v1 元数据的 fixture 实跑。本机没有 v1 任务。
- **多项目环境下的三条 switch 诊断**仍然只有单元测试。
- **`ipc-lock` 引入的 `windows 0.62` 依赖树**对 release 体积和编译时长的影响没有测量。

## 我确认过没有问题的地方（沿用上一版结论）

- **UBT 锁名**。MD5 输入只有 UBT dll 的绝对路径，不含产品名或构建目标；`dunce::canonicalize` 的修正是必要的，busy/available 双向实测能对上。
- **元数据原子写与回退**。只让能解析的主文件覆盖备份，主文件坏了回退到备份并写回，6 个用例覆盖空文件、截断、双坏三种情形。
- **Task#032 / Task#033 的行为保障**。`tests/create_sources.rs` 15 个与 `tests/multi_plugin.rs` 4 个在末版全绿。
- **不采用 `require_frozen_context` 的判断成立**。本机 `prefab-web-sync` 确实没有 `context` 字段，两边都不存在所谓「受控迁移」命令。
- **`switch` 的项目范围收紧**是有意的行为变化，只作用于任务分支，`switch main` 不受限，帮助文本已说明。
