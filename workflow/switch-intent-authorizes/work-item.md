---
schema_version: 1
protocol: 1.3.0
id: wi_01M2081ANJ3JCWCFEF3FW3AXAK
short_id: 95t1d211
title: 用户提到 switch 时直接执行
status: done
created_at: 2026-09-08
home_repository: https://github.com/pb763396199/UnrealDevFlow.git
kind: fix
branch_or_pr: fix/switch-intent-authorizes
base_revision: 42b9a34cb21615af51989a036bf5c4628861f218
---
# 用户提到 switch 时直接执行

## 原始请求

当前工程的switch指令太机械了，用户需要三令五申才执行switch，用户都提到switch就不要再让用户二次确认。

## 目标

用户明确提到要对当前任务执行 `switch` 时，agent 直接执行命令，不再要求第二次确认。

## 范围

修改仓库提供给 agent 的 `switch` 使用规则和覆盖该规则的自动化测试。保留命令本身的错误检查，不改变 `merge` 与 `cleanup` 的确认要求。

## 强约束

只有用户消息表达了对当前任务执行 `switch` 的意图时才视为授权。纯粹询问命令含义或引用文档不触发执行。

## 验收条件

- AC-001: 用户表达对当前任务执行 `switch` 的意图后，agent 指南要求直接执行且不再询问确认。
  Verify: case:switch_intent_authorizes_execution
- AC-002: `merge` 策略选择与 `cleanup` 的人工确认规则保持不变。
  Verify: case:other_destructive_confirmations_unchanged
- AC-003: 仓库内不再保留“agent 不要自己执行 switch”或“switch 必须二次授权”的冲突说明。
  Verify: case:no_conflicting_switch_guidance
