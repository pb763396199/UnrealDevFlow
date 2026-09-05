---
schema_version: 1
protocol: 1.3.0
artifact: validation
artifact_id: ar_01M1R3NQVY5WV7TVJ3GZ3D4JZQ
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T06:22:12Z
producer: aes-validate
outcome: passed
supersedes: ar_01M1R037SYH1G2QZKXK2JPF4KD
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts:
    - artifact_id: ar_01M1QPYQJSMRQ3VT6JK5E65CVM
      digest: sha256:253b56514c88f3db86af66b9047e6b75db8e7944244adae89ace39db12e22cf0
      locator: design/run-native-tools-design-v3.md
    - artifact_id: ar_01M1QW61K3PD4D6D4YDB1X8GC5
      digest: sha256:7f91ee5ee927c680f1ba116f5ea1518dde116db2816abee1b485706018a874b8
      locator: plan.md
    - artifact_id: ar_01M1R037AQKA7EH2JC1880FZGM
      digest: sha256:2c7d5c213081ce5f26d2870baf2ae19897b2f3f5d7480bd6280b5facaed94c77
      locator: change-note.md
    - artifact_id: ar_01M1R037FYYKNP9ZZJB90GFW52
      digest: sha256:2bfc043ac773b47c1688429aca31cedb7ffabcf696daea56eddee4747b312990
      locator: implementation.md
    - artifact_id: ar_01M1R1SCTEG1CEFHEFA14M4YA7
      digest: sha256:3ddfe5cd3ac81a6d8062f333d1d2601886f9571afad48e599f78b4fc7044c21a
      locator: reviews/code-review.md
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
acceptance:
  - acceptance_id: AC-001
    outcome: passed
    method: 对照真实 PowerShell 失败探针和退出案例核对工程地图参数来源
    evidence: research/ue-launch-research.md; research/lifecycle-and-dev-research.md; design/run-native-tools-design-v3.md
  - acceptance_id: AC-002
    outcome: passed
    method: 逐类核对六类运行用途矩阵
    evidence: design/run-native-tools-design-v3.md 六类使用情况
  - acceptance_id: AC-003
    outcome: passed
    method: 核对每类原生入口最小配置结果和缺口处理
    evidence: design/run-native-tools-design-v3.md 职责与六类使用情况
  - acceptance_id: AC-004
    outcome: passed
    method: 运行 list plan check 并检查稳定 JSON 字段
    evidence: tests/run_commands.rs; tests/run_profile.rs; target/run-evidence/cli-config-20260905
  - acceptance_id: AC-005
    outcome: passed
    method: 核对 task context workspace main host 和参数校验
    evidence: src/commands/run.rs; src/run_profile.rs; design/run-native-tools-design-v3.md
  - acceptance_id: AC-006
    outcome: passed
    method: 对照三方案和兼容风险检查选定方案
    evidence: design/run-native-tools-design-v3.md 方案对照
  - acceptance_id: AC-007
    outcome: passed
    method: 运行 workflow 结构校验和中文写作检查
    evidence: workflow_tool.py validate; check_writing.py 输出
  - acceptance_id: AC-008
    outcome: passed
    method: 运行 EditorBootTest 并核对严格退出节点职责
    evidence: research/native-probe-research.md; target/run-evidence/host-20260905/host-start-editor-boot.stdout.json
  - acceptance_id: AC-009
    outcome: passed
    method: 读取真实 start 记录区分 UAT UE 和业务结果
    evidence: target/run-evidence/cli-config-20260905/executions/run
  - acceptance_id: AC-010
    outcome: passed
    method: 对照两次同配置 execution 且不读取快照门禁
    evidence: target/run-evidence/cli-config-20260905; tests/run_lifecycle.rs
  - acceptance_id: AC-011
    outcome: passed
    method: 核对 v0.5.0 configure check plan 约定和 v2 删除项
    evidence: design/run-native-tools-design-v3.md 从 v2 删除或调整的内容
  - acceptance_id: AC-012
    outcome: passed
    method: 用文档和实际 list plan 复核下一会话接手路线
    evidence: README.md; AGENTS.md; CLAUDE.md; skills/unrealdevflow/SKILL.md
  - acceptance_id: AC-013
    outcome: passed
    method: 两个进程复用同名配置并核对 revision digest
    evidence: tests/run_commands.rs; target/run-evidence/cli-config-20260905
  - acceptance_id: AC-014
    outcome: passed
    method: 检查显式 scope 冻结 Host 主项目地图解析和错误停止
    evidence: python tests/skills/aes-workflow/test_run_acceptance.py; tests/run_commands.rs
  - acceptance_id: AC-015
    outcome: passed
    method: 查询前后检查无 execution 和子进程副作用
    evidence: python tests/skills/aes-workflow/test_run_acceptance.py; tests/run_commands.rs; tests/run_lifecycle.rs
  - acceptance_id: AC-016
    outcome: passed
    method: 检查 nativeArgv 地图窗口参数受控开关和路径边界
    evidence: src/run_profile.rs 19 tests; target/run-evidence/shells-game-20260905-r3
  - acceptance_id: AC-017
    outcome: passed
    method: 检查真实 UAT 结果失败单信封和普通 Editor started
    evidence: tests/run_lifecycle.rs; target/run-evidence/host-20260905
  - acceptance_id: AC-018
    outcome: passed
    method: 运行严格退出规则并读取 raw UE code 和 WasKilled
    evidence: target/run-evidence/p1-gauntlet-rules-20260905-r3/runuat.stdout.log; udf-editor-exit.json
  - acceptance_id: AC-019
    outcome: passed
    method: 运行精确 Automation 测试并确认匹配完成和通过
    evidence: target/run-evidence/host-20260905/host-start-automation-smoke.stdout.json; native.log
  - acceptance_id: AC-020
    outcome: passed
    method: 检查缺终态 unknown compare 差异和 pass-after-fail 判定
    evidence: python tests/skills/aes-workflow/test_run_acceptance.py; tests/run_lifecycle.rs; run status and run compare 输出
  - acceptance_id: AC-021
    outcome: passed
    method: 检查已有 Editor PID 候选和未知当前地图
    evidence: python tests/skills/aes-workflow/test_run_acceptance.py; tests/run_existing_editor.rs
  - acceptance_id: AC-022
    outcome: passed
    method: 编译专用 Host 并真实运行 EditorBoot 和 Udf.EditorExit
    evidence: target/run-evidence/host-20260905/summary.json
  - acceptance_id: AC-023
    outcome: passed
    method: 从 PowerShell cmd Git Bash 真实启动 Game 并读引擎地图日志
    evidence: target/run-evidence/shells-game-20260905-r3/summary.json; 三个 map-loaded.txt
  - acceptance_id: AC-024
    outcome: passed
    method: 两个无历史继承的独立会话分别复用 editor-exit 和 perflab-game
    evidence: target/run-evidence/independent-editor-exit-20260905/08-independent-editor-exit-verdict.json; target/run-evidence/independent-perflab-game-20260905/10-verification-summary.json
  - acceptance_id: AC-025
    outcome: passed
    method: 运行全量 Rust 测试 fmt Clippy release build 和验收包装
    evidence: cargo test --workspace --all-targets --all-features --locked; cargo fmt --all -- --check; cargo clippy; cargo build --release --locked; tests/skills/aes-workflow/test_run_acceptance.py
---

# 逐条验收

| 编号 | 结果 | 方法 | 证据 |
| --- | --- | --- | --- |
| AC-001 | passed | 对照真实 PowerShell 裸参数失败和退出案例，核对错误来源与工程/参数选择 | `research/ue-launch-research.md`、`research/lifecycle-and-dev-research.md`、`design/run-native-tools-design-v3.md` |
| AC-002 | passed | 检查 v3 设计的六类用途矩阵 | `design/run-native-tools-design-v3.md`“六类使用情况” |
| AC-003 | passed | 逐类核对原生入口、最小配置、结果和缺口处理 | `design/run-native-tools-design-v3.md`“职责”“六类使用情况” |
| AC-004 | passed | 运行 `run list/plan/check` 并检查 JSON 叶子字段 | `tests/run_commands.rs`、`tests/run_profile.rs`、`target/run-evidence/cli-config-20260905` |
| AC-005 | passed | 核对 task 冻结 context、workspace 显式选择、main/host 与受控参数校验 | `src/commands/run.rs`、`src/run_profile.rs`、`design/run-native-tools-design-v3.md` |
| AC-006 | passed | 核对三方案比较、兼容风险和影响面 | `design/run-native-tools-design-v3.md`“方案” |
| AC-007 | passed | 运行 workflow 结构验证和中文写作检查 | `workflow_tool.py validate`、`check_writing.py` 输出 |
| AC-008 | passed | 先跑 EditorBootTest，再核对自然退出必须由 Udf.EditorExit 节点负责 | `research/native-probe-research.md`、`target/run-evidence/host-20260905/host-start-editor-boot.stdout.json` |
| AC-009 | passed | 读取一次真实 start 记录，区分 UAT、UE 和业务结果 | `target/run-evidence/cli-config-20260905/executions/run/run-editor-exit-20260905T045736974Z-5384-0/record.json` |
| AC-010 | passed | 对照两次同配置真实 execution，且 compare 不读取快照或修改工程 | `target/run-evidence/cli-config-20260905`、`tests/run_lifecycle.rs` |
| AC-011 | passed | 核对 v0.5.0 configure/check/plan 约定及 v2 删除项 | `design/run-native-tools-design-v3.md`“从 v2 删除或调整的内容” |
| AC-012 | passed | 用 README、AGENTS、CLAUDE、skill 和实际 list/plan 复核接手路线 | `README.md`、`AGENTS.md`、`CLAUDE.md`、`skills/unrealdevflow/SKILL.md` |
| AC-013 | passed | 两个进程 configure/list/plan；真实 start 同名配置并记录 revision/digest | `tests/run_commands.rs`、`target/run-evidence/cli-config-20260905` |
| AC-014 | passed | 显式 task/workspace、冻结 Host、主项目和地图解析；错误地图不启动 | `tests/run_commands.rs`、`tests/run_profile.rs` |
| AC-015 | passed | 查询命令前后检查 execution 目录和子进程；run_lifecycle 覆盖查询语义 | `tests/run_commands.rs`、`tests/run_existing_editor.rs`、`tests/run_lifecycle.rs` |
| AC-016 | passed | 检查 nativeArgv 顺序、地图、窗口参数、受控开关拒绝和路径边界 | `src/run_profile.rs` 19 tests、`target/run-evidence/shells-game-20260905-r3` |
| AC-017 | passed | 检查真实 UAT 结果、失败 compare 单信封和普通 Editor started 记录 | `tests/run_lifecycle.rs`、`target/run-evidence/host-20260905` |
| AC-018 | passed | UAT 加载 19 个严格退出规则用例，真实节点导出 raw UE code/WasKilled | `target/run-evidence/p1-gauntlet-rules-20260905-r3/runuat.stdout.log`、`udf-editor-exit.json` |
| AC-019 | passed | 运行精确 Automation 测试并确认匹配、完成和通过结果 | `target/run-evidence/host-20260905/host-start-automation-smoke.stdout.json`、对应 `native.log` |
| AC-020 | passed | status 缺终态返回 unknown，compare 展示字段差异并执行期望判定 | `tests/run_lifecycle.rs`、`run status/run compare` 输出 |
| AC-021 | passed | 已有 Editor 计划只列 PID/actions，未知 currentMap 保持 null | `tests/run_existing_editor.rs` |
| AC-022 | passed | 新 Host 编译、EditorBoot、Udf.EditorExit 和原生报告均成功 | `target/run-evidence/host-20260905/summary.json`、`target/run-evidence/host-20260905/host-start-editor-exit.stdout.json` |
| AC-023 | passed | PowerShell、cmd、Git Bash 各真实启动 Game，三份引擎日志出现目标地图和 1366×1024 argv | `target/run-evidence/shells-game-20260905-r3/summary.json`、三个 `*.map-loaded.txt`、三个 start JSON |
| AC-024 | passed | 两个无历史继承的独立测试会话分别复用 editor-exit 与 perflab-game；核对帮助、list、plan、start、status 和实际日志。配置候选字段、revision、digest 未变；start 只追加 lastValidation 观察元数据 | `target/run-evidence/independent-editor-exit-20260905/08-independent-editor-exit-verdict.json`、`target/run-evidence/independent-perflab-game-20260905/10-verification-summary.json` |
| AC-025 | passed | 全量 Rust 测试、fmt、Clippy、release build、验收包装和范围审查 | `cargo test --workspace --all-targets --all-features --locked`、`cargo fmt --all -- --check`、`cargo clippy ... -D warnings`、`cargo build --release --locked`、`tests/skills/aes-workflow/test_run_acceptance.py`、`reviews/code-review.md` |

## 当前结论

AC-001 至 AC-025 均有可复核证据。两个独立会话都没有重新登记候选配置。editor-exit 的 start 会为同一 revision/digest 写入 lastValidation，这是运行观察元数据，不改变候选配置字段。

## 实验环境

| 项目 | 值 |
| --- | --- |
| UDF | 0.5.0，提交 `6097f4187a766476eae1231eaa8eab0e6499c4e1` |
| UE | `C:\Program Files\Epic Games\UE_5.5` |
| 主项目 | `F:\ShanghaiP4\neon\UGA\DEV\UGA.uproject` |
| 验收 Host | `F:\ShanghaiP4\neon\Hosts\W-neon-dev\T-udf-run-acceptance_Host` |
| 真实地图 | `/Game/Maps/UGA_local/aes6_sh_sz_q1` |
| 证据根 | `target/run-evidence/` 下各独立目录 |
