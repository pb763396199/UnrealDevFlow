---
schema_version: 1
protocol: 1.3.0
artifact: delivery
artifact_id: ar_01M2025BD1XQKXMSDFT3KST2S4
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-08T15:40:00+08:00
producer: aes-finish
outcome: delivered
landing_revision: b8bede7f7c4fbdfd3cdff714040a854fa7e68f5e
landing_branch: dev
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts:
    - artifact_id: ar_01M201Z5S9Z68CST1SDEHDPFGR
      digest: sha256:08151e31596b3c06d9790e555f0b3870dfc43aefb88f9523e40cd5809c10775e
      locator: implementation.md
    - artifact_id: ar_01M201Z63CWYPKJ2DDRX7HAD0H
      digest: sha256:5f70b4d8b6b3b90d07e1756a67449cb34ae5842d48519d878a31babe8cb04c61
      locator: reviews/code-review.md
    - artifact_id: ar_01M201Z68FQ0GDTFPCQQHBV07Z
      digest: sha256:8c438a0961d8cda1559b7edc13c22698147cac94bcb98b3827f9442d533f1de0
      locator: validation.md
  subject:
    kind: change_set
    digest: sha256:60d56a644472396b2f4495673b45ebb362aa4069f4af3537fa2c9f0eae2e90c5
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 284525b422739bd6b2689bf1e7aca9c08fc1835a
    revision: b8bede7f7c4fbdfd3cdff714040a854fa7e68f5e
    tree: b5718aa1f6f34ec823fc4c9c1c6af9b468650a71
    content_digest: sha256:60d56a644472396b2f4495673b45ebb362aa4069f4af3537fa2c9f0eae2e90c5
    branch_or_pr: fix/package-cache-lifecycle
    workflow_excluded: true
---

# 交付记录

## 落地结果

代码评审为 `approved`，十条验收均为 `passed`，人工清单三条均已勾选。已按用户选择的 `rebase` 处理，并以快进方式落地到 `dev` 的 `b8bede7f7c4fbdfd3cdff714040a854fa7e68f5e`。

## 核对结果

1. `merge-verify` 已确认 `dev` 包含落地提交和实际变更。
2. `finish-check --landing-revision b8bede7` 已返回 `phase: close` 与 `eligible: true`。
3. 回滚方式：将 `dev` 回退到落地前的 `edeb9921bf576dad2f2ba286a99e0fc6a23b19f1`，并重新发布后续修复版本。
