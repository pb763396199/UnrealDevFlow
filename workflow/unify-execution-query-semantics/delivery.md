---
schema_version: 1
protocol: 1.3.0
artifact: delivery
artifact_id: ar_01KZZ5FBY8MFV49DH743XGV8RK
work_item_id: wi_01KZZ3WP75XMDZYJ25DBT1DF9R
created_at: 2026-08-14T03:36:30Z
producer: aes-finish
outcome: ready_to_land
supersedes: null
landing_revision: 7765b7bebb54b28e04d49d177b854cb2c44878d3
landing_branch: dev
dependencies:
  work_item_contract_digest: sha256:eaca9f43850bcae5352d32568c68ea2d6aa58a0fea1f8a1d180814ec0701177e
  artifacts:
    - artifact_id: ar_01KZZ4ZMP8FP236HJH40TA1M61
      digest: sha256:1ba373e75e159d91b5f9809894480ed70d50a5c4daa3151e9de1040c594d4c38
      locator: reviews/code-review.md
    - artifact_id: ar_01KZZ4ZMT5FVCBS2WQCD347690
      digest: sha256:3d93feae52a3a17db17dd5fca643235a6cb9385ef551ad4b12c5ed22d6f96a5b
      locator: validation.md
    - artifact_id: ar_01KZZ4ZMXA6G4QNMY6THDCDB06
      digest: sha256:c2595c071bfa15dbb5810a6c4910296fd2622bfb1018b06c3a6c3db62b576952
      locator: manual-test.md
  subject:
    kind: change_set
    digest: sha256:06c8256ad6b1e4f25ca682c86630cebe1a1f3184ff61409a7d6087f5f04c6a38
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 40a81bf7b9b3059bb9f17bca6bff45b7fa55bc5c
    revision: 7765b7bebb54b28e04d49d177b854cb2c44878d3
    tree: 2cb4f276017f1bfe2de7777d1c5e3501491dfd38
    content_digest: sha256:06c8256ad6b1e4f25ca682c86630cebe1a1f3184ff61409a7d6087f5f04c6a38
    branch_or_pr: refactor/unify-execution-query-semantics
    workflow_excluded: true
---

# 交付准备

## 要落地的内容

统一 Build 与 Package 的 check、plan、status 职责，并保留旧命令入口和旧记录显式读取兼容。

## 证据链

- 代码评审：`reviews/code-review.md`，结论 approved。
- 自动验收：`validation.md`，五条标准全部 passed。
- 人工核对：`manual-test.md`，零条人工项，结果 passed。
- 全局安装：`C:\Users\YUMEI\.unrealdevflow\bin\udf.exe` 已由官方 FromSource 安装脚本刷新。

## 回滚办法

1. 将目标分支快进前的版本记为回滚点。
2. 若尚未推送，可把目标分支恢复到该回滚点。
3. 重新从回滚点源码运行 `scripts/install.ps1 -FromSource`，恢复全局二进制和技能。

## 落地判断

变更集、评审、验收和全局安装证据一致，可以快进落地到 dev。
