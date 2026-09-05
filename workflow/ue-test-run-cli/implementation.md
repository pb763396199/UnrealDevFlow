---
schema_version: 1
protocol: 1.3.0
artifact: implementation
artifact_id: ar_01M1R037FYYKNP9ZZJB90GFW52
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T05:19:26Z
producer: aes-execute
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts:
    - artifact_id: ar_01M1R037AQKA7EH2JC1880FZGM
      digest: sha256:2c7d5c213081ce5f26d2870baf2ae19897b2f3f5d7480bd6280b5facaed94c77
      locator: change-note.md
    - artifact_id: ar_01M1QW61K3PD4D6D4YDB1X8GC5
      digest: sha256:7f91ee5ee927c680f1ba116f5ea1518dde116db2816abee1b485706018a874b8
      locator: plan.md
  subject:
    kind: change_set
    digest: sha256:fdb7027fe71981b5fcba9d7a45c06c933cb8524dda7a1c1894c189dc1cea4cb4
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 2a516b50dfb8b3d58e5a7962b7c0705c84ba78ce
    revision: 6097f4187a766476eae1231eaa8eab0e6499c4e1
    tree: e1760916841fb034c5b04c1406e99b1dec639dd1
    content_digest: sha256:8d547cc41dda5b6ba67dfed8d70b61ce82e5d2145311d05f6a64671b68481fa4
    branch_or_pr: feature/ue-test-run-cli
    workflow_excluded: true
---

# 实现记录

## 做了什么

把 UDF 的 `run` 入口接到 UE 原生工具。会话先登记用途配置，再用 `list`、`plan`、`check` 核对真实目标，`start` 调用 Editor、Commandlet 或 RunUAT/Gauntlet。每次测试记录配置 revision/digest、项目、引擎、地图、原生 argv、UAT/UE 退出码和报告路径。

## 逐步结果

| 步骤 | 结果 | 证据 |
| --- | --- | --- |
| P0 原生路线 | 通过 | 新 Host `neon-dev/udf-run-acceptance` 编译 496 个动作；`UE.EditorBootTest` 通过参数数组加载并启动 Editor |
| P1 Gauntlet 节点 | 通过 | `target/run-evidence/p1-gauntlet-rules-20260905-r3/runuat.stdout.log`，19 个规则全通过；`Udf.Automation.csproj` 由 UAT 编译加载 |
| P2 配置 | 通过 | `cargo test --locked --bin udf run_profile::tests`，19 passed |
| P3 查询 | 通过 | `cargo test --locked --test run_profile --test run_commands --test run_existing_editor`，6 passed；跨进程 configure/list/plan 无 execution 写入 |
| P4 生命周期 | 通过 | `cargo test --locked --test run_lifecycle`，3 passed；失败数据单 JSON，缺终态为 unknown |
| P5 回归 | 通过 | `cargo test --workspace --all-targets --all-features --locked`，88 个单元测试及全部集成测试通过；Clippy、fmt、release build 通过 |
| P6 Host | 通过 | `target/run-evidence/host-20260905/summary.json`；EditorBoot、Udf.EditorExit、Earth.Elevation.SpatialElementConstruction 全部通过 |
| P6 Shells | 通过 | `target/run-evidence/shells-game-20260905-r3/summary.json`；PowerShell、cmd.exe、Git Bash 均真实启动 Game，三份引擎日志出现目标地图与 1366×1024 参数 |

## 改了哪些代码

- `src/cli.rs`、`src/main.rs`、`src/commands/mod.rs` 注册七个 `run` 叶子命令和显式作用域。
- `src/run_profile.rs` 保存封闭配置、校验受控参数、处理 revision、digest、原子替换和 task/workspace 继承。
- `src/commands/run.rs` 解析冻结 context、解析地图和原生入口、构造参数数组、展开 Gauntlet 资源、执行并记录结果。
- `src/output.rs` 增加带 data 的失败信封，主入口避免重复 JSON。
- `resources/gauntlet/` 提供严格自然退出节点和规则自测；`scripts/test-run-live.ps1` 提供隔离配置、Host 和三种 shell 套件。
- `tests/` 覆盖配置、查询、已有 Editor、生命周期、CLI 分类和 AES 验收包装；README、AGENTS、CLAUDE、两个 skill 文档加入 `run` 用法。

## 自审发现的问题

| 问题 | 发现方式 | 处理 |
| --- | --- | --- |
| 安装版 UE 根目录下还有 `Engine` 层，初版 executable 路径少了一层 | 真实 `run check` 返回 native_tool_missing | 修正为 `Engine/Binaries/Win64` 和 `Engine/Build/BatchFiles`，随后真实 UAT 通过 |
| PowerShell 裸传带点测试名时 UAT 解析成 `UE` | P0 失败探针 | 所有执行使用 argv 数组，三种 shell 计划实测一致 |
| UAT 普通 `ExecCmds` 会追加到节点参数后，出现 `QUIT_EDITOR,stat unit` | 真实 EditorOutput.log | 引入内部 `UdfSyncCmds`，节点先写同步命令再写唯一 QUIT_EDITOR，实机日志为 `stat unit,QUIT_EDITOR` |
| `then_some(matching[0])` 会在条件为 false 时仍先求值 | `run_existing_editor` 进程级回归 | 改成显式长度分支，并纳入全量测试 |
| 失败 compare 原先会以 `ok=true` 输出后再返回非零 | 生命周期回归 | 改为 `emit_failure_with_data`，主进程只输出一个 `ok=false` 文档 |

## 哪里没按计划走

| 偏差 | 原因 | 对验收的影响 |
| --- | --- | --- |
| `Udf.EditorExit` 的同步字段没有直接沿用普通 `-ExecCmds`，改为 `-UdfSyncCmds` | UAT 会把普通字段追加到节点之后，实际退出顺序会反过来 | 增加了一个受控内部参数和黑名单；不改变用户配置字段，真实报告证明顺序正确 |
| P6 先用已有 `cli-config-20260905` 做一次单命令实跑，再用脚本跑 Host/Shells | 需要先验证 CLI 端到端，再验证脚本编排 | 所有后续脚本使用独立 EvidenceRoot，结果没有复用 execution ID |
| 计划原文仍保留“尚未实现”的历史句子 | 施工计划是开始时的固定记录 | 实现和验收记录给出当前代码与证据，收口前由评审确认不把该句当现状 |

## 跑过的检查

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --release --locked
python tests/skills/aes-workflow/test_run_acceptance.py
RunUAT.bat -ScriptDir=<probe> TestGauntlet -Test=Udf.EditorExitRulesTests
pwsh scripts/test-run-live.ps1 -Suite Host ...
pwsh scripts/test-run-live.ps1 -Suite Shells ...
```

## 剩余风险

首版没有 `wait`、`stop` 或后台恢复协议；普通 Editor 的结果只能是 `started`。Game smoke 为确认地图而清理本次 UDF 返回的 PID，不把强制结束当作自然退出通过。已有 Editor 计划只列 PID 和原生操作路线，不发送控制命令。UDF 不把完整项目布局或 DLL 快照作为通过条件，特定业务前提仍需项目自己的 Automation/Gauntlet 测试覆盖。
