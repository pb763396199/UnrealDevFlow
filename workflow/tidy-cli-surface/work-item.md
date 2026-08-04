---
schema_version: 1
id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
short_id: svt5c9xd
home_repository: "https://github.com/pb763396199/UnrealDevFlow.git"
title: "整理 unrealdevflow 的命令行表面：命名一致性与 --format 覆盖面"
status: done
created_at: 2026-08-03T09:34:52.135459Z
created_by: codex
selected_attempt: at_01KZ3FM1W8WB4J9MNKW8AQBXGK
---

# 整理 unrealdevflow 的命令行表面：命名一致性与 --format 覆盖面

## 原始请求

> 整理 unrealdevflow 的 CLI 命名与 --format 覆盖面。三个已知问题写在上一条路线（compare-devflow-parity / full-feature-audit）的 implementation.md「已知但故意留着的问题」里：--format 只有 5/20 个命令真的支持、build 与 build-project 的命名是反的、位置参数 TASK_ID 与 TASK_REF 不一致。都会动已发布接口。

## 目标

让人和 AI 都不用记住 20 个命令名，也不用每次敲 13 个字母。

## 范围

做：可执行文件改名为 `udf`；顶层收成 `workspace`、`task`、`build`、`skill` 四个名词组；
删掉 `configure` 和 `start` 两个冗余命令；位置参数统一为 `<TASK_REF>`；
所有叶子命令支持 `--format json`；发布链路与全部文档同步。

不做：不改任何命令的实际行为；不碰受控构建策略、Junction 切换、元数据 schema；
不改产品名和仓库名；不动用户已有的配置与状态。

## 强约束

- 用户已有的配置目录 `~/.unrealdevflow/`、安装目录 `~/.unrealdevflow/bin`、User PATH 项
  和环境变量 `UNREALDEVFLOW_CONFIG_DIR` / `UNREALDEVFLOW_UE_ENGINE_ROOT` 一律不改名。
- 发布资产名 `unrealdevflow-installer.ps1` 不变，README 里那条一行安装命令的 URL 不能失效。
- 旧命令名不保留任何别名。
- 这是破坏性变更，版本跳到 `0.2.0`。

## 验收条件

- AC-001: 可执行文件叫 `udf`；安装之后 `~/.unrealdevflow/bin` 里只有 `udf.exe`，
  遗留的 `unrealdevflow.exe` 被删除；配置目录、安装目录、User PATH 项、两个环境变量名都没有变。
- AC-002: `udf --help` 的顶层只列出 `workspace`、`task`、`build`、`skill` 四个组加 `help`。
- AC-003: 全仓库搜不到 `build-project`、`build-check`、`build-gate`、`build-status`、
  `configure`、`start` 这些旧命令名，也搜不到把 `unrealdevflow` 当可执行文件用的写法；
  Release notes 里的改名对照表是唯一例外。
- AC-004: 每个叶子命令都接受 `--format json` 并输出结构化结果，写操作命令返回它做了什么，
  不是只返回成功。
- AC-005: 所有接受任务引用的位置参数都叫 `<TASK_REF>`，语义统一为
  「`workspace/task-id`，只有一个 workspace 时可省略前缀」；查询类命令省略时按最近任务解析，
  动作类命令必填。
- AC-006: `AGENTS.md`、`CLAUDE.md`、`README.md`、`skill/SKILL.md`、
  `skills/unrealdevflow/SKILL.md` 里的每一条命令示例都能照着敲通。
- AC-007: 现有 65 个测试全绿，`cargo fmt --check`、
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、
  `cargo test` 三条退出码都是 0。
