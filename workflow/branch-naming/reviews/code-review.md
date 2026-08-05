---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01KZ90RQWTN4CZY3Q76MQ2T7GK
work_item_id: wi_01KZ8TFVAGQNAZH74W8JDZG6GA
created_at: 2026-08-04T11:35:00Z
producer: aes-review
verdict: approved
review_type: code
supersedes: ar_01KZ90EF43KZJH32ZRXHJX0GWV
dependencies:
  work_item_contract_digest: sha256:33710c8da277ec096e0ee4c641b5343bb94e79e99040433659321b2812de8ed9
  artifacts:
    - artifact_id: ar_01KZ90AH4A1X5QQZ331VK18945
      digest: sha256:0eef773cc5a3f35a2ec098d3ad5744bddd24da6f030e1520d382dee93c0adf83
      locator: ../implementation.md
  subject:
    kind: change_set
    digest: sha256:c1c33cc92a83b89fd2a710045a8fc89b914c838547a7a493e020b3c21120fd49
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 395a2625650bfe9bf9a84483b1be31f6d7363830
    revision: b83b1e3aff1ed4392771d65172816decfb4dd089
    tree: 853bc40544d290bbfcb2816e6e0d5a6124f2364d
    content_digest: sha256:c1c33cc92a83b89fd2a710045a8fc89b914c838547a7a493e020b3c21120fd49
    branch_or_pr: feature/branch-naming
    workflow_excluded: true
---

自己评审自己。这份改动是我写的，下面的问题是我按验收标准倒推行为、再回头读代码找出来的。

## 审查范围

基线 `395a262`，末版 `b83b1e3`，两个提交。文件清单全部落在
`src/`、`tests/` 和五份文档里，没有范围外的改动。85 个用例在末版全过。

第一版结论是 `changes_requested`，两条阻断在 `b83b1e3` 修完并逐条复核过，本版改为
`approved`。下面保留原问题描述，各自附上复核结果。

## 原来必须修的问题（已复核修复）

### 1. 孤儿清理认不出裸 task-id，分支留在仓库里而命令报成功

`src/commands/cleanup.rs:158` 把用户输入的 `task_ref` 原样交给台账查询：

```rust
for branch in git::branch_ledger::lookup(&repo_path, task_ref).unwrap_or_default() {
```

台账里存的是完整引用 `<workspace>/<task-id>`（`create.rs:99` 写死了这个格式）。用户敲
`udf task cleanup bare` 时 `task_ref` 就是 `"bare"`，跟 `"ws/bare"` 不相等，台账这条腿直接
空手而归。名字反推那条腿拼出 `task-bare` 和 `task/ws/bare`，自定义分支名两个都不是，也找不到。

裸引用是支持的输入形态——`host::parse_task_ref` 返回 `Option<workspace>`，
`expected_branch_names:43-47` 专门为它遍历了所有 workspace。台账这条腿没做同样的事。

实测（临时仓库，末版二进制）：

```
建好: release/bare-custom
台账: ws/bare
$ udf task cleanup bare --force
  ✓ Task 'bare' cleaned up successfully!
清理后: release/bare-custom      <-- 还在
```

后果具体：AC-005 要求「Host 丢失时孤儿分支清理仍然有效」。用完整引用时有效，用裸引用时
无效，而且**命令报的是成功**——`cleanup.rs:177-178` 在 `matches` 为空时发的是
`untouched(task_ref, true)`，`complete: true` 渲染成成功文案。用户看到勾，仓库里留着分支。

这条正好打在这次改动的主卖点上：分支名自由的前提是台账能兜住，兜不住就退回名字反推，
而自定义名字本来就是名字反推救不了的那一类。

**最小改法**：台账查询跟着 `expected_branch_names` 的做法走。`parse_task_ref` 拿不到
workspace 时，对 `config.workspace_names()` 里每个 workspace 各查一次
`<workspace>/<task-id>`；拿得到就只查那一个。

**复核（`b83b1e3`）：已修。** 新增 `ledger_task_refs`，跟 `expected_branch_names` 用同一套
补全逻辑。回归测试
`cleanup_reaches_the_ledger_even_when_the_task_is_named_without_its_workspace` 做过负向
验证：回退代码修复后 FAILED，装回修复后通过。顺带把空匹配的文案跟成功分开（问题 3 一并解掉）。

### 2. merge 的台账判定丢掉了 workspace 限定

`src/commands/merge.rs:239-241`：

```rust
let ledger_says_ours = git::branch_ledger::task_of(&primary.source_repo, &primary.branch)
    .unwrap_or_default()
    .is_some_and(|owner| owner == task_id || owner.ends_with(&namespaced_suffix));
```

`namespaced_suffix` 是 `/<task-id>`。所以台账里写着 `ws2/save-bug` 的分支，在合并
`ws1/save-bug` 时也会被判为「属于本任务」。

设计里专门为这件事定过：「值用完整任务引用而不是裸 task-id，这样同名 task-id 落在不同
workspace 时不会互相认领」。存的时候照做了，比的时候用 `ends_with` 又把限定放开了，
等于白存。

触发条件：两个 workspace 共用同一个插件仓库（`workspace add` 允许，端到端 T3 就是这么建的），
两边有同名 task-id，且元数据里插件级分支跟任务级分支对不上——这最后一条正是这个守卫存在的
理由。范围窄，但正是设计点名要防的那一种。

**最小改法**：拿规范化的任务引用去比，别用后缀。`run_inner` 手上有 `meta`，
`meta.task_uid`（形如 `ws/save-bug`）就是要的东西，传给 `merge_single_plugin` 参与比对；
`task_uid` 缺失的老元数据再退回现在的宽松判定。

**复核（`b83b1e3`）：已修，就是按这个改法。** `task_uid` 存在时比全串，缺失时才退回宽松判定，
老元数据不受影响。回归测试 `merge_refuses_a_branch_the_ledger_assigns_to_another_workspace`
同样做过负向验证。

## 建议修但不阻断

### 3. 空匹配报成功，掩盖了问题 1 的现象（已修）

`cleanup.rs:177-178` 在一个分支都没找到时发 `complete: true`。这不是这次引入的（上一个任务
加 `untouched` 时就是这样），但它把问题 1 从「清理失败」变成了「静默不清理」。

建议：找不到任何分支时用一句区分得开的话，比如「没有找到属于这个任务的残留分支」，
而不是「cleaned up successfully」。

**复核（`b83b1e3`）：已修**，采用了建议的文案。

### 4. 台账读失败被无声吞掉

`cleanup.rs:158` 的 `unwrap_or_default()` 和 `merge.rs:239` 的 `unwrap_or_default()` 会把
真实的 git 错误当成「查不到」。目录不是 git 仓库时这是对的，但仓库损坏、配置文件读不了这类
情况也一样静默。

建议：至少在 `--verbose` 下把错误打出来。不阻断，因为退化行为是安全的一侧。

## 我确认过没有问题的地方

- **台账模块本身写得对。** `lookup` 用 `strip_prefix` / `strip_suffix` 而不是按点切分，
  所以 `release/v1.2` 这种带点的分支名解析得回来，有专门的测试。返回前用
  `find_branch` 过滤掉指向不存在分支的孤儿条目，也有测试。这两条都是容易漏的地方。
- **写台账失败不回滚整个任务是对的。** `create.rs:297-304` 只警告。分支和 worktree 已经
  建好了，为一个索引回滚主产物不成比例，警告文案也说清了后果。
- **老任务的路径一条没动。** `merge.rs` 原有三条判定原样保留，`cleanup.rs` 的
  `expected_branch_names` 也原样保留，新逻辑都是追加。端到端 T5 验了两种老形态分支仍被清掉。
- **`--type` 的校验交给 clap 是对的，不是偷懒。** `ValueEnum` 在解析阶段就拒绝并列出可选值，
  比自己写校验更早、更准，测试断言 stderr 里同时出现 `feature` 和 `hotfix`。
- **实现记录如实写了自审发现的两件事**——原来的 merge 守卫测试是假的、`#[allow]` 加重了，
  没有粉饰。

## 没有覆盖的范围

- **没在用户的真实插件仓库上跑过**，端到端全在临时仓库里。
- **多主插件没单独验**。台账是按 `plan.source_repo` 逐个写的，代码上没问题，但没有一组
  多主插件的端到端。
- **端到端在末版复跑过**，五组仍全过，两条新回归测试也在其中。
- **`task delete` 没看**。它有自己的一套 `expected_branches`（`delete.rs:159-168`），这次
  没动它，也没验它对自定义分支名的行为。
