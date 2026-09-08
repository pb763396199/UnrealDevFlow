---
schema_version: 1
protocol: 1.3.0
artifact: validation
artifact_id: ar_01M201Z68FQ0GDTFPCQQHBV07Z
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-08T15:36:00+08:00
producer: aes-validate
outcome: passed
executed_at: 2026-09-07T23:25:00+08:00
environment: "Windows, Python 3.14, UnrealDevFlow fix/package-cache-lifecycle @ b8bede7"
supersedes: ar_01M1ZXMH54JKCKW5Q7B4RPYGJX
acceptance:
  - acceptance_id: AC-001
    outcome: passed
    method: "用 release 二进制运行 package clean --dry-run，并按 research 记录复算 UDF 根目录、日志、backup 和最终包。"
    evidence: "最终 release inventory 已执行；manual-test.md 的 AC-001 已由用户标记通过。"
  - acceptance_id: AC-002
    outcome: passed
    method: "python -X utf8 -m unittest discover -s tests/skills/aes-workflow -k final_output_is_never_an_automatic_cleanup_target"
    evidence: "退出码 0；Ran 1 test；OK。"
  - acceptance_id: AC-003
    outcome: passed
    method: "python -X utf8 -m unittest discover -s tests/skills/aes-workflow -k clean_inventory_reports_categories_and_protects_final_outputs"
    evidence: "退出码 0；Ran 1 test；OK。"
  - acceptance_id: AC-004
    outcome: passed
    method: "python -X utf8 -m unittest discover -s tests/skills/aes-workflow -k cache_key_ignores_profile_revision_name_output_and_reason"
    evidence: "退出码 0；Ran 1 test；OK。"
  - acceptance_id: AC-005
    outcome: passed
    method: "python -X utf8 -m unittest discover -s tests/skills/aes-workflow -k global_preflight_uses_historical_cache_size_and_exposes_cap"
    evidence: "退出码 0；Ran 1 test；OK。"
  - acceptance_id: AC-006
    outcome: passed
    method: "检查 project/plugin execution JSON 的 taskRef，并确认不会生成 AES Work Item 绑定字段。"
    evidence: "项目记录可见 UDF taskRef；旧插件记录没有 taskRef，也没有 AES Work Item 字段；manual-test.md 的 AC-006 已由用户标记通过。"
  - acceptance_id: AC-007
    outcome: passed
    method: "python -X utf8 -m unittest discover -s tests/skills/aes-workflow -k scoped_stale_cleanup_requires_yes_and_updates_each_record"
    evidence: "退出码 0；Ran 1 test；OK；真实清理后 legacy 更新 7 条 execution，stale 更新 8 条 execution。"
  - acceptance_id: AC-008
    outcome: passed
    method: "按 plan 的 S8-S11 执行 release、进程检查、legacy/stale 清理和最终复核。"
    evidence: "legacy 命令删除 12 个旧目标，stale 命令删除 11 个旧目标；manual-test.md 的 AC-008 已由用户标记通过。"
  - acceptance_id: AC-009
    outcome: passed
    method: "python -X utf8 -m unittest discover -s tests/skills/aes-workflow -k successful_plugin_delivery_removes_private_stage"
    evidence: "退出码 0；Ran 1 test；OK。成功路径删除私有 stage 并从 cleanupTargets 移除。"
  - acceptance_id: AC-010
    outcome: passed
    method: "python -X utf8 -m unittest discover -s tests/skills/aes-workflow -k plugin_stage_uses_read_only_junctions_and_private_generated_dirs"
    evidence: "退出码 0；Ran 1 test；OK。只读输入目录是 Junction，生成目录和 Binaries 是私有目录。"
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts:
    - artifact_id: ar_01M201Z5S9Z68CST1SDEHDPFGR
      digest: sha256:08151e31596b3c06d9790e555f0b3870dfc43aefb88f9523e40cd5809c10775e
      locator: implementation.md
    - artifact_id: ar_01M201Z63CWYPKJ2DDRX7HAD0H
      digest: sha256:5f70b4d8b6b3b90d07e1756a67449cb34ae5842d48519d878a31babe8cb04c61
      locator: reviews/code-review.md
    - artifact_id: ar_01M201Z5Y9RNEH8DWME72PJD4S
      digest: sha256:2eda9b8f6bf687bc5cf76a315e6e6d68e4b37c155fc6da555764b72497052899
      locator: manual-test.md
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

# 验收结论

十条验收均通过。用户已在 manual-test.md 标记 AC-001、AC-006、AC-008；自动检查重新执行了 AC-002、AC-003、AC-004、AC-005、AC-007、AC-009、AC-010。全量 Rust 测试本轮通过：96 单测和全部 14 组集成测试通过。

本次新增的两条验收证明：插件包成功交付后会删除私有 stage；Source、Content 等大目录不会复制，生成目录和 Binaries 保持私有。失败 stage 的真实强制清理已在三条 execution 上完成，stage 总占用降为 0 GiB，三条记录均写为 `cleaned`。
