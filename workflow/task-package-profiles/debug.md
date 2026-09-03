---
schema_version: 1
protocol: 1.3.0
artifact: debug
artifact_id: ar_01M1JP3MJDG785GAW51M12CEV7
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-03T04:55:00Z
producer: aes-debug
result: fixed
supersedes: ar_01M1JKS2HHY0P3D1070AJXF865
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1GD7JRRHR8QXDXHPXFTJ2Q8
      digest: sha256:25c4d285e83cf0d34053512bcad767ed1b3766b4431d6eb7c794e87e1d5eee0d
      locator: plan.md
  subject:
    kind: change_set
    digest: sha256:7d730c2b95cc64238c5c3f2d58a4335a1c62b00a0cf59b31388cae6f92c8bc7e
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: d608cb80ec14295e3baa19919055bd03efec6e32
    revision: e46fc5d3041c88ecd39c9fa4b41699920b0561c8
    tree: 056fb0b6bff80dc78f4baa75c4979db403854461
    content_digest: sha256:7d730c2b95cc64238c5c3f2d58a4335a1c62b00a0cf59b31388cae6f92c8bc7e
    branch_or_pr: feature/task-package-profiles
    workflow_excluded: true
---

# 调试结论

## 2026-09-03 Junction 清理漏洞

### 复现

1. 在配置的主项目 `Project/Plugins/AesWorld` 创建一个指向任务 Host 的 Junction。
2. 模拟 `switch` 在保存 `state.json` 前中断，确保 `state.json` 没有该项目记录。
3. 先删除 Host 中的目标目录，留下主项目里的悬空 Junction。
4. 执行任务清理。

修复前回归测试得到 `removed = 0`，主项目 Junction 仍存在。修复后得到 `removed = 1`，
Junction 目录项已删除，目标目录之外的文件未受影响。

### 假设与排除

| 假设 | 证据 | 结论 |
| --- | --- | --- |
| Windows 悬空 Junction 无法被删除 | 既有 `junction` 单元测试已证明 `symlink_metadata` 能发现，`junction::delete` 能删除 | 排除 |
| 清理顺序先删 Host 导致目标匹配失败 | 现有 cleanup/delete 已在删除 Host 前调用 Junction 清理；新回归把 Host 目标先删掉后仍复现 | 不是主要根因 |
| `state.json` 是完整项目发现来源 | 新回归清空 state 后，旧实现无法发现配置主项目上的 Junction | 已证伪 |

### 根因

`collect_project_candidates` 原来只遍历 `state.projects`。当 Junction 已经创建，但 `switch`
尚未完成 state 写入，或者 state 被清理、损坏时，清理器没有主项目路径可扫描。它随后仍能
删除 Host 和 worktree，却把主项目 Junction 留在原处，造成下一次打包或切换看到失效插件路径。

### 修复

- `cleanup_for_task` 和缺失 Host 恢复路径现在接收配置上下文。
- 清理器合并扫描 `config.default_project`、所有 workspace 的 `default_project` 和 state 中的历史项目。
- 仍只扫描每个项目 `Plugins` 的直接子项，并且只有 Junction 目标位于当前任务 Host 根目录内时才删除。
- 新增 `cleanup_scans_configured_project_when_state_has_no_entry` 回归测试，覆盖 Host 先删除、state 无项目记录的悬空 Junction 场景。

代码提交：`b0fb527fd3632d598e360fc518fa1325ee5f93f2`。

## 本轮剩余边界修复

### 复现

1. 将 metadata 的 dependency Junction 路径改为 `../../../Outside/Plugins/Victim`，执行任务清理。
2. 切换任务后删除 Host 和 `state.json`，再把 workspace 配置改指其他项目，执行缺失 Host 清理。
3. 修复前，第一种场景会删除 Host 外 Junction，第二种场景无法找到原主项目 Junction。

### 根因与修复

- 清理器原来直接把 metadata 路径拼到 Host，未拒绝绝对路径、根路径和 `..`，现在会拒绝并将其计入 `failed`。
- 缺失 Host 恢复原来只有当前配置和 state，没有持久证据；新增原子写入的 `task-routes.json`，切换前保存任务 Host 与主项目路径，清理完整成功后删除。
- 恢复路径现在合并持久化路由、任务冻结 context、当前配置和 state，并继续按 Host 目标过滤项目 Junction。

### 证据

`cleanup_rejects_dependency_junction_paths_outside_the_host`、
`cleanup_scans_task_context_project_when_workspace_path_changed` 和
`cleanup_missing_host_uses_persisted_task_route_after_project_configuration_changes` 均通过。

## 环境

- 项目：`F:\ShanghaiP4\neon\UGA\DEV\UGA.uproject`
- 引擎：`C:\Program Files\Epic Games\UE_5.5`
- AesWorld：`F:\ShanghaiP4\neon\Plugins\AesWorld`，`dev`，提交 `3413ea52d5a787e8addfe3ff36ee7ec3877ffacf`
- 输出根目录：`C:\Package`
- 配置：Shipping、Pak、名称 `UGA-Win64-Shipping-20260901`

## 复现与排除

| 假设 | 证据 | 结论 |
| --- | --- | --- |
| UAT 缺少项目目标或引擎参数，导致 Cook 失败 | `package plan project` 已生成 `-target=UGA`、`-unrealexe=...UnrealEditor-Cmd.exe`、`-installed`、`-skipbuildeditor`、`-nocompile*`；这些参数与此前成功 UAT 日志一致 | 当前证据不支持 |
| DEV 输入本身阻止打包 | `DEV\Content\Effects -> G:\ShanghaiP4\Mars\COS\COS4\Content\Effects` 的目标不存在；`package check project` 明确报告断链 | 已确认 |
| 共享 UBT 锁阻止打包 | 外部任务持续运行多个 UnrealBuildTool 进程，预检报告 `ubt_mutex_busy` | 已确认 |
| 最新 AesWorld dev 源码不能编译 | `package plugin AesWorld` 在 1721/1727 个 action 后退出码 6，错误位于 `AesStreamingIoDispatchTests.cpp:392`，类型 `IAesStreamingHttpResponse` 缺少定义和 `GetContent` | 已确认 |
| 安装版引擎可以生成 Installed Build | `package engine` 退出码 1，缺少 `Engine\Build\InstalledEngineBuild.xml` | 已确认不支持该输入 |

为单独验证 UAT 参数，测试曾临时移除断链 Junction；工具随后开始复制 DEV 临时副本。复制阶段包含大量项目文件，外层测试会话先结束，未产生完整 UAT/Cook 执行记录。原 Junction 已用原目标精确恢复。

## 根因

本轮没有复现“进入 Cook 后因 UAT 参数失败”。项目命令的当前阻塞来自项目输入断链和共享 UBT 锁。工具另有两个已修复问题：项目 staging 会复制不参与 Cook 的 IDE/Agent 元数据，Engine 失败清理没有登记 UAT 创建的输出根目录。

## 修复

- `src/ue_commands.rs` 补齐成功 UAT 日志使用的项目目标、引擎可执行文件、Installed、跳过编辑器构建和禁止 UAT 编译参数。
- `src/commands/package.rs` 在 Engine 失败清理清单中加入输出目录，并跳过 IDE/Agent 元数据目录。
- `tests/package_commands.rs` 和 package 内部测试锁定上述行为。

## 未完成

待输入 Junction 恢复、外部 UBT 进程全部结束后，重新执行 `udf package project --workspace neon-dev`，再判断 Shipping Cook 本身。当前不能将 `C:\Package\Windows` 视为工具产物。
