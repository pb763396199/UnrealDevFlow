---
schema_version: 1
protocol: 1.3.0
artifact: change-note
artifact_id: ar_01M2095V6YGJ8M98C787RY8AMA
work_item_id: wi_01M2081ANJ3JCWCFEF3FW3AXAK
created_at: 2026-09-08T00:30:00Z
producer: aes-execute
result: complete
supersedes: ar_01M208N8K8JAZ8XSXTNB5NJRNN
dependencies:
  work_item_contract_digest: sha256:ed781313e35620b5bdabfb7f07de6dba2388dbf6392c4d97baa12ad041ac198a
  artifacts:
    - artifact_id: ar_01M208TQ1DQRBZE10C46Q7ZJNJ
      digest: sha256:17d0df036b9083d890f4d8e00f65c35f625b80f1d69c2ab35cad3b72585d18e4
      locator: plan.md
  subject:
    kind: change_set
    digest: sha256:9627bb1c6892a900d666ee8df6edfeec4435b0722d8c549961e3da58f501813a
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 42b9a34cb21615af51989a036bf5c4628861f218
    revision: 33e231377b8dedabf74d564368c6ff19f232e5ea
    tree: 76fab3657816be5852e02902a0e736544bf622bd
    content_digest: sha256:9627bb1c6892a900d666ee8df6edfeec4435b0722d8c549961e3da58f501813a
    branch_or_pr: fix/switch-intent-authorizes
    workflow_excluded: true
---
# 修改说明

## 覆盖账本

| 来源 | 代码范围 | 结果 |
| --- | --- | --- |
| AC-001 | 五份 agent 入口、`src/commands/switch.rs`、`src/cli.rs` | 用户表达当前任务的切换意图后直接执行；CLI 只警告，不再询问 |
| AC-002 | 五份 agent 入口、`tests/switch_authorization.rs` | `merge` 策略选择和 `cleanup` 确认文字继续受测试保护 |
| AC-003 | 五份 agent 入口、`tests/switch_authorization.rs` | 旧的“agent 不执行”和“帮我切也要确认”规则已删除 |

## 修改理由

| 修改 | 理由 | 依据 |
| --- | --- | --- |
| 删除 CLI 的 `dialoguer::Confirm` 分支 | 显式调用 `udf task switch` 已经表达执行意图，再询问会造成重复确认 | `design.md` 的选定行为 |
| 保留编辑器运行警告和 `--force` | 用户仍需知道切换在下次启动后生效，旧脚本也不能因参数消失而失败 | `design.md` 的边界与失败 |
| 同步五份 agent 入口 | Codex、Claude Code、Copilot、opencode 和安装后的 skill 必须给出同一规则 | `design.md` 的影响面 |
| 新增 Rust 和 Python 回归测试 | 自动检查授权语义、冲突文字和其他确认边界；让 AES `verify` 能按用例名执行 | `plan.md` 的 S1 |

## 代码范围

- Rust 行为：`src/commands/switch.rs`、`src/cli.rs`。
- Agent 入口：`AGENTS.md`、`CLAUDE.md`、`.github/copilot-instructions.md`、`skill/SKILL.md`、`skills/unrealdevflow/SKILL.md`。
- 回归保护：`tests/switch_authorization.rs`、`tests/skills/aes-workflow/test_switch_authorization.py`。

## 未解释或例外改动

五份入口文件原有 37 处破折号被 AES 提交门禁拒绝。改动将这些标点换成冒号、逗号、分号或明确文字，没有改变对应规则。没有未解释的代码改动。
