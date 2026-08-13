---
schema_version: 1
protocol: 1.3.0
artifact: debug
artifact_id: ar_01KZWF4R0J0DTCF96PEBAPAE49
work_item_id: wi_01KZWF13NWAB3WM1WDMBX9VHKC
created_at: 2026-08-13T02:27:41Z
producer: aes-debug
result: fixed
supersedes: null
dependencies:
  work_item_contract_digest: sha256:6c5d4463a14ee7512dc818706ee9cd137f547bdffdb416b1aa0e5e20bf814d29
  artifacts: []
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

# heavy 构建档位误报引擎头警告

## 复现

1. 读取 `BuildProfile::Heavy.flags()`。
2. 断言参数包含 `-ForceHeaderGeneration`、`-Rebuild`、`-DisableUnity` 和 `-NoSharedPCH`。
3. 断言参数不包含 `-WarningsAsErrors`。
4. 修改前运行测试，断言在 `src/build_profile.rs:121` 失败，因为 heavy 参数仍包含 `-WarningsAsErrors`。

## 假设与排除

| 假设 | 证据 | 结论 |
| --- | --- | --- |
| MSVC 版本导致正常构建环境失效 | 用户提供的打包和 Installed Build 已通过，失败只出现在 heavy 组合 | 排除为直接原因 |
| heavy 只检查插件自身警告 | `-NoSharedPCH` 展开更多引擎头，`-WarningsAsErrors` 对整个目标生效 | 排除 |
| medium 也应删除警告升级 | medium 不关闭共享 PCH，现有定位就是 PR 前的警告检查 | 排除 |
| heavy 的参数责任边界过宽 | 代码同时加入 `-NoSharedPCH` 与 `-WarningsAsErrors`，文档还承诺接近 Installed Build | 确认 |

## 根因

heavy 把两种不同目标合在一个参数集里：关闭共享 PCH 用来暴露头文件依赖问题，全局警告升级用来约束编译目标的警告。两者叠加后，插件构建直接展开 UE 5.5 引擎头，并把 MSVC 14.51 报出的引擎弃用警告升级为错误。现有文档把这个组合描述为“接近 Install Build”，扩大了插件维护者的验收责任。

## 修复

- 从 heavy 删除 `-WarningsAsErrors`，保留完整重建、强制 UHT、关闭 Unity 和关闭共享 PCH。
- medium 保留 `-WarningsAsErrors`。
- 文档明确 heavy 的检查目标和与 Installed Build 的差异。

## 回归测试

- `heavy_rebuilds_without_promoting_engine_header_warnings` 固定 heavy 的四个核心参数，并禁止 `-WarningsAsErrors`。
- `medium_keeps_plugin_warning_enforcement_without_heavy_rebuild_flags` 固定 medium 的警告升级能力，并禁止 heavy 专属参数渗入。
