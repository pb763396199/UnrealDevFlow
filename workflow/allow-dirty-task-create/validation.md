---
schema_version: 1
protocol: 1.3.0
artifact: validation
artifact_id: ar_01M1XJ7K8BSGXD3YJTW142SSZD
work_item_id: wi_01M1XFY0XRRPCQKBVK083X2P2Q
created_at: 2026-09-07T09:12:19Z
producer: aes-validate
outcome: passed
supersedes: ar_01M1XHM2XJ6WZATP27KG602CA0
executed_at: 2026-09-07T09:12:19Z
environment: Windows PowerShell, rustc 1.97.1, Codex isolated worktree
acceptance:
  - acceptance_id: AC-001
    outcome: passed
    method: case:task_create_allows_untracked_main_checkout_changes
    evidence: AES verify 退出码 0，Ran 1 test in 16.576s，未跟踪文件保留且未进入任务 worktree
  - acceptance_id: AC-002
    outcome: passed
    method: case:task_create_allows_unstaged_tracked_main_checkout_changes
    evidence: AES verify 退出码 0，Ran 1 test in 10.395s，未暂存描述符中的额外依赖未影响 Host，主检出状态不变
  - acceptance_id: AC-003
    outcome: passed
    method: case:task_create_allows_staged_tracked_main_checkout_changes
    evidence: AES verify 退出码 0，Ran 1 test in 8.258s，已暂存描述符中的额外依赖未影响 Host，主检出状态不变
  - acceptance_id: AC-004
    outcome: passed
    method: case:task_create_rejects_unmerged_conflicts
    evidence: AES verify 退出码 0，Ran 1 test in 7.461s，冲突仓库被拒绝且没有创建 Host
  - acceptance_id: AC-005
    outcome: passed
    method: case:task_create_dirty_checkout_docs_are_consistent
    evidence: AES verify 退出码 0，Ran 1 test in 5.837s，README、v0.5.0 发布说明和 AI skill 都列出允许状态与阻断状态
dependencies:
  work_item_contract_digest: sha256:3fecfd5988882d94f409d55054f4240248d5ddd042304907e8f45abfcb674e3d
  artifacts:
    - artifact_id: ar_01M1XGPARKH1R28M1SVK82VK7B
      digest: sha256:6df3d9eb605b4b83ae4f51c655eb165b4491ba494171c743c24d7f41144c249c
      locator: debug.md
    - artifact_id: ar_01M1XHJZ6KCH9VENKXZASTWFE4
      digest: sha256:f9a13687b079b9e69481cb5b591368d2a680f1a3742f5b68ce6583891b615440
      locator: reviews/code-review.md
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
# 验收记录

5 条验收标准均由 AES 固定入口在最终变更集上重新执行，结果为 5 passed、0 failed。

全仓验证也已完成：`cargo fmt --all -- --check` 通过，严格 Clippy 通过，`cargo test --all --no-fail-fast` 得到 201 passed、0 failed。

本任务没有需要人工判断的界面或编辑器交互项。
