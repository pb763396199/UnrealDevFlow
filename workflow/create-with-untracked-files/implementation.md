---
schema_version: 1
protocol: 1.3.0
artifact: implementation
artifact_id: ar_01M07D1QYWP0T1SQ4FD3NK8D3E
work_item_id: wi_01M07CNY9RM7B36W3AK11ZVZ7F
created_at: 2026-08-17T08:30:00Z
producer: aes-execute
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:d444f7a30ab66f3cab02a47ea7ae326e65adacd42265fe8feaf42c8b4353500f
  artifacts:
    - artifact_id: ar_01M07CS7QJSFGHKWX3P2WYRPB1
      digest: sha256:9796db88c27dc9485d9ed370215b0198e3318de72836a403efd0f43ddc7533d5
      locator: plan.md
  subject:
    kind: change_set
    digest: sha256:3bc4fd401103f5d97693b47499e5411d60766dfcf716fbb17f75aa592a9cd6eb
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: b605b56e93f2214ec04736f9e0428ae77cd401a4
    revision: 158ef72e4f59f10095116fe8664ab5e770219ada
    tree: 177ee4a86a645134eea451a4da6c0e3cbed5ff27
    content_digest: sha256:3bc4fd401103f5d97693b47499e5411d60766dfcf716fbb17f75aa592a9cd6eb
    branch_or_pr: feature/create-with-untracked-files
    workflow_excluded: true
---

# 实现回执：创建任务时忽略未跟踪临时文件

## 做了什么

在 Git 模块增加 `status_porcelain_tracked`，显式使用 `status --porcelain=v1 --untracked-files=no`；`task create` 的主插件基线检查改用该入口，保留通用 `status_porcelain` 的完整状态语义。

## 逐步结果

1. 在临时 Git 仓库中加入 `workflow/session.md` 等未跟踪内容，创建任务成功，任务 worktree 不包含该目录，源仓库状态保持不变。
2. 在临时 Git 仓库中加入已跟踪文件的暂存修改，同时放入未跟踪临时文件，创建任务被拒绝且没有 Host 残骸。
3. 临时恢复旧的全量状态检查后，放行未跟踪文件的回归测试按预期失败；恢复修复后重新通过。
4. 代码已提交为 `dbf0857` 和 `158ef72`，流程记录保持未暂存，符合代码与 workflow 分离提交约束。

## 改了哪些代码

| 文件 | 改动 |
| --- | --- |
| `src/git/mod.rs` | 新增只检查 tracked/index/conflict 状态的命名辅助函数 |
| `src/commands/create.rs` | 创建任务时使用 tracked-only 检查 |
| `tests/create_sources.rs` | 增加未跟踪文件放行、不复制和 tracked 改动仍拒绝的集成回归 |
| `tests/skills/aes-workflow/test_create_acceptance.py` | 将新增 Rust 回归和质量门禁接入 AES `verify` |

## 自审发现的问题

| 问题 | 发现方式 | 处理 |
| --- | --- | --- |
| 通用 `status_porcelain` 是否应全局改成忽略未跟踪项 | 调用点审计显示当前只有创建命令使用，但未来语义可能不同 | 未改通用函数，新增边界函数降低隐式影响 |
| 是否需要自动 stash 保护用户临时文件 | 设计阶段比较恢复失败、并发和数据接管风险 | 不采用 stash；新 worktree 只从提交点创建 |

## 哪里没按计划走

Rust 测试代码是在实现入口之后补入的，而不是严格先写测试再实现；随后通过回退实现的负向验证确认测试能锁住旧行为，并在恢复后重跑通过。AES 验收适配层是为满足 `Verify` 机器入口新增的计划内接线。除此之外没有偏离计划。

## 跑过的检查

- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo test --test create_sources create_`：21 个创建相关测试通过。
- `cargo test`：49 个单元测试、4 个 CLI taxonomy、21 个 create_sources、5 个 execution contract、22 个 merge/cleanup、4 个 multi_plugin、6 个 package_commands、12 个 package_lifecycle、3 个 query_semantics、2 个 workspace_inspect，共 128 个测试通过。

## 剩余风险

- 尚未在用户真实插件仓库上执行创建；验证均使用临时 Git 仓库，符合任务强约束。
- 当前验收机器无法证明用户是否希望把某个未跟踪源码文件纳入任务基线；实现明确按工具的提交点模型处理，不会静默复制未跟踪文件。
