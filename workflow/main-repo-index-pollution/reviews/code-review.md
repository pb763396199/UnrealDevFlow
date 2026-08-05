---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01KZ93RHR2PADKCYFQJ8TJPGBS
work_item_id: wi_01KZ921GEY99YYYE1S9GVNC501
created_at: 2026-08-05T14:20:00Z
producer: aes-review
verdict: approved
review_type: code
supersedes: ar_01KZ93JHJYAX4D9FMTT5HP1MMG
dependencies:
  work_item_contract_digest: sha256:8e8cfc19cd5d95fa0a6d13766dc6d2bbe7ec238051e1bd0772d5fa2863d4c2ba
  artifacts:
    - artifact_id: ar_01KZ93EG4WSRVP1ZYPD0KEECXB
      digest: sha256:b37faafe8f37490fc55f77057076625e968452c9e6a999d9b780bb772551a6b4
      locator: ../debug.md
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

自己评审自己。下面的问题是按验收标准倒推行为、再回头读代码和实测找出来的。

## 审查范围

基线 `b4df27a`，末版 `699652f`，三个提交，三个文件全部在 `src/` 和 `tests/`，
没有范围外的改动。90 个用例在末版全过。

第一版结论是 `changes_requested`，两条阻断在 `699652f` 修完并逐条复核过，本版改为
`approved`。下面保留原问题描述，各自附上复核结果。

## 原来必须修的问题（已复核修复）

### 1. squash 新加的干净检查把未跟踪文件也算进去，会在用户的真实仓库上直接拒绝

`src/git/mod.rs` 的 `squash_branch` 开头：

```rust
let dirty = git_stdout(repo_path, &["status", "--porcelain"])?;
if !dirty.is_empty() { return Err(...); }
```

`git status --porcelain` 默认包含未跟踪文件。**用户的 `AesWorld` 现在的 status 就是一行
`?? workflow/`**，所以这一改之后 `--strategy squash` 在他们的主仓库上会被无条件拒绝。

这个严格度换不到任何安全性。实测确认两件事：

- `git merge --squash` 本身不介意未跟踪文件（只有当合并会覆盖某个未跟踪文件时 git 才拒绝，
  那是 git 自己的保护，不需要我们再挡一层）。
- 失败回滚用的 `git reset --hard HEAD` 不碰未跟踪文件，所以它们在不在场，跟「回滚是否安全」
  这个论证没有关系。

我加这个前置条件的理由是「知道开工前是干净的，之后暂存的一定是我们弄的，所以 `reset --hard`
才证明得了安全」。这个论证只需要**已跟踪文件**干净，未跟踪文件被多算进来了。

**最小改法**：`git status --porcelain --untracked-files=no`。

**复核（`699652f`）：已修**，就是这个改法。回归测试 `an_untracked_file_does_not_block_squash_or_rebase`
做过负向验证：回退修复后 squash 报「target worktree is not clean」，装回后通过。

### 2. `rebase_branch` 有同样的过严检查，用户已经被它挡过

`src/git/mod.rs:311` 是同一个写法，pre-existing，不是这次引入的：

```rust
let clean_status = git_stdout(repo_path, &["status", "--porcelain"])?;
```

记录在案的事实：上一个任务的冒烟里，`AesWorld` 因为有 `?? workflow/` 而被 rebase 拒绝，
当时判断成「工具正确拒绝」。现在看，拒绝的理由不成立——`restore_target_tip` 用的也是
`reset --hard`，同样不碰未跟踪文件。

后果具体：用户主仓库里只要有任何未跟踪文件（`workflow/`、编辑器临时文件、本地脚本），
四种合并策略里就有两种用不了，而且错误信息把未跟踪文件列成「工作区不干净」，看起来像是
他们自己的问题。

放在必须修里，是因为这次改动把受影响的策略从一种扩大到了两种。只修 squash 而留着 rebase，
等于知道有同一个毛病却只修一半。

**最小改法**：同上，两处一起加 `--untracked-files=no`。

**复核（`699652f`）：已修**，`rebase_branch` 一起改了，上面那条回归测试同时覆盖两种策略。

## 建议修但不阻断

### 3. 调用点丢了 `open_repo` 的前置校验（已修）

`src/commands/merge.rs` 原来是：

```rust
let repo = git::open_repo(&source_repo)?;
match git::merge_branch(&repo, &primary.branch) {
```

改成直接传路径之后，`source_repo` 不是 git 仓库时不再报 `NotARepo`，而是让 `git merge` 自己
失败，错误信息变成一段 git 的英文输出。功能上不影响（该失败还是失败），只是诊断信息退化。

建议：`merge_branch` 开头加一次轻量校验，或者在调用点保留 `open_repo` 只做校验。

**复核（`699652f`）：已修**，调用点保留了 `open_repo` 只作校验。

### 4. `git commit` 失败那条路没有直接测试

`squash_branch` 新加的「commit 失败就 `reset --hard HEAD`」只有代码走查。要真测得装一个会
拒绝的 `pre-commit` 钩子，或者临时清掉 `user.email`。

建议补一条：临时把 `core.hooksPath` 指向一个必然失败的钩子目录，跑 squash，断言主仓库仍然干净。

## 我确认过没有问题的地方

- **`merge_branch` 换成 `git merge --no-ff --no-edit -m <msg>` 是对的。** 实测这三个参数
  一起用能正常产生合并提交。`--no-ff` 保住了原来 git2 实现「总是产生一个双亲合并提交」的语义；
  已经合过时 git 报 `Already up to date.` 并返回 0，跟原来的 `merge_base == branch_commit`
  早返回等价。
- **冲突判定同时看 stdout 和 stderr 是必须的，且实测过。** `git merge --squash` 把
  `CONFLICT (content): ...` 写在 stdout，stderr 是空的。原来只查 stderr，所以冲突永远判不出来。
- **审计断言选的四个信号是对的。** `status --porcelain`、`ls-files --unmerged`、
  `MERGE_HEAD`、`CHERRY_PICK_HEAD` 分别覆盖「有未提交改动」「有冲突条目」「停在 MERGING」
  「停在 cherry-pick 中途」，四种残骸形态各有一个。
- **两条负向验证是真做了的，不是声称。** 回退 `merge` 修复后
  `merge_strategy_leaves_the_source_repo_index_matching_head` 报 `left: "D  feature.txt"`；
  回退 `squash` 修复后 `a_conflicting_squash_does_not_strand_the_source_repo` 报
  `left: "AA probe.txt"`。
- **`create` 被判为干净是有实测支撑的，不是读代码下的结论。**
  `creating_and_deleting_a_task_leaves_the_source_repo_undisturbed` 真跑了 `task create` 和
  `task delete` 再断言。用户点名怀疑的就是这条路。

## 没有覆盖的范围

- **没在用户的真实仓库上验过**，任务强约束要求全部在临时仓库里做。
- **审计只覆盖会动插件仓库的命令。** `switch` 动 Junction 和 `state.json`，`build` 动 Host
  目录，都不在这次审计范围里，它们是不是也有各自的「留下残骸」问题，这次没查。
- **多主插件没单独审计。** 审计测试都是单主插件，多个主仓库同时被合并时的失败中断行为没验。
