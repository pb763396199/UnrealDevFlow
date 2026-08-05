---
schema_version: 1
protocol: 1.3.0
artifact: delivery
artifact_id: ar_01KZ90YH5VXNDRVXQYQBC7ZB05
work_item_id: wi_01KZ8TFVAGQNAZH74W8JDZG6GA
created_at: 2026-08-04T12:00:00Z
producer: aes-finish
outcome: delivered
landing_branch: "dev"
landing_revision: b83b1e3aff1ed4392771d65172816decfb4dd089
supersedes: null
dependencies:
  work_item_contract_digest: sha256:33710c8da277ec096e0ee4c641b5343bb94e79e99040433659321b2812de8ed9
  artifacts:
    - artifact_id: ar_01KZ90SP8SX7A064YVG6VEGZZF
      digest: sha256:7b4e3ec63a76695d17a56259790fb6d66bdf7851046204590c56ff6175a5348b
      locator: validation.md
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

## 交了什么

`feature/branch-naming` 上的两个提交，基线 `395a262`，末版 `b83b1e3`，
已经用 `--ff-only` 快进到本地 `dev`。

默认分支名从 `task/<workspace>/<task-id>` 变成 `<type>/<task-id>`，`task create` 加了
`--type`。分支身份搬进 git 的分支台账，名字可以随便起。

## 落地核对

`dev` 的树和 `b83b1e3` 的树都是 `853bc40`，同一份。落地之后在 `dev` 上重跑三条门禁，
退出码都是 0，85 个用例全过。

没有推到 `origin`，也没有合进 `master`。

## 用户会看到什么变化

| | 之前 | 现在 |
| --- | --- | --- |
| 默认分支名 | `task/neon-dev1/prefab-save-bug` | `feature/prefab-save-bug` |
| 修缺陷的任务 | 一样是 `task/...` | `fix/prefab-save-bug` |
| 自定义分支名 | 可以，但 Host 丢了就找不回来 | 可以，随便起，照样找得回来 |

不需要迁移。老任务的分支名带工程名、没有台账，走的还是原来那条名字反推的路。

## 怎么回滚

`dev` 还没推 `origin`，`git reset --hard 395a262` 就能完全回到落地前。已经推出去的话
`git revert` 这两个提交。没有数据迁移、没改元数据 schema、没改配置文件格式，退回去不留
半截状态。已经写进用户插件仓库的台账条目在删分支时由 git 自己带走，回滚后它们只是无人读的
配置行，不影响任何命令。

## 遗留风险

- **没在用户的真实插件仓库上跑过。** 端到端全在临时仓库里做。差异是体积和已有分支数量，
  不影响这次改动的逻辑，但没实测就是没实测。
- **多主插件没单独验。** 台账按 `plan.source_repo` 逐个写，代码上没问题，但没有一组
  多主插件的端到端。
- **`task delete` 这次没动也没验。** 它有自己的一套 `expected_branches`，对完全自定义的
  分支名会不会拒绝，没有验过。
- **台账写进用户插件仓库的 `.git/config`。** 每个任务分支一行。用底层命令删分支会留下孤儿
  条目，`lookup` 已经过滤掉指向不存在分支的条目，有测试守着。
