---
schema_version: 1
protocol: 1.3.0
artifact: validation
artifact_id: ar_01KZWFC0N4CJXXGGTPZM2QHNH8
work_item_id: wi_01KZWF13NWAB3WM1WDMBX9VHKC
created_at: 2026-08-13T02:31:39Z
producer: aes-validate
outcome: passed
executed_at: 2026-08-13T02:29:00Z
environment: Windows PowerShell, Rust workspace at F:\AiProject\UnrealDevFlow
acceptance:
  - acceptance_id: AC-001
    outcome: passed
    method: cargo test build_profile::tests -- --nocapture
    evidence: 2 profile boundary tests passed; heavy test confirms the four required flags and rejects -WarningsAsErrors
  - acceptance_id: AC-002
    outcome: passed
    method: cargo test build_profile::tests -- --nocapture
    evidence: src/build_profile.rs medium test confirms -WarningsAsErrors remains and 4 heavy-only flags are absent
  - acceptance_id: AC-003
    outcome: passed
    method: rg -n "WarningsAsErrors|Installed Build|heavy" AGENTS.md docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md
    evidence: docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md line 68 renames heavy, omits -WarningsAsErrors from its 5-flag list, and states it is not equivalent to Installed Build
  - acceptance_id: AC-004
    outcome: passed
    method: cargo fmt --check; cargo test; cargo clippy --workspace --all-targets --all-features --locked -- -D warnings; git diff --check
    evidence: format exited 0; 92 Rust tests passed; Clippy exited 0 with warnings denied; diff check exited 0
supersedes: null
dependencies:
  work_item_contract_digest: sha256:6c5d4463a14ee7512dc818706ee9cd137f547bdffdb416b1aa0e5e20bf814d29
  artifacts:
    - artifact_id: ar_01KZWF4R0J0DTCF96PEBAPAE49
      digest: sha256:710e87d67745e208c9120fb1f21ff965b0b382378072b56a6a2af72f645a4a1f
      locator: debug.md
    - artifact_id: ar_01KZWFAQZ1MEYD3C4DYT3KQW4M
      digest: sha256:7ec006ef2279febcf41476b728e482147f84e0eb45447ff0e05434cf7626d001
      locator: reviews/code-review.md
  subject:
    kind: change_set
    digest: sha256:bc020be704b5c1e1afdc011a6766685e93ab0923dca6705ab9b7c656f64caa7e
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: b70d1a7363e4d62cb4ef45de6dc3ac71d3869682
    revision: 70c6f330827914920cdfedd2fb7f5b38de4b9ec9
    tree: a76aec6579c4ad9d128003f5efc7e33c9a08e4b8
    content_digest: sha256:bc020be704b5c1e1afdc011a6766685e93ab0923dca6705ab9b7c656f64caa7e
    branch_or_pr: fix/heavy-profile-warning-boundary
    workflow_excluded: true
---

# 验收

四条验收条件全部通过。参数级回归测试证明 heavy 和 medium 的责任边界，完整 Rust 测试与 Clippy 证明改动没有破坏现有命令和策略代码。

真实 UE 5.5 heavy 构建未纳入本次自动验收。该构建耗时长且依赖外部插件任务；本次改动直接删除造成误报的全局参数，后续实际任务可补充运行证据。
