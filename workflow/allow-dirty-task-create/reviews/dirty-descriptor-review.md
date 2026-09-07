---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01M1XH1WVCE7SXA9ZM1HABFVBT
work_item_id: wi_01M1XFY0XRRPCQKBVK083X2P2Q
created_at: 2026-09-07T08:51:43Z
producer: aes-review
verdict: changes_requested
blocking_findings:
  - 脏主检出的 .uplugin 仍会影响 Host 依赖规划
review_type: code
reviewers:
  - code-reviewer
supersedes: null
dependencies:
  work_item_contract_digest: sha256:3fecfd5988882d94f409d55054f4240248d5ddd042304907e8f45abfcb674e3d
  artifacts: []
  subject:
    kind: change_set
    digest: sha256:e3207ded934b32181a747928f44b9095a8aa89cbd186c435c9a58a11781ed10f
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 284525b422739bd6b2689bf1e7aca9c08fc1835a
    revision: 843a44054c0c0d8544954a4d3ecd00e0aff7aab8
    tree: e9c2799ee6da856a693f2c67947ed94715bbc367
    content_digest: sha256:e3207ded934b32181a747928f44b9095a8aa89cbd186c435c9a58a11781ed10f
    branch_or_pr: fix/allow-dirty-task-create
    workflow_excluded: true
---
# 脏描述符仍会影响 Host 规划

## 结论

提交 `843a440` 需要修改。仓库状态门禁已经允许普通修改，但依赖规划仍从主检出的工作目录读取 `.uplugin`，任务 Host 可能使用不属于 `HEAD` 的依赖信息。

## 必须修的问题

| 严重程度 | 位置 | 后果 | 证据 | 最小改法 |
| --- | --- | --- | --- | --- |
| 高 | `src/commands/create.rs:132-140` | 未提交的描述符可以误加依赖、漏掉依赖或错误阻止任务创建，Host 与任务 worktree 的提交内容不一致 | 临时仓库的 `HEAD` 没有依赖，未暂存描述符加入 `DirtyOnlyMissing` 后，命令因找不到该依赖而退出 1 | 从每个 `PrimaryPlan.based_on` 读取描述符名称和依赖，不能读取可变工作目录 |

## 已核实的事实

- 主插件 worktree 由 `PrimaryPlan.based_on` 创建，文件内容不包含普通未提交修改。
- Host 创建发生在主插件 worktree 创建之前。
- 依赖扫描读取 `project_plugins` 中的主检出路径。
- 未合并冲突与进行中的 Git 操作已有单独门禁。

## 非阻断问题

没有。

## 没覆盖的范围

本轮没有审查任务创建之外的 merge、delete、switch 和 build 行为。
