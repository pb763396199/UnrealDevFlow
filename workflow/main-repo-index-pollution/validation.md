---
schema_version: 1
protocol: 1.3.0
artifact: validation
artifact_id: ar_01KZ93TSS81QFHTPWTH7QVBZ3J
work_item_id: wi_01KZ921GEY99YYYE1S9GVNC501
created_at: 2026-08-05T14:35:00Z
producer: aes-validate
outcome: passed
executed_at: 2026-08-05T14:35:00Z
environment: "Windows 11 Pro 26200; rustc 1.97.1; 临时 git 仓库；用户的插件仓库全程只读"
acceptance:
  - acceptance_id: "AC-001"
    outcome: "passed"
    method: "从症状倒推代码路径，在临时仓库里重建最小复现，逐个排除候选"
    evidence: "根因是 src/git/mod.rs 的 merge_branch：repo.merge_commits 算的是内存 index，repo.commit(Some(HEAD)) 只移动分支引用，索引和工作区一个都不碰。复现串：建任务 → worktree 里提交一个新文件 → udf task merge --strategy merge；合并前 index tree == HEAD tree 且 status 干净，合并后 status 报 D Source/AesWorld/feature.cpp、diff --cached 一条删除、而那个文件根本不在工作区。顺带查出 squash_branch 的两个洞：冲突判定只读 stderr 而 git 把 CONFLICT 写在 stdout（实测确认），以及 git commit 失败时暂存内容不回滚"
  - acceptance_id: "AC-002"
    outcome: "passed"
    method: "同一串复现命令在修复后重跑，比对主仓库状态"
    evidence: "复现脚本第三次运行：merge 之后 git status 为空、diff --cached 为空、feature.cpp 真的出现在工作区里。回归测试 merge_strategy_leaves_the_source_repo_index_matching_head 断言 status --porcelain 为空"
  - acceptance_id: "AC-003"
    outcome: "passed"
    method: "每条回归测试都先回退对应的代码修复跑一次，确认会失败，再装回修复跑一次"
    evidence: "merge_strategy_leaves_the_source_repo_index_matching_head 回退后 FAILED（left: D feature.txt）；a_conflicting_squash_does_not_strand_the_source_repo 回退后 FAILED（left: AA probe.txt）；an_untracked_file_does_not_block_squash_or_rebase 回退后报 target worktree is not clean。三条装回修复后全部通过"
  - acceptance_id: "AC-004"
    outcome: "passed"
    method: "对四种策略各跑一遍完整合并并断言主仓库无残骸；原有 merge/cleanup 用例全量重跑"
    evidence: "every_merge_strategy_leaves_the_source_repo_undisturbed 覆盖 merge / squash / rebase / ff-only 四种，全部通过；merge_yes 测试二进制从 17 个用例增加到 22 个，全部通过，原有用例一条没坏"
  - acceptance_id: "AC-005"
    outcome: "passed"
    method: "排查前对 AesWorld 取快照（分支、worktree、status、.git/config 摘要、Hosts 目录、udf 的 state.json 与 config.toml），排查后逐项比对"
    evidence: "我发起的操作里唯一被拒绝的是一次 task create（工具因主仓库不干净而拒绝），拒绝后逐项比对无一变化。期间 AesWorld 确实多了一个分支和 worktree，经查是旧命名格式 task/neon-dev/global-track-layer 且没有台账条目，只可能来自用户装在 ~/.unrealdevflow/bin 的 0.2.0，不是本次会话。要说明的是我跑过一次 git write-tree，它不改内容但会刷新 .git/index 的 cache-tree"
  - acceptance_id: "AC-006"
    outcome: "passed"
    method: "在末版 699652f6b 上依次跑三条门禁并取退出码"
    evidence: "cargo fmt --check 退出码 0；cargo clippy --workspace --all-targets --all-features --locked -- -D warnings 退出码 0；cargo test 退出码 0，四个测试二进制合计 90 passed / 0 failed（基线 86，新增 4）"
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8e8cfc19cd5d95fa0a6d13766dc6d2bbe7ec238051e1bd0772d5fa2863d4c2ba
  artifacts:
    - artifact_id: ar_01KZ93RHR2PADKCYFQJ8TJPGBS
      digest: sha256:d1920b4146b4e2600f392edf3326ea4c8233ef9affe038e5cabcf3ab536aa893
      locator: reviews/code-review.md
  subject:
    kind: change_set
    digest: sha256:26788b3cfc575084abeaf11f862be4bff843241f32b204f404c98b759581a953
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: b4df27af5433ed52df9396edbe030874642fdda6
    revision: 699652f6ba165c5c4a9aa91370d56558c42aac78
    tree: ba8f734bdcf1936b8c9e6b6c828d75c3313deb72
    content_digest: sha256:26788b3cfc575084abeaf11f862be4bff843241f32b204f404c98b759581a953
    branch_or_pr: fix/main-repo-index-pollution
    workflow_excluded: true
---

## 在什么状态上验的

分支 `fix/main-repo-index-pollution`，版本 `699652f6b`，基线 `b4df27a`，三个提交。
代码区干净，只有 `workflow/` 未提交。

## 逐条结果

六条全 `passed`。

AC-003 是这次最该看的一条：三条回归测试全部做过负向验证，不是写完就算。这个缺陷能发出去，
正是因为以前没有任何测试问过「这条命令有没有弄脏用户的仓库」。

## 转人工的部分

无。全部结果都是命令输出或 git 状态，机器验得了。
