---
schema_version: 1
protocol: 1.3.0
artifact: change-note
artifact_id: ar_01M1GJ71HS5RPB8TS0T4EFVTD8
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-02T08:05:00Z
producer: aes-execute
result: complete
supersedes: ar_01M1GFACNH350X4544P3EQBJ2Z
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1GCKQ5K74MTZJ9FFZK74M54
      digest: sha256:174e055c18b8c7c4af7126964a7b983ad9df997c24c51b8a0891d68b3739089b
      locator: design/package-design-v5.md
    - artifact_id: ar_01M1GD7JRRHR8QXDXHPXFTJ2Q8
      digest: sha256:25c4d285e83cf0d34053512bcad767ed1b3766b4431d6eb7c794e87e1d5eee0d
      locator: plan.md
  subject:
    kind: change_set
    digest: sha256:4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: a817dd2473ebe5fa18ceb0168be9692bf905fcea
    revision: 2d76b395291f8b79b8ba447409a5b6d4dd402c26
    tree: bb896285024e07a65b6c43696026c79e104573f6
    content_digest: sha256:e402fec3b80a318be4aeab9bb0ec3ee423eed114772f8d290b379b9b07dd09ac
    branch_or_pr: feature/task-package-profiles
    workflow_excluded: true
---

# 修改说明

## 覆盖账本

| 阶段 | 预计范围 | 理由 |
| --- | --- | --- |
| 入口与范围 | src/cli.rs、src/main.rs | 普通项目包与显式 advanced 工具链分开，旧目标只迁移不执行 |
| 空间与记录 | src/package_storage.rs、src/commands/package.rs | 记录输出、临时目录、lineage、空间决策和清理结果 |
| 交付与清理 | src/commands/package.rs、src/task_junctions.rs、src/commands/cleanup.rs、src/commands/delete.rs | manifest 摘要交付、旧 UDF 文件回收、最终包保护、范围 clean，以及 Host/worktree 删除前的全量 Junction 收口 |
| 验证与文档 | tests、README.md、skills/unrealdevflow/SKILL.md | 固定 CLI、生命周期和验收入口 |

## 修改理由

- 配置写入来自 design/package-design-v5.md 的固定 profile 方案。
- Shipping、容器格式及禁用插件来自用户确认和 uat-research.md。
- 项目副本、Junction 保护和文件级交付来自 source-research.md 与 delivery-research.md。
- running、退出码和错误分类来自 execution-research.md。
- UAT 参数补全来自本机成功 UAT 日志和 UE5.5 `ProjectParams.cs` 的参数解析。
- Engine 失败清理和 staging 元数据排除来自本轮真实测试中的残留与临时副本证据。
- 日常开发与完整发布的模式区分来自 design/package-design-v4.md，固定在 profile 中而不是临时执行参数中。
- 会话 A 暴露的残留 Junction 来自 cleanup/delete 的生命周期缺口：原实现没有统一清理项目侧链接，并用会跟随目标的 `Path::exists()` 漏判悬空链接；本次将项目侧、Host 内依赖链接和缺失 Host 恢复路径统一收口。

## 当前阶段

施工代码已完成本版计划中的 CLI 范围收敛、空间预检、执行记录扩展、manifest 交付和清理保护，并补上任务删除前的 Junction 生命周期修复。真实 UE UAT 仍由人工清单验收。

## 例外和风险

真实 UE 副本、完整 Shipping Cook 和 Windows 中断交付恢复仍需人工验收；空间初始估算是保护阈值，不是精确资源预测。当前已用 Windows 临时 Junction fixture 验证正常、悬空、缺失 Host、Host 依赖保护及其他任务链接隔离。
