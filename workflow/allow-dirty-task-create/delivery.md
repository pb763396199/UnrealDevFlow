---
schema_version: 1
protocol: 1.3.0
artifact: delivery
artifact_id: ar_01M1XJDTBETSNSZX7N50064JBV
work_item_id: wi_01M1XFY0XRRPCQKBVK083X2P2Q
created_at: 2026-09-07T09:15:42Z
producer: aes-finish
outcome: delivered
landing_revision: 8439d1ba60b59fa2a5e42374967bb600ac46a1c5
landing_branch: dev
supersedes: ar_01M1XJ7KKET9YQS12NMFQ113BY
dependencies:
  work_item_contract_digest: sha256:3fecfd5988882d94f409d55054f4240248d5ddd042304907e8f45abfcb674e3d
  artifacts:
    - artifact_id: ar_01M1XHJZ6KCH9VENKXZASTWFE4
      digest: sha256:f9a13687b079b9e69481cb5b591368d2a680f1a3742f5b68ce6583891b615440
      locator: reviews/code-review.md
    - artifact_id: ar_01M1XJ7K8BSGXD3YJTW142SSZD
      digest: sha256:e727894262fe6a63e822e8ffe92708d20bbb8eefc823b5ef95a74a75637bb6f5
      locator: validation.md
    - artifact_id: ar_01M1XJ7KDVRJ3RAJ8FAH9AA9X1
      digest: sha256:b13504d05a03c281edd488a6910500a0497715641a611848e06137bd9cc41ec5
      locator: manual-test.md
  subject:
    kind: change_set
    digest: sha256:5d8258a52c4a43d50dedff5b0d4cd574490b9402760e30df26cd3b3f1c7736d3
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 284525b422739bd6b2689bf1e7aca9c08fc1835a
    revision: 8439d1ba60b59fa2a5e42374967bb600ac46a1c5
    tree: 679323326f538cbe96706825a1ee0af4c002458a
    content_digest: sha256:5d8258a52c4a43d50dedff5b0d4cd574490b9402760e30df26cd3b3f1c7736d3
    branch_or_pr: fix/allow-dirty-task-create
    workflow_excluded: true
---
# 交付说明

## 要落地的内容

`task create` 允许 `dev` 主检出保留三类普通修改，并继续拒绝未合并冲突与未完成的 Git 操作。Host 依赖规划和任务 worktree 都读取同一个 `based_on` 提交。

## 证据

- 第二轮独立代码评审批准，0 个问题。
- AES 五条验收全部通过。
- 格式与严格 Clippy 通过。
- 完整测试集 201 passed、0 failed。

## 回滚

1. 在 `dev` 上反向提交 `8439d1b`，恢复依赖规划读取主检出描述符的行为。
2. 再反向提交 `843a440`，恢复已跟踪修改阻断规则和旧文档。
3. 运行 `cargo test --test create_sources`，确认旧行为与测试保持一致。

## 落地结果

`merge-verify` 确认 `dev` 包含落地提交 `8439d1b`，相对任务基线有 8 个文件的真实变更。`finish-check` 返回 `phase: close`、`eligible: true`，代码与已评审变更集完全一致。
