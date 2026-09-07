---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01M07DRF56PHTDJQPBVQ3ZEWE5
work_item_id: wi_01M07CNY9RM7B36W3AK11ZVZ7F
created_at: 2026-08-17T08:42:00Z
producer: aes-validate
result: passed
supersedes: ar_01M07DE271W23AZWJWJJV964R1
dependencies:
  work_item_contract_digest: sha256:d444f7a30ab66f3cab02a47ea7ae326e65adacd42265fe8feaf42c8b4353500f
  artifacts:
    - artifact_id: ar_01M07DQR79CRXM4G43GPSF7290
      digest: sha256:1250fa011893da39b9afb0537b4b392591594d564547d549399e6af3657d2a34
      locator: validation.md
---

## 人工核对清单

<!-- AC-005 -->
- [x] 在本任务执行前后对比真实插件主仓库的 `git status --porcelain` 和索引状态，确认没有新增由本任务造成的内容或索引修改；任务分支仍为 `feature/create-with-untracked-files`。

机器回归和真实仓库只读快照均已通过，AC-005 已闭合。
