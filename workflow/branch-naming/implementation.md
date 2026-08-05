---
schema_version: 1
protocol: 1.3.0
artifact: implementation
artifact_id: ar_01KZ90AH4A1X5QQZ331VK18945
work_item_id: wi_01KZ8TFVAGQNAZH74W8JDZG6GA
created_at: 2026-08-04T11:20:00Z
producer: aes-execute
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:33710c8da277ec096e0ee4c641b5343bb94e79e99040433659321b2812de8ed9
  artifacts:
    - artifact_id: ar_01KZ8ZQQ0VJZNQ7N2JQT05ZWVW
      digest: sha256:e1ea30b1b219c8ca1ce675bd27bee451db4a7cf8850e6c11fbb8ab422b0a71f2
      locator: plan.md
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

## 做完了什么

默认分支名从 `task/<workspace>/<task-id>` 改成 `<type>/<task-id>`，`task create` 加了
`--type`。分支身份搬进 git 的分支台账，名字可以随便起。

两个提交，基线 `395a262`，末版 `b83b1e3`。

| 步骤 | 改了什么 |
| --- | --- |
| S1 | 新增 `src/git/branch_ledger.rs`：`record` / `task_of` / `lookup` |
| S2 | `src/cli.rs` 加 `ChangeType` 词表和 `--type` |
| S3 | `create.rs` 默认名改规则、写台账；`main.rs` 传参 |
| S4 | `cleanup.rs::expected_branch_names` 先查台账再拼名字 |
| S5 | `merge.rs::branch_matches` 加第四条台账判定 |
| S6 | 端到端五组 |
| S7 | 五份文档 |
| S8 | 评审两条阻断的修复 |

## 评审打回了什么

**孤儿清理认不出裸 task-id。** `cleanup.rs` 把用户输入的 `task_ref` 原样拿去查台账，可台账
存的是完整的 `<workspace>/<task-id>`。敲 `udf task cleanup bare` 时两边对不上，台账空手而归，
名字反推对自定义分支名又无能为力——分支留在仓库里，命令却报「cleaned up successfully」。
`expected_branch_names` 早就为裸引用遍历了所有 workspace，台账这条腿漏了同样的事。补一个
`ledger_task_refs` 做同样的补全。顺带把「一个分支都没找到」跟「清理成功」分开说，空匹配
报成功正是这个问题从「失败」变成「静默」的原因。

**merge 的台账判定放开了 workspace 限定。** 用 `ends_with("/<task-id>")` 比，台账写着
`ws2/save-bug` 的分支在合并 `ws1/save-bug` 时也算数。设计里专门定过用完整引用存就是为了
防这个，存的时候照做了、比的时候又放开，等于白存。改成拿 `meta.task_uid` 比全串，老元数据
没有 `task_uid` 才退回原来的宽松判定。

两条回归测试都先做过负向验证：回退代码修复后各自 FAILED，装回修复后各自通过。

## 跑了什么，结果如何

三条门禁在末版 `b83b1e3` 通过，退出码都是 0。**85 个用例全过**（基线 71，新增 14）。

文档核对：138 条示例、18 个命令路径，参数核对失败 0 条。

端到端在临时 git 仓库里跑，没碰用户的真实插件库：

| 组 | 验什么 | 结果 |
| --- | --- | --- |
| T1 | 默认类型 | 分支 `feature/add-thing`，台账 `ws/add-thing` |
| T2 | `--type fix` | 分支 `fix/squash-bug`，台账 `ws/squash-bug` |
| T3 | 两个 workspace 同名 task-id | 生成规则相同，分支名都不含工程名 |
| T4 | 自定义名 + Host 丢失 | `release/nothing-to-do-with-it` 被 cleanup 精确找到并删掉 |
| T5 | 两种老形态分支 | `task-legacy-flat` 和 `task/ws/legacy-ns` 都照常清掉 |

## 自审发现的问题

**原来的 merge 归属判定测试是假的。** 我按计划先写 S5 的测试，写了一条「元数据说的分支跟
任务对不上就该拒绝」，结果 merge 成功了。查下去发现 `branch_matches` 的第一条是
`primary.branch == task_branch`，而测试辅助 `write_test_meta` 把同一个分支名同时写进任务级
和插件级两个字段——第一条永远成立，守卫从来没被验到过。

这不是我这次改坏的，是原来就有的测试盲区。改法是让插件级分支跟任务级真正分开：任务记
`hotfix/ledger-refuse`，插件记 `hotfix/someone-elses-work`，名字不含 task-id、不是 `task/`
形态、台账里也没有，四条判定全不成立，merge 才如期拒绝。

**`create::run` 的 `#[allow(clippy::too_many_arguments)]` 加重了。** 它本来就有一条，我加参数
时又补了一条，clippy 报 `duplicated attribute`。本地 fmt 和 build 都不管这个，只有 clippy 抓到。

## 哪里没按计划走

**计划说 S7 要「把分支名示例改成新形态」，实际是从零补写。** 五份文档里原来根本没有一处写过
分支命名规则——`grep 'task/<workspace>'` 零命中。所以这一步不是改写而是新增：`--type` 的取值
说明加进五份文档，命名规则和台账机制的正文加进 `AGENTS.md` 和给 AI 读的 `SKILL.md`。

**计划里 S2 写「非法取值自己写校验」，实际不用写。** clap 的 `ValueEnum` 在解析阶段就拒绝并
列出可选值，退出码 2。测试直接断言 stderr 里同时出现 `feature` 和 `hotfix`。

## 还剩什么风险

**台账写进用户插件仓库的 `.git/config`。** 每个任务分支一行，`git branch -D` 时 git 自己带走。
用底层命令（`git update-ref -d`）删分支会留下孤儿条目，`lookup` 已经过滤掉指向不存在分支的
条目，专门有一条测试守着。

**没在用户的真实插件仓库上跑过。** 端到端全在临时仓库里做的。真实仓库的差异是体积和已有分支
数量，不影响这次改动的逻辑，但没实测就是没实测。

**老任务的行为没有回归测试覆盖全部命令。** T5 验了 cleanup 的两种老形态，merge 那边靠原有的
15 个用例兜着，没有专门为「老形态 + 新代码」再造一组。
