---
schema_version: 1
protocol: 1.3.0
artifact: plan
artifact_id: ar_01M07CS7QJSFGHKWX3P2WYRPB1
work_item_id: wi_01M07CNY9RM7B36W3AK11ZVZ7F
created_at: 2026-08-17T08:19:00Z
producer: aes-plan
result: ready
supersedes: null
dependencies:
  work_item_contract_digest: sha256:d444f7a30ab66f3cab02a47ea7ae326e65adacd42265fe8feaf42c8b4353500f
  artifacts:
    - artifact_id: ar_01M07CQNFVDJEE2DGP5Q9YY6EB
      digest: sha256:2b8ce090ac1cb4d232e95fd1e730aaf7db33dadcf53df69a312e2bef74174796
      locator: design.md
---

# 实施计划：创建任务时忽略未跟踪临时文件

## 步骤

| 步骤 | 实施 | 验证 |
| --- | --- | --- |
| 1 | 在 `src/git/mod.rs` 增加只返回 tracked/index/conflict 状态的明确辅助函数，保持 `status_porcelain` 原语义不变；让 `prepare_primary_plans` 使用该函数 | 编译通过；调用点只改变创建任务边界 |
| 2 | 在 `tests/create_sources.rs` 增加“未跟踪文件允许创建”的集成回归，并断言源仓库未跟踪文件仍在、任务 worktree 不含该文件；在 `tests/skills/aes-workflow/test_create_acceptance.py` 接通具名验收用例 | 回退辅助函数后 Rust 回归失败，恢复后通过；AES `verify` 能找到该用例 |
| 3 | 增加或补强 tracked 修改仍拒绝的回归，覆盖“临时文件 + 真实 tracked 修改”组合 | 创建失败且 Host 不存在 |
| 4 | 通过 `tests/skills/aes-workflow/test_create_acceptance.py` 运行格式、Clippy、创建测试和全量测试，记录输出 | 对应 AC-003、AC-004 |

## 依赖

步骤 2、3 依赖步骤 1；步骤 4 依赖全部代码和测试修改完成。

## 风险

| 风险 | 控制 |
| --- | --- |
| 用户把未跟踪源码误以为任务基线的一部分 | 任务仍然只从 `HEAD` 创建；未跟踪文件明确留在主仓库，不会被静默带入 worktree |
| 其他调用需要完整 dirty 状态 | 不修改现有 `status_porcelain`，只新增命名明确的专用函数 |
| 失败创建留下残骸 | 不改变现有 partial-create 回滚；回归测试继续断言 Host 不存在 |
