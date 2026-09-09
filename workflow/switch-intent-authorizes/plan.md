---
schema_version: 1
protocol: 1.3.0
artifact: plan
artifact_id: ar_01M208TQ1DQRBZE10C46Q7ZJNJ
work_item_id: wi_01M2081ANJ3JCWCFEF3FW3AXAK
created_at: 2026-09-08T00:10:00Z
producer: aes-plan
result: ready
supersedes: ar_01M20875Y1M6M6BG75T93JDVD8
dependencies:
  work_item_contract_digest: sha256:ed781313e35620b5bdabfb7f07de6dba2388dbf6392c4d97baa12ad041ac198a
  artifacts:
    - artifact_id: ar_01M2085HSSGR8CZ8N6CVF178DH
      digest: sha256:21298802c8b1d18c2f94a2696d910092a17a39b9b095d681dd4ba6794d563443
      locator: design.md
---
# 用户提到 switch 时直接执行计划

## 步骤

| 步骤 | 改动 | 文件 | 验证与证据 | 退回办法 |
| --- | --- | --- | --- | --- |
| S1 | 先加入三条回归测试，锁住 agent 授权规则、CLI 不弹确认框、`merge` 与 `cleanup` 规则不变；给 AES `verify` 提供同名用例 | `tests/switch_authorization.rs`、`tests/skills/aes-workflow/test_switch_authorization.py` | 运行 `cargo test --test switch_authorization`，旧实现应因五份入口仍要求二次确认且源码含 `dialoguer::Confirm` 而失败；Python 用例复用相同断言，并能被 `verify` 找到 | 删除两份新增测试文件，生产代码保持原样 |
| S2 | 让 CLI 在编辑器运行时只警告并继续；保留 `--force` 兼容旧脚本，用它省略警告 | `src/commands/switch.rs`、`src/cli.rs` | 运行 `cargo test --test switch_authorization`，确认源码中没有确认对话框且警告仍存在 | 恢复两处 Rust 改动，测试会重新失败 |
| S3 | 把五份 agent 入口统一为“用户表达当前任务的 switch 意图即授权并直接执行”，同时写清纯询问不触发；保留 `merge` 与 `cleanup` 规则 | `AGENTS.md`、`CLAUDE.md`、`.github/copilot-instructions.md`、`skill/SKILL.md`、`skills/unrealdevflow/SKILL.md` | 运行 `cargo test --test switch_authorization`，三条用例全部通过；运行写作检查，错误为零 | 恢复五份入口文件，不改 Rust 行为 |
| S4 | 跑 AES 验收、格式、定向测试、全量测试和静态检查 | 本任务全部改动 | 运行 `workflow_tool.py verify --repo . --work-item switch-intent-authorizes`、`cargo fmt --check`、`cargo test --test switch_authorization`、`cargo test`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`；退出码都为零且三条验收均为 `passed` | 若检查发现回归，回到对应步骤修正后重跑全部检查 |

## 依赖关系

```mermaid
flowchart LR
    S1[回归测试先失败] --> S2[移除 CLI 二次确认]
    S2 --> S3[统一 agent 入口]
    S3 --> S4[完整验证]
```

## 风险

| 风险 | 控制办法 |
| --- | --- |
| agent 把知识询问误当成切换请求 | 五份入口都限定为“表达对当前任务执行 switch 的意图”，并列出纯询问和引用不触发 |
| 旧脚本仍传 `--force` | 保留参数；参数继续省略编辑器运行警告 |
| 顺手放宽 `merge` 或 `cleanup` | 回归测试单独检查策略选择和清理确认文字仍存在 |
