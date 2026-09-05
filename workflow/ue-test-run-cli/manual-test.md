---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01M1R3NQMXWPA586JQHB1HCKF0
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T06:22:12Z
producer: aes-validate
result: passed
supersedes: ar_01M1R1PNTY5SCQ6RK39Q2EMBWB
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts:
    - artifact_id: ar_01M1R3NQVY5WV7TVJ3GZ3D4JZQ
      digest: sha256:1a8cfcda8727a89e8bed37a5aa89b7bc0780d909485a930bcb9be7cfc8fe90f1
      locator: validation.md
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
---

# 人工接手清单

这份清单只验证一个没有继承来源会话历史的接手者能否复用已经登记的配置。接手者不要打开旧会话日志，不要修改候选 JSON，不要手工拼接 UAT 或 Editor 命令。

## AC-024

- [x] 在新会话中运行 `udf --help` 和 `udf run --help`，确认能看到 `run list/plan/start/status`。证据：`target/run-evidence/independent-editor-exit-20260905/01-udf-help.txt`、`02-udf-run-help.txt`。
- [x] 对任务运行 `udf run list --task neon-dev/udf-run-acceptance`，从列表选择 `editor-exit`，没有重新 configure。证据：`target/run-evidence/independent-editor-exit-20260905/03-run-list.json`。
- [x] 运行 `udf run plan editor-exit --task neon-dev/udf-run-acceptance`，确认 project 是验收 Host、入口是 RunUAT/Gauntlet。证据：`target/run-evidence/independent-editor-exit-20260905/04-run-plan.json`。
- [x] 运行 `udf run start editor-exit --task neon-dev/udf-run-acceptance --format json`，得到 `run-editor-exit-20260905T061314498Z-40888-0`，结果为 `state=passed`、`exitResult=passed`、`ueExitCode=0`。证据：`target/run-evidence/independent-editor-exit-20260905/05-run-start.json`。
- [x] 从列表选择 `perflab-game`，运行 `udf run plan perflab-game --workspace neon-dev`，确认地图是 `/Game/Maps/UGA_local/aes6_sh_sz_q1`、窗口为 1366×1024。证据：`target/run-evidence/independent-perflab-game-20260905/04-run-plan.json`。
- [x] 运行 `udf run start perflab-game --workspace neon-dev --format json`，得到 `run-perflab-game-20260905T061627199Z-52964-0`，结果为 `state=started`；status 找到日志，日志出现 `LogLoad: LoadMap` 和目标地图后只关闭了本次 PID 树。证据：`target/run-evidence/independent-perflab-game-20260905/05-run-start.json`、`06-run-status-initial.json`、`07-map-log-evidence.txt`、`08-ended-pid-summary.txt`、`09-run-status-final.json`。
- [x] 已保存两个新的 executionId、status JSON 和日志路径；候选配置字段、revision、digest 未变化。`start` 追加的 `lastValidation` 属于运行观察元数据，不是重新登记配置。证据：两个独立证据目录及 `independent-editor-exit-20260905/08-independent-editor-exit-verdict.json`。

## 通过标准

所有条目均已标 `[x]`，两个案例各有一份新 execution 记录。editor-exit 追加了与同一 revision/digest 绑定的 lastValidation 元数据，候选配置本身没有改变。
