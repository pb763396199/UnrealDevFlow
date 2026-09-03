---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01M1K29683S5YVZ66ZA7K6R0ZX
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-03T06:30:00Z
producer: aes-review
verdict: approved
supersedes: ar_01M1JP3MJCH9ZSCSRB9CC7XK9D
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
review_type: code
blocking_findings: []
reviewers:
  - implementation boundary and package lifecycle review
---

# 代码评审

## 结论

评审通过。除原有普通项目打包范围、显式高级入口、空间预检、固定 profile、manifest 交付约束外，本轮复核确认任务删除不会再把项目 Junction 残留到 Host/worktree 删除之后。新增修复还覆盖了 state 缺失时从配置项目恢复扫描的路径。

追加复核确认：主仓 dirty 不再作为 `task create` 的硬阻断；任务基线仍固定为当前 `dev` 的提交，未提交内容不会进入 Host。新增回归测试覆盖该行为，未发现阻断问题。

## 检查范围

| 范围 | 结论 |
| --- | --- |
| CLI 路由 | `package project/run/plan/check` 只到项目包；旧 plugin/engine 入口只给迁移提示；高级目标必须写 `package advanced` |
| 交付安全 | 项目包和高级插件包都通过 manifest 与事务日志交付；未拥有文件不覆盖，旧的 UDF 文件按摘要回收，Junction 拒绝接管 |
| 清理边界 | 无参数 clean 只盘点；范围 clean 和 execution clean 只处理受管临时目标；旧记录缺少可信 targetKind 时拒绝猜测删除；Junction 只删除指向当前任务 Host 的直接项目插件入口；metadata 越界路径拒绝执行 |
| 任务生命周期 | `cleanup/delete` 先清项目侧及 Host 内依赖 Junction；按 `symlink_metadata` 识别悬空链接；缺失 Host 可按 workspace Host 根恢复；非当前任务目标和外部依赖目录保留 |
| 空间与版本 | plan/check/project 计算输出、临时目录、预计新增、卷空间和同前缀兄弟版本，并把 blocked 状态挡在 UAT/BuildGraph 之前 |
| 兼容性 | 缺少新增 metadata 的旧 execution record 可读取；旧 cook profile 仍按 full 语义；新 profile 仍按 iterate 语义 |

## 已在评审期间修正的问题

- 高级插件包原先会直接删除已有输出目录，已改为逐插件 manifest 事务交付，并保留每个插件的恢复日志。
- manifest 路径在 stale reconciliation 前增加越界校验，避免篡改清单把删除目标带出输出目录。
- `package clean --task/--workspace` 与无范围盘点分开处理，避免声明支持范围却总是只读。

## 非阻断风险

- 本轮没有启动真实 UE5.5 UAT/Cook。Rust 测试证明命令构造、路由和文件生命周期，不能替代用户指定项目的真实 Shipping 验证。
- 初次没有历史输出时，空间估算使用保守的 1 GiB 基线；真实 UAT 可能高于该值，因此结果是预警和保护，不是精确容量承诺。
- 兄弟版本容量优先读取 UDF manifest；没有 manifest 的人工目录不会被当成 UDF 版本，也不会被清理。

## 验证证据

- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo test --all --no-fail-fast`：通过，62 个单测、19/5/23/4/8/15/6/3/2 个集成测试分别通过。
- CLI smoke：`package --help` 只显示 advanced 高级分组；旧 `package plugin AesWorld` 返回迁移提示且不创建 execution；`package clean --dry-run` 只报告。
- Windows Junction 回归：正常、悬空、缺失 Host 三条路径通过；Host 内依赖目标和其他任务 Junction 均保持不变。
- 新增回归：配置中的主项目没有 state 登记且 Host 目标已先删除时，悬空项目 Junction 仍被删除；其他目标不受影响。
- 新增回归：workspace 路径变化后通过任务冻结 context 和持久化 `task-routes.json` 恢复；恶意 `..` 路径不会删除 Host 外 Junction。
