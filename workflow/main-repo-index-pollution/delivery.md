---
schema_version: 1
protocol: 1.3.0
artifact: delivery
artifact_id: ar_01KZ93WWW1VZ0S8VZ8JVCAE5JH
work_item_id: wi_01KZ921GEY99YYYE1S9GVNC501
created_at: 2026-08-05T14:45:00Z
producer: aes-finish
outcome: delivered
landing_branch: "dev"
landing_revision: 699652f6ba165c5c4a9aa91370d56558c42aac78
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8e8cfc19cd5d95fa0a6d13766dc6d2bbe7ec238051e1bd0772d5fa2863d4c2ba
  artifacts:
    - artifact_id: ar_01KZ93TSS81QFHTPWTH7QVBZ3J
      digest: sha256:05df4e2f7f37200ea36365c315f4c14a8b9e57d686ba98bdfd5b5bfd58635d88
      locator: validation.md
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

## 交了什么

`fix/main-repo-index-pollution` 上的三个提交，基线 `b4df27a`，末版 `699652f`，
已经用 `--ff-only` 快进到本地 `dev`。

udf 不再把主插件仓库的暂存区搞脏。

## 落地核对

`dev` 的树和 `699652f` 的树都是 `ba8f734`，同一份。落地之后在 `dev` 上重跑三条门禁，
退出码都是 0，90 个用例全过。

没有推到 `origin`，也没有合进 `master`。

## 用户会看到什么变化

| 命令 | 之前 | 现在 |
| --- | --- | --- |
| `merge --strategy merge` | 主仓库留下一堆没人做过的暂存改动 | 索引和工作区跟着 HEAD 走 |
| `merge --strategy squash` 冲突时 | 留在带冲突标记的半合并状态 | 自动回滚，仓库干净 |
| `merge --strategy squash/rebase` 且仓库有未跟踪文件 | 被拒绝，说「工作区不干净」 | 正常执行 |

最后一条对用户是立竿见影的：`AesWorld` 长期有一个 `?? workflow/`，之前四种策略里有两种
用不了。

## 怎么回滚

`dev` 还没推 `origin`，`git reset --hard b4df27a` 就能回到落地前。没有数据迁移、没改
schema、没改配置格式。

## 遗留风险

- **修复不回溯清理已经发生的污染。** 哪个插件仓库现在还留着旧版本造成的残留，装新版之后
  它不会自己消失，要用户自己看一眼再处置。
- **没在用户的真实仓库上验过**，任务强约束要求全部在临时仓库里做。
- **审计只覆盖会动插件仓库的命令。** `switch` 动 Junction 和 `state.json`，`build` 动 Host
  目录，它们有没有各自的「留下残骸」问题，这次没查。
- **`git commit` 失败那条路只有代码走查**，没有直接测试。
- **多主插件没单独审计**，审计测试都是单主插件。
