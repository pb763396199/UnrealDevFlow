---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01M1XJ7KDVRJ3RAJ8FAH9AA9X1
work_item_id: wi_01M1XFY0XRRPCQKBVK083X2P2Q
created_at: 2026-09-07T09:12:19Z
producer: aes-validate
result: passed
supersedes: ar_01M1XHM34SAMFQYSP5X6Y66GC6
dependencies:
  work_item_contract_digest: sha256:3fecfd5988882d94f409d55054f4240248d5ddd042304907e8f45abfcb674e3d
  artifacts:
    - artifact_id: ar_01M1XJ7K8BSGXD3YJTW142SSZD
      digest: sha256:e727894262fe6a63e822e8ffe92708d20bbb8eefc823b5ef95a74a75637bb6f5
      locator: validation.md
---
# 人工核对

没有人工核对项。三类主检出修改、任务 worktree 内容、Host 依赖规划、主检出状态、冲突拒绝和文档一致性都由临时 Git 仓库中的自动化测试直接验证。
