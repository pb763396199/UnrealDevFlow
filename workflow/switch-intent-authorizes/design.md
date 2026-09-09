---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01M2085HSSGR8CZ8N6CVF178DH
work_item_id: wi_01M2081ANJ3JCWCFEF3FW3AXAK
created_at: 2026-09-08T00:00:00Z
producer: aes-brainstorm
result: accepted
supersedes: null
dependencies:
  work_item_contract_digest: sha256:ed781313e35620b5bdabfb7f07de6dba2388dbf6392c4d97baa12ad041ac198a
  artifacts: []
---
# 用户提到 switch 时直接执行

## 要解决什么

当前 agent 指南要求用户先提出切换，再等待 agent 复述完整命令并确认一次。CLI 在编辑器运行时还会显示 `Continue with switch?`。同一意图可能被确认两次以上，用户需要重复发令。

## 已确认事实

| 位置 | 当前行为 |
| --- | --- |
| 五份 agent 入口文件 | 包含“不要自己执行”“用户运行”或“即使帮我切也要再确认”等规则 |
| `src/commands/switch.rs` | 编辑器运行时创建 `dialoguer::Confirm`，等待再次确认 |
| `merge` 与 `cleanup` | 分别要求用户选策略和确认清理，本任务不改 |

## 方案对照

| 方案 | 怎么做 | 代价 | 结论 |
| --- | --- | --- | --- |
| 同时修改 agent 指南和 CLI | 用户表达切换意图后由 agent 直接运行；CLI 只警告编辑器仍在运行并继续执行 | 需要同步五份入口文件，并保留旧 `--force` 参数兼容脚本 | 采用，能消除整条调用路径上的重复确认 |
| 只修改 agent 指南 | agent 不再询问，但 CLI 在编辑器运行时仍会询问 | 用户仍可能遇到第二次确认 | 放弃，没有达到用户要求 |
| 保持现状 | 不改任何规则 | 用户继续重复发令 | 放弃，与原始请求冲突 |

## 选定行为

用户消息只要表达对当前任务执行 `switch` 的意图，例如“switch 一下”“帮我切过去”或直接给出 `udf task switch <task-ref>`，agent 就直接执行。纯知识询问、文档引用和未指向当前任务的讨论不算执行意图。

CLI 把显式调用 `udf task switch` 视为授权。编辑器运行时保留风险警告，但不再创建确认对话框。`--force` 为旧脚本保留，继续用于省略编辑器运行警告。

## 边界与失败

- 目标任务无法确定时，agent 先查任务清单；只有存在多个同名候选且无法消歧时才询问目标。
- 编辑器运行不会阻止切换；命令打印切换只会在下次启动编辑器后生效的警告。
- Junction、项目绑定、路径占用和任务元数据检查仍由 CLI 执行，失败时返回原有错误。
- `merge` 的策略选择和 `cleanup` 的确认要求保持原样。

## 影响面

改动覆盖五份 agent 入口文件、一份 Rust 命令实现和一份回归测试。动手前的复算命令：

```powershell
rg -l --glob '!workflow/**' --glob '!docs/brainstorms/**' --glob '!docs/plans/**' "不要自己执行 switch|不要自动 switch|agent 不自己执行 switch|user runs, not agent|必须用户授权|明确授权.*switch|switch.*明确授权|就算用户说了.*帮我切" AGENTS.md CLAUDE.md skill skills .github
rg -n "Continue with switch\?|dialoguer::Confirm" src/commands/switch.rs
```

第一条命中五个文件，第二条在一个文件中命中两处。

## 怎么算做对

- 自动化测试逐份读取五个 agent 入口，确认都写明“用户表达 switch 意图即授权”，且没有旧的二次确认规则。
- Rust 测试和静态检查确认 `task switch` 不再创建确认对话框。
- 现有 `merge`、`cleanup` 测试继续通过。

## 未决问题

没有。用户已经给出授权判据，本设计按该判据实施。
