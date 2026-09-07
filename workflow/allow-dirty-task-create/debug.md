---
schema_version: 1
protocol: 1.3.0
artifact: debug
artifact_id: ar_01M1XGPARKH1R28M1SVK82VK7B
work_item_id: wi_01M1XFY0XRRPCQKBVK083X2P2Q
created_at: 2026-09-07T08:46:08Z
producer: aes-debug
result: fixed
supersedes: null
dependencies:
  work_item_contract_digest: sha256:3fecfd5988882d94f409d55054f4240248d5ddd042304907e8f45abfcb674e3d
  artifacts: []
  subject:
    kind: change_set
    digest: sha256:5d8258a52c4a43d50dedff5b0d4cd574490b9402760e30df26cd3b3f1c7736d3
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 284525b422739bd6b2689bf1e7aca9c08fc1835a
    revision: 8439d1ba60b59fa2a5e42374967bb600ac46a1c5
    tree: 679323326f538cbe96706825a1ee0af4c002458a
    content_digest: sha256:5d8258a52c4a43d50dedff5b0d4cd574490b9402760e30df26cd3b3f1c7736d3
    branch_or_pr: fix/allow-dirty-task-create
    workflow_excluded: true
---
# 主检出有普通修改时无法创建任务

## 复现

1. 在临时主插件仓库的 `dev` 分支修改一个已跟踪文件，不提交。
2. 分别保留未暂存状态和执行 `git add` 后的已暂存状态。
3. 执行 `udf task create`。
4. 修复前两种输入都退出 1，错误写着主仓工作区不干净；同一仓库只有未跟踪文件时可以创建。
5. 制造内容冲突后再次执行，修复前只给出同一条笼统错误。

## 假设与排除

| 假设 | 证据 | 结论 |
| --- | --- | --- |
| `git worktree add` 会复制主检出的普通修改 | 既有未跟踪用例与新增已跟踪用例都能检查任务工作区内容；worktree 由提交号创建 | 排除 |
| 创建前的状态检查把普通修改当成危险状态 | `prepare_primary_plans` 调用 `status_porcelain_tracked`，该命令同时返回未暂存、已暂存和冲突条目 | 确认 |
| 删除状态检查就能满足要求 | 未合并冲突也会出现在原检查结果里；直接删除会让冲突仓库继续创建 | 排除 |
| worktree 从提交创建就足以隔离主检出修改 | 首次修复后，依赖规划仍从主检出读取 `.uplugin`；加入一个只存在于未提交描述符里的依赖会错误阻止创建 | 排除 |
| 用户说明已经一致 | README 写所有未提交修改都允许，v0.5.0 发布说明和 AI skill 仍写已跟踪修改会阻止创建 | 确认 |

## 根因

任务创建有两个读取来源没有分开。状态门禁把普通修改和未合并冲突放进同一个判断，导致安全的未暂存和已暂存修改也无法通过。首次收窄门禁后，Host 依赖规划仍从主检出的工作目录读取 `.uplugin`，所以未提交描述符虽然不进入 worktree，仍会改变创建结果。状态判断和依赖规划都必须以各自明确的输入为准。

## 修复

- 创建前单独读取未合并路径，并读取 Git 当前操作状态。
- 未合并冲突或未完成操作继续返回错误，普通修改改为警告。
- 主插件描述符名称和依赖从 `PrimaryPlan.based_on` 对应的提交树读取，Host 与任务 worktree 使用同一基线。
- 警告明确说明任务从当前 `dev` 的 `HEAD` 创建，普通修改留在主仓，不进入 Host，也不自动提交。
- README、v0.5.0 发布说明和 AI skill 使用同一组边界说明。

## 回归测试

| 用例 | 证明内容 |
| --- | --- |
| `task_create_allows_untracked_main_checkout_changes` | 未跟踪文件保留在主仓，任务工作区不包含该文件 |
| `task_create_allows_unstaged_tracked_main_checkout_changes` | 未暂存描述符加入不存在的依赖后仍能创建，任务工作区和 Host 规划读取 `HEAD` |
| `task_create_allows_staged_tracked_main_checkout_changes` | 已暂存描述符加入不存在的依赖后仍能创建，任务工作区和 Host 规划读取 `HEAD` |
| `task_create_rejects_unmerged_conflicts` | 未合并冲突给出专门错误，且不创建 Host |
| `task_create_dirty_checkout_docs_are_consistent` | 三份用户说明都列出允许状态和阻断状态 |

## 证据

- 修复前定向执行时，未暂存和已暂存用例均退出 1；冲突用例没有得到专门错误。
- 修复后 `cargo test --test create_sources task_create_` 得到 5 passed、0 failed。
- `cargo test --test create_sources` 得到 25 passed、0 failed。
- 独立评审用未暂存描述符加入 `DirtyOnlyMissing`，首次修复仍错误退出 1；第二次修复后 staged 和 unstaged 用例均通过。
- 代码提交为 `843a44054c0c0d8544954a4d3ecd00e0aff7aab8` 与 `8439d1ba60b59fa2a5e42374967bb600ac46a1c5`。
