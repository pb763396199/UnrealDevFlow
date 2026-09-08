---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01M201Z63CWYPKJ2DDRX7HAD0H
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-08T15:34:00+08:00
producer: aes-review
verdict: approved
supersedes: ar_01M1ZXMGS541XQY06PSJMQ2EGP
review_type: code
blocking_findings: []
reviewers:
  - aes-review
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts:
    - artifact_id: ar_01M1Y6N5EPKA8HMNC9E6JGKQS4
      digest: sha256:6eae68193a7ce7c4595f85f1ae2d3658093e6e8829e966d0a212acdc06dd9907
      locator: plan.md
    - artifact_id: ar_01M201Z5S9Z68CST1SDEHDPFGR
      digest: sha256:08151e31596b3c06d9790e555f0b3870dfc43aefb88f9523e40cd5809c10775e
      locator: implementation.md
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

# 代码评审

## 结论

`approved`。插件 stage 现在只链接只读输入，生成目录保持私有，交付摘要验证成功后才删除 stage；未发现阻断问题。

## 核对结果

| 检查点 | 证据 |
| --- | --- |
| 缓存隔离 | `src/package_cache.rs` 的 key 不含 profile revision/name/output/reason；`src/commands/package.rs` 只在可验证字段匹配时迁移旧槽 |
| 活动保护 | lease 同时校验 PID 和 process start time；`src/package_inventory.rs` 还读取 Windows CIM 命令行，当前活动 staging 被盘点为 active |
| 清理边界 | `src/package_inventory.rs` 只扫描固定根和 execution 记录的 cleanupTargets；最终输出、profile、record、无 manifest 输出均 protected，Junction 不递归 |
| 旧记录 | `legacy_target_verified` 要求 execution ID、记录目标和命令行信号同时匹配；没有证据的目录保持 unknown |
| 交付 | `publish_directory` 逐文件核对新摘要后才写 delivered 并删除 backup；复制/校验失败仍停在 delivering |
| 记录一致性 | `package clean` 逐项更新所有引用已删除路径的 execution JSON，不再只修改第一条记录 |
| 回归验证 | `cargo clippy ... -D warnings` 与 `cargo test --locked --all-targets` 均通过 |
| 输入目录 | Source、Content、Config、Resources、Shaders、ThirdParty 是 Junction，插件根不是 Junction |
| 生成目录 | Intermediate、Binaries、Saved 是普通私有目录；Binaries 内部的 Junction 会复制目标内容 |
| 成功清理 | `cleanup_plugin_stage_after_success` 仅在每个插件 manifest 交付完成后运行，并从 cleanupTargets 移除 stage |
| 强制失败清理 | `--force` 要求 execution ID 和 `--yes`；活动任务仍被拒绝；仅跳过失败保护期 |

## 非阻断注意

- 真实机器 inventory 需要遍历约 1 TB 受管数据，dry-run 约需一分钟；这是字节级审计的预期成本。
- 旧版 delivered backup 没有新版 journal 摘要时只能通过 `--legacy` 清理；没有三项证据的日志和 orphan 保留给人工处理。
- 真实 UE 插件包尚需人工运行一次，确认 UBT 只在私有生成目录写入；单元测试已覆盖目录边界。
- 源插件 Binaries 如果本身很大，单次 stage 仍会复制它们。这是预编译 DLL 兼容所需的空间成本。
- 已明确执行 `--force` 的失败现场不可恢复，执行记录保留失败日志与 `manual-clean` 结果。
