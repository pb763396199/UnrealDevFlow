---
schema_version: 1
protocol: 1.3.0
artifact: delivery
artifact_id: ar_01KZWHKREPYY8F60A901Z1MCZ1
work_item_id: wi_01KZWF13NWAB3WM1WDMBX9VHKC
created_at: 2026-08-13T03:10:50Z
producer: aes-finish
outcome: delivered
landing_revision: 70c6f330827914920cdfedd2fb7f5b38de4b9ec9
landing_branch: dev
supersedes: null
dependencies:
  work_item_contract_digest: sha256:6c5d4463a14ee7512dc818706ee9cd137f547bdffdb416b1aa0e5e20bf814d29
  artifacts:
    - artifact_id: ar_01KZWFC0N4CJXXGGTPZM2QHNH8
      digest: sha256:56d112c5f8f73d77d702ff4ff9891afa60b024f16a284d574a0d7a821be3af21
      locator: validation.md
    - artifact_id: ar_01KZWFENMXPEC9TTZ21FNDPM4W
      digest: sha256:f2598c48cd106eee1ada145f61ee95fb49cacafab5dfc5b8b8029b0b160a0c11
      locator: manual-test.md
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

# 已交付

## 要落地的改动

- heavy 删除全局 `-WarningsAsErrors`，保留 4 个重建与隔离参数。
- medium 继续启用 `-WarningsAsErrors`。
- 参数参考文档明确 heavy 不等价于 Installed Build。

## 证据

- 独立评审通过，阻断问题 0。
- 4 条验收条件全部通过。
- 92 项 Rust 测试通过，Clippy 在拒绝警告的条件下通过。
- 人工核对项为 0。
- 真实 UE 5.5 heavy 构建完成 Clean 和 UHT，并执行 547 个 action；引擎头 C4996 保持为 warning。
- 真实构建在 UnrealMCP 自身的 93 条独立头文件依赖错误处失败，证明 heavy 已越过原来的引擎警告误报。

## 回滚

1. 在任务分支回退提交 `70c6f33`。
2. 重跑 `cargo test build_profile::tests`，确认参数恢复到预期状态。

## 落地判断

代码提交 `70c6f33` 的父提交就是当前 `dev`，内容可按快进方式等价落地。用户已明确要求推到 `dev` 并安装这版工具。
