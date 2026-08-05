---
schema_version: 1
protocol: 1.3.0
id: wi_01KZ8TFVAGQNAZH74W8JDZG6GA
short_id: btb303mc
home_repository: "https://github.com/pb763396199/UnrealDevFlow.git"
title: "任务分支名按变更类型分前缀，不再嵌工程名"
status: done
created_at: 2026-08-04T10:20:00Z
created_by: claude
kind: feature
base_revision: 395a2625650bfe9bf9a84483b1be31f6d7363830
---

# 任务分支名按变更类型分前缀，不再嵌工程名

## 原始请求

> 当前有一个实现有问题，那就是分支名的创建，它不应该带工程名，而是应该根据需求来创建或者
> 调用 cli 的人来创建更合适的分支名，是 feature 就是 feature/xxx，是 bug 或者是 refactor
> 都应该有不同的前缀，你按软件行业最先进的时间来设计吧

## 目标

看一眼分支名就知道这次改动是什么性质，而不是知道它属于哪个工程。

## 范围

做：`task create` 生成默认分支名的规则；调用方指定类型或整个分支名的入口；依赖旧命名规则
的那几处代码（孤儿分支清理、合并前的归属判定）；文档与 skill 里的示例。

不做：不改 worktree 布局、Host 目录结构、`.udf-meta.json` 的 schema；不改 Junction 切换；
不动已经存在的任务分支。

## 强约束

- **已有任务不能被弄坏。** 现存 12 个任务的分支名是 `task/<workspace>/<task-id>` 或更早的
  `task-<task-id>`，两种都必须继续能 merge、能 cleanup。
- **孤儿分支清理靠反推分支名。** `cleanup.rs` 的 `expected_branch_names` 在 Host 已经丢失、
  元数据读不到时，只能靠 `(workspace, task-id)` 反推候选分支名。分支名一旦可以自由指定，
  这条路就断了，必须有替代方案。
- **合并前的归属判定不能放松。** `merge.rs` 现在用「分支名是否包含 task-id」挡住误合。
  换命名规则之后仍要有等强度的判定，不能改成无条件放行。
- 分支名必须通过 `git check-ref-format --branch`。

## 验收条件

- AC-001: 不给 `--branch` 时，生成的默认分支名不包含 workspace / 工程名，且带一个表示变更
  类型的前缀。用两个不同 workspace 下同名 task-id 各建一个任务，两边分支名相同。
- AC-002: 调用方能显式决定分支名。至少支持指定类型（由工具补全其余部分），以及直接给完整
  分支名。两条路径都跑通并检查真实创建出来的 git 分支名。
- AC-003: 类型取值是一个受控集合，非法取值在建任务之前就被拒绝，错误信息列出可选值。
- AC-004: 用旧规则建的任务仍能 `task merge` 和 `task cleanup`。用两种历史形态
  （`task/<ws>/<id>` 和 `task-<id>`）各构造一个任务验证。
- AC-005: Host 丢失时的孤儿分支清理仍然有效。构造一个 Host 已删、分支还在的任务，
  `task cleanup` 能找到并清掉它的分支。
- AC-006: `AGENTS.md`、`CLAUDE.md`、`README.md`、`skill/SKILL.md`、
  `skills/unrealdevflow/SKILL.md` 里的分支名示例与新规则一致，逐条能照着敲通。
- AC-007: `cargo fmt --check`、
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、
  `cargo test` 三条退出码都是 0。
