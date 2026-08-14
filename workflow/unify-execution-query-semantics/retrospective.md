---
schema_version: 1
protocol: 1.3.0
artifact: retrospective
artifact_id: ar_01KZZ5FBV9YETK7FMT2HSY296E
work_item_id: wi_01KZZ3WP75XMDZYJ25DBT1DF9R
created_at: 2026-08-14T03:36:30Z
producer: aes-retrospect
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:eaca9f43850bcae5352d32568c68ea2d6aa58a0fea1f8a1d180814ec0701177e
  artifacts:
    - artifact_id: ar_01KZZ4ZMT5FVCBS2WQCD347690
      digest: sha256:3d93feae52a3a17db17dd5fca643235a6cb9385ef551ad4b12c5ed22d6f96a5b
      locator: validation.md
---

# 复盘

## 范围

复盘从发现 Package `check/plan/status` 生命周期混用，到统一 Build 与 Package 查询语义、完成自动验收和刷新全局安装这一段。

## 经过

最初 Package 的 check 和 plan 都借用了执行记录，因此会生成执行编号并覆盖 latest；Build 的 check 与 plan 则返回完全相同的结构。实现先拆开 Package 的查询和执行，再拆 Build 输出，最后用同名验收用例和真实 DEV_1 冒烟锁定行为。

全局安装第一次核对仍显示旧 JSON。证据表明原因不是构建失败，而是当前环境没有 `pwsh`，安装子步骤实际未运行。改用 `powershell.exe` 执行同一个官方脚本后，安装器报告新提交 `7765b7beb-dirty`，全局 Build check 返回新的 domain、action、readiness 字段。

## 有用与没用的做法

| 做法 | 结果与证据 |
| --- | --- |
| 先用生命周期测试锁住 latest 不变 | `plans_do_not_replace_latest_execution` 和 `status_reads_only_real_executions` 均通过 |
| 在真实 DEV_1 上只跑 check/plan/status | 验证了真实引擎、项目和插件路径，同时没有启动耗时 UE 构建 |
| 仅凭命令链最终退出码判断安装成功 | 没用；`pwsh` 不存在是非终止错误，随后读到旧全局程序 |
| 安装后核对 JSON 字段而不只看版本号 | 有用；版本仍是 0.3.0，但新旧结构可直接区分 |

## 根子

查询和执行原先共用结果模型，是语义混乱的根子；安装误判则来自 PowerShell 非终止错误没有自动中断命令链。两者共同说明，验证必须核对目标行为本身，不能只核对“命令跑到末尾”或版本号。

## 可复用改进

- 每个执行域先固定三条生命周期不变量：check 无记录、plan 无记录、status 只读真实执行。
- 验收用例名同时提供 AES 发现入口，避免仓库测试存在但工作流找不到。
- 本机安装验证必须同时检查安装脚本输出、`Get-Command udf` 路径和一个能区分新旧版本的 JSON 字段。
