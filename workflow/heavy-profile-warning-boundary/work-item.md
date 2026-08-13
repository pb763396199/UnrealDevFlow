---
schema_version: 1
protocol: 1.3.0
id: wi_01KZWF13NWAB3WM1WDMBX9VHKC
short_id: rkj05b7e
title: 收窄 heavy 构建档位的警告责任边界
status: done
kind: fix
created_at: 2026-08-13T02:25:48Z
created_by: codex
home_repository: https://github.com/pb763396199/UnrealDevFlow.git
branch_or_pr: fix/heavy-profile-warning-boundary
base_revision: b70d1a7363e4d62cb4ef45de6dc3ac71d3869682
---

# 收窄 heavy 构建档位的警告责任边界

## 目标

让 heavy 档位继续执行强制头文件生成、完整重建、关闭 Unity 和关闭共享 PCH，同时不把 UE 5.5 引擎头在当前 MSVC 下的弃用警告升级为插件交付错误。

## 范围

- 调整 heavy 档位的 UBT 参数。
- 补充参数级回归测试。
- 修正文档中 heavy 与 Installed Build 接近程度的表述。

## 强约束

- medium 继续启用 `-WarningsAsErrors`。
- 不新增构建档位，除非现有代码证据表明参数模型无法表达所需边界。
- 不改变 light 和 medium 的其他参数。

## 验收条件

- AC-001：heavy 生成的 UBT 参数包含 `-ForceHeaderGeneration`、`-Rebuild`、`-DisableUnity` 和 `-NoSharedPCH`，不包含 `-WarningsAsErrors`。
- AC-002：medium 生成的 UBT 参数仍包含 `-WarningsAsErrors`，且不包含 heavy 专属参数。
- AC-003：用户文档明确说明 heavy 用于暴露非 Unity 和非共享 PCH 问题，不承诺等价于 Installed Build。
- AC-004：受影响的 Rust 测试、格式检查和静态检查通过。

## 用户原话

其他 agent 反馈 heavy 同时使用 `-NoSharedPCH` 和 `-WarningsAsErrors`，会把 UE 5.5 引擎头在 MSVC 14.51 下的 C4996 升级为错误。建议 medium 保留 `-WarningsAsErrors`，heavy 删除全局 `-WarningsAsErrors`，必要时未来另设 paranoid。
