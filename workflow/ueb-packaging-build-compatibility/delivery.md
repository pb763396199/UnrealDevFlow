---
schema_version: 1
protocol: 1.3.0
artifact: delivery
artifact_id: ar_01KZXF4RHVPCG94F66FPSNTEPY
work_item_id: wi_01KZWHP5WGDDBVMW0WVRJTAJZE
created_at: 2026-08-13T11:10:00Z
producer: aes-finish
outcome: delivered
landing_revision: df02180b3ec0e11d3584bde3214eb073f5b05316
landing_branch: dev
supersedes: ar_01KZXF0RT9TF73809MQEQCE9CF
dependencies:
  work_item_contract_digest: sha256:57b10aa47e84304ef4ecf7f309eea2b72f99bc3f110ea3a68a92e08c803731b4
  artifacts:
    - artifact_id: ar_01KZXF0RM7P1CG7JKN30BZ7C22
      digest: sha256:8027660d9da7046e19c42744a58da798ee5015c186ec7a6cebe6158c9d60a807
      locator: validation.md
    - artifact_id: ar_01KZXF0RQBWYCWW5VP6V3DBQZ1
      digest: sha256:48315e87f04390663ec4c30e93fe08f6c73458916edfb48a32a5c7475ac376bc
      locator: manual-test.md
  subject:
    kind: change_set
    digest: sha256:3d13fc82e5166b396e5d98a2ade107529797777faa244e8ce76e9b341bafdc41
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 5138373f7a15d01c95a06022c153f73c8be7b3e6
    revision: df02180b3ec0e11d3584bde3214eb073f5b05316
    tree: af0bd4ffee4f0295817642cae110f872a093ef26
    content_digest: sha256:3d13fc82e5166b396e5d98a2ade107529797777faa244e8ce76e9b341bafdc41
    branch_or_pr: research/ueb-packaging-build-compatibility
    workflow_excluded: true
---

# 待落地

代码、评审、六条验收标准和零项人工清单都已闭合。准备把当前分支快进合入 `dev`。

回滚时在 `dev` 上反向提交本任务的四个代码提交。插件打包产物和 staging 位于 `UnrealDevFlow` 受管目录，可按 execution ID 清理，不需要修改 AesWorld 源码。
