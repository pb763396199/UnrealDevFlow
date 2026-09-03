---
schema_version: 1
protocol: 1.3.0
artifact: validation
artifact_id: ar_01M1K296E4SMC060K71PQ91SB7
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-03T06:35:00Z
producer: aes-validate
outcome: passed
executed_at: 2026-09-03T06:35:00Z
environment: Windows PowerShell, Rust workspace at F:\\AiProject\\UnrealDevFlow-worktrees\\task-package-profiles
acceptance:
  - acceptance_id: AC-001
    outcome: passed
    method: manual:核对调查中的日志路径、计数及三项用户更正
    evidence: manual-test.md 中 AC-001 已由用户勾选为 [x]
  - acceptance_id: AC-002
    outcome: passed
    method: manual:检查所有调查票均有证据与结论，地图没有遗漏问题
    evidence: manual-test.md 中 AC-002 已由用户勾选为 [x]
  - acceptance_id: AC-003
    outcome: passed
    method: manual:阅读设计中的配置生命周期及命令示例
    evidence: manual-test.md 中 AC-003 已由用户勾选为 [x]
  - acceptance_id: AC-004
    outcome: passed
    method: manual:逐项核对正常执行、失败、中断和来源变化场景
    evidence: manual-test.md 中 AC-004 已由用户勾选为 [x]
  - acceptance_id: AC-005
    outcome: passed
    method: manual:核对文件计数命令、兼容表与验证矩阵
    evidence: manual-test.md 中 AC-005 已由用户勾选为 [x]
  - acceptance_id: AC-006
    outcome: passed
    method: manual:检查本任务代码 diff、实现记录和真实执行记录
    evidence: manual-test.md 中 AC-006 已由用户勾选为 [x]
  - acceptance_id: AC-007
    outcome: passed
    method: case:project_settings_parse
    evidence: Python acceptance 1 test OK；profile baseline and drift test passed
  - acceptance_id: AC-008
    outcome: passed
    method: case:native_argv_mapping
    evidence: Python acceptance 1 test OK；native project argv test passed
  - acceptance_id: AC-009
    outcome: passed
    method: case:cook_reuse_matrix
    evidence: Python acceptance 1 test OK；saved profile plan and reuse compatibility test passed
  - acceptance_id: AC-010
    outcome: passed
    method: case:stale_profile
    evidence: Python acceptance 1 test OK；project settings drift test passed
  - acceptance_id: AC-011
    outcome: passed
    method: manual:记录 UAT ExitCode、产物、配置快照和清理结果
    evidence: manual-test.md 中 AC-011 已由用户勾选为 [x]
  - acceptance_id: AC-012
    outcome: passed
    method: case:package_help_describes_cook_modes
    evidence: Python acceptance 1 test OK；CLI help test passed
  - acceptance_id: AC-013
    outcome: passed
    method: case:cook_mode_is_fixed_per_profile
    evidence: Python acceptance 1 test OK；fixed profile test passed
  - acceptance_id: AC-014
    outcome: passed
    method: case:project_package_scope_is_explicit
    evidence: Python acceptance 1 test OK；legacy plugin entrypoint returned migration guidance without execution
  - acceptance_id: AC-015
    outcome: passed
    method: case:package_disk_and_lineage_guard
    evidence: Python acceptance 1 test OK；clean inventory、manifest reconciliation 与 Junction 生命周期回归通过；新增无 state 登记、配置漂移、任务路由恢复和 Host 外路径保护回归通过
supersedes: ar_01M1JKZ39J9N72D167JMEYW3M4
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1K2962MV7BJDAWW3KPG1TAP
      digest: sha256:fbf05cef7ed2bf8c6c0b77436ffd7729b2a8d558ec9f5f9e0cd5bdd322b2d1fd
      locator: implementation.md
    - artifact_id: ar_01M1K29683S5YVZ66ZA7K6R0ZX
      digest: sha256:fd7e2911b5e9db0160c1c9ccf706fcc37f37acfbf3a40b918ed686cdf3fe9067
      locator: reviews/code-review.md
    - artifact_id: ar_01M1K296MG12RFN2Y3V9GAQK2J
      digest: sha256:fac97c789e2f66714469a25cbc9ec3b11170961ec25ad26b81e01713b9a7fe11
      locator: manual-test.md
  subject:
    kind: change_set
    digest: sha256:4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: a817dd2473ebe5fa18ceb0168be9692bf905fcea
    revision: 71085c0b785e41317d03c43fa4d9747c58b8c125
    tree: 08907cbb20d394efbb6ec06b13d3fcca9454c266
    content_digest: sha256:169f8e6829e36f04a2286d4b419f80c9abdd76033d5d043a05d1f8eb599b7055
    branch_or_pr: feature/task-package-profiles
    workflow_excluded: true
---

# 验收记录

## 结果

自动可验证的代码、CLI 条件和 Junction 生命周期回归已通过；用户已完成 11 条人工验收清单，当前验收记录为 `passed`。

## acceptance

| 编号 | 方法 | 证据 | 结果 |
| --- | --- | --- | --- |
| AC-001 | `manual:核对调查中的日志路径、计数及三项用户更正` | 尚未替人复核 | `not_run` |
| AC-002 | `manual:检查所有调查票均有证据与结论，地图没有遗漏问题` | 尚未替人复核 | `not_run` |
| AC-003 | `manual:阅读设计中的配置生命周期及命令示例` | 设计已落盘，尚未替人复核 | `not_run` |
| AC-004 | `manual:逐项核对正常执行、失败、中断和来源变化场景` | Windows cleanup/delete fixture 覆盖悬空链接与缺失 Host；人工中断恢复未执行 | `not_run` |
| AC-005 | `manual:核对文件计数命令、兼容表与验证矩阵` | 尚未替人复核 | `not_run` |
| AC-006 | `manual:检查本任务代码 diff、实现记录和真实执行记录` | 实现记录已写，尚未替人复核 | `not_run` |
| AC-007 | `case:project_settings_parse` | Python acceptance 1 test OK，底层 `configure_inherits_project_packaging_baseline_and_detects_changes` 通过 | `passed` |
| AC-008 | `case:native_argv_mapping` | Python acceptance 1 test OK，底层 `project_profile_uses_native_project_settings_for_package_args` 通过 | `passed` |
| AC-009 | `case:cook_reuse_matrix` | Python acceptance 1 test OK，底层 profile plan/reuse compatibility test 通过 | `passed` |
| AC-010 | `case:stale_profile` | Python acceptance 1 test OK，底层 project settings drift test 通过 | `passed` |
| AC-011 | `manual:记录 UAT ExitCode、产物、配置快照和清理结果` | 本轮未启动真实 UE UAT | `not_run` |
| AC-012 | `case:package_help_describes_cook_modes` | Python acceptance 1 test OK，CLI taxonomy test 通过 | `passed` |
| AC-013 | `case:cook_mode_is_fixed_per_profile` | Python acceptance 1 test OK，fixed profile test 通过 | `passed` |
| AC-014 | `case:project_package_scope_is_explicit` | Python acceptance 1 test OK，旧 plugin 入口返回迁移提示且无 execution | `passed` |
| AC-015 | `case:package_disk_and_lineage_guard` | Python acceptance 1 test OK，clean inventory、manifest reconciliation 与 Junction 生命周期测试通过；新增无 state 登记、配置漂移、任务路由恢复和 Host 外路径保护测试通过 | `passed` |

## 机器验证

- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo test --all --no-fail-fast`：通过，67 个单元测试及全部集成测试通过。
- Junction 回归：`cleanup_missing_host_removes_dangling_project_junctions_before_branch_cleanup`、`delete_removes_dangling_project_junctions_before_host_and_worktree_cleanup`、`cleanup_scans_configured_project_when_state_has_no_entry`、`cleanup_scans_task_context_project_when_workspace_path_changed`、`cleanup_rejects_dependency_junction_paths_outside_the_host`、`cleanup_missing_host_uses_persisted_task_route_after_project_configuration_changes` 通过。
- `workflow_tool.py verify --repo . --work-item task-package-profiles`：AC-007 至 AC-010、AC-012 至 AC-015 通过；AC-001 至 AC-006、AC-011 为人工未执行。

## 人工后续

详见 `manual-test.md`。至少需要在用户指定 UE 项目上验证真实 Shipping UAT、固定包目录、MCP 禁用、外部 Junction/用户文件保护和中断恢复。
