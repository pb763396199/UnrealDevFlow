---
schema_version: 1
protocol: 1.3.0
artifact: delivery
artifact_id: ar_01M1K296X3184KK59161PJSG3E
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-03T06:40:00Z
producer: aes-finish
outcome: delivered
landing_revision: 71085c0b785e41317d03c43fa4d9747c58b8c125
landing_branch: dev
supersedes: ar_01M1JP8B0ACZSPERYGWZM3BZ6F
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1K29683S5YVZ66ZA7K6R0ZX
      digest: sha256:ae3c5a4415005963995749d010573e786237371eebf929cba5fd8cf5af0ca724
      locator: reviews/code-review.md
    - artifact_id: ar_01M1K296E4SMC060K71PQ91SB7
      digest: sha256:9e691fd7f0cadee3e485f91849a998accfbe03434ec6cc976bb90668985f566e
      locator: validation.md
    - artifact_id: ar_01M1K296MG12RFN2Y3V9GAQK2J
      digest: sha256:db1856231b76fcbf53250f36b34f382587c0f4f06c371a9e19da4cc5839b8bb8
      locator: manual-test.md
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

# 待落地

代码施工已完成，代码审查和自动验收均已针对 `71085c0` 更新并通过；用户已完成 11 条人工验收清单。目标分支 `dev` 已快进落地，本记录为 `delivered`。

## 交付内容

- `package configure` 保存每个 task 的固定配置，支持 Shipping、容器选择、固定输出和插件排除。
- `package plan/check/project` 消费同一份配置，项目打包使用隔离副本，MCP 等不可打包插件可在副本中禁用。
- 普通项目包与 `package advanced plugin/engine` 分开，旧顶层目标只返回迁移提示。
- package 预检报告输出、临时目录、版本 lineage 和空间决策；成功 full 包自动清理 staging。
- manifest 只授权 UDF 自有文件交付和回收，`package clean` 无参数只盘点。
- `package recover` 只按事务日志恢复未完成交付，保护外部 Junction、用户文件和既有输出；任务 `cleanup/delete` 先清理项目侧、悬空和 Host 内依赖 Junction。
- Junction 清理还会持久化任务路由，拒绝 metadata 越界路径，并覆盖 Host/state/config 同时变化的恢复场景。
- Rust 全量测试、fmt 和 Clippy 严格检查通过；本轮代码提交为 `a28c5e2`，分支为 `feature/task-package-profiles`。

## 交付前人工条件

`manual-test.md` 的 11 条均已勾选；在目标分支完成落地并通过 `merge-verify` 前，任务保持 `in_review`。

## 回滚办法

1. 保留当前基线 `07b7c4b72b2612db3ad0a12ac05fcf89985a8e08` 作为回滚点。
2. 未落地前只需不合入该分支；已落地后使用正常的目标分支回滚流程。
3. 运行中的 package 只用对应 execution ID 的 `package recover` 或 `package clean`，不手动删除固定输出目录和 Junction。
