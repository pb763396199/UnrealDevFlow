---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01KZ8ZMVE7NJARVHD1ZNC322XB
work_item_id: wi_01KZ8TFVAGQNAZH74W8JDZG6GA
created_at: 2026-08-04T10:45:00Z
producer: aes-brainstorm
result: accepted
supersedes: null
dependencies:
  work_item_contract_digest: sha256:33710c8da277ec096e0ee4c641b5343bb94e79e99040433659321b2812de8ed9
  artifacts: []
---

## 目标

看一眼分支名就知道这次改动是什么性质。

## 背景

`create.rs:90-91` 现在这样生成默认分支名：

```rust
custom_branch.unwrap_or_else(|| format!("task/{}/{}", workspace_name, task_id))
```

`workspace_name` 是工程名。它在分支名里不是给人看的，是给工具认的。

工具平时并不靠名字认。`.udf-meta.json` 里每个主插件都存了 `branch`，`merge.rs:237` 和
`delete.rs:159` 都是先拿这个字段比对，名字反推只是额外的容错。

**只有一处真的靠名字反推：`cleanup.rs::expected_branch_names`。** 它跑在 Host 已经丢失、
元数据读不到的孤儿路径上，只能拿 `(workspace, task-id)` 拼出候选名再去仓库里找。

所以分支名今天不能自由，根因是**身份被寄存在名字里**，而不是名字本身有什么讲究。

## 非目标

- 不改 worktree 布局、Host 目录结构、`.udf-meta.json` 的 schema。
- 不动已经存在的任务分支，也不做批量改名。
- 不改 Junction 切换、受控构建、合并策略。

## 方案对比

**一、只换命名规则，孤儿清理按名字尾部扫描。** 默认改成 `<type>/<task-id>`，孤儿清理不再
精确拼名，改成枚举本地分支、匹配以 `/<task-id>` 结尾的。代价：`--branch` 给了不含 task-id
的名字就找不回来，等于把自由度砍掉一半。没选，因为用户明确要的就是那份自由度。

**二、另存一份分支台账到 `~/.unrealdevflow/state.json`。** 全局文件记「任务 → 分支」。
代价：多一份要维护的全局状态，而且它跟分支不在同一个地方，仓库被移动、克隆或手工删分支之后
就会漂移，漂移了没人发现。没选。

**三、把身份写进 git 自己的分支配置。**（选定）建分支时同时写
`branch.<分支名>.udftask = <workspace>/<task-id>` 到主插件仓库的配置里。孤儿清理按任务引用
反查，不再猜名字。

**四、什么都不做。** 分支名继续带工程名。代价：用户提的问题原样留着，而且随着 workspace 变多，
分支名只会更长。没选。

## 为什么选三

台账跟分支存在同一个地方，**它们同生共死**，这是方案二给不了的。原型验证过四件事：

| 场景 | 结果 |
| --- | --- |
| 分支名叫 `release/totally-unrelated-name`，完全不含 task-id | 台账仍记得它属于 `neon-dev1/mytask` |
| Host 目录整个删掉 | 台账还在，它在主仓库的 `.git/config` 里 |
| `git worktree prune` 跑过（`cleanup` 自己会调） | 台账不受影响 |
| 按任务引用反查到分支并 `git branch -D` | 查得到；删完 git 自动带走 `branch.<name>.*` 配置段，无残留 |

最后一条决定了这个方案的成本：**不需要写清理代码**。git 删分支时会自己收走那一段配置。

## 选定方案

### 命名规则

默认分支名 `<type>/<task-id>`，不含工程名。

类型词表跟 AES Workflow 协议一致，也是这个仓库自己分支在用的那套：

| 类型 | 用在什么时候 |
| --- | --- |
| `feature` | 新功能、能力增强（默认） |
| `fix` | 缺陷修复 |
| `hotfix` | 线上急修 |
| `refactor` | 重构 |
| `docs` | 纯文档 |
| `chore` | 杂务 |

`task create` 加一个可选参数 `--type`，取值限定在上表，非法值在建任务之前就拒绝并列出可选值。
不给就是 `feature`。`--branch` 保持原样，给了就整个用它，`--type` 被忽略。

两个参数同时给时不报错而是忽略 `--type`，因为 `--branch` 表达的意图更具体；但会打一行提示，
免得调用方以为 `--type` 生效了。

### 分支台账

建分支成功之后，往该主插件仓库写：

```
branch.<分支名>.udftask = <workspace>/<task-id>
```

值用完整任务引用而不是裸 task-id，这样同名 task-id 落在不同 workspace 时不会互相认领。

读的地方只有一处——`cleanup.rs::expected_branch_names` 的孤儿路径。它改成两条腿走路：

1. 先查台账：枚举仓库里所有 `branch.*.udftask`，取值等于目标任务引用的那些分支。
2. 台账查不到再按老规则拼名字（`task-<id>`、`task/<ws>/<id>`），兜住这次改动之前建的任务。

`merge.rs` 的归属判定同样加一条：台账说它属于这个任务，就算匹配。原有三条判定一条不删。

### 为什么不需要迁移

老任务的分支名带工程名，没有台账。它们走第二条腿，行为跟今天完全一样。新任务两条腿都有。
不存在「必须先迁移才能用」的时刻。

## 边界与失败

**写台账失败怎么办。** 分支和 worktree 已经建好了，台账没写上。这时不该回滚整个任务——
分支是主产物，台账是辅助索引。打警告说明「孤儿清理将退回按名字反推」，继续往下走。

**用户手工改了分支名。** `git branch -m` 会把配置段一起搬走，台账自动跟上。这是 git 的行为，
不需要我们做什么。

**用户手工删了分支。** 配置段被 git 一起带走，台账不留孤儿条目。

**同一个仓库里两个任务撞了分支名。** 不可能——分支名在一个仓库里天然唯一，`create` 撞名时
本来就会失败。

**中断后回来接着做。** 台账是幂等写入，重复写同一个键是覆盖，不会累积。

## 怎么算做对

- 两个不同 workspace 下建同名 task-id，分支名一模一样，都不含工程名。
- `--type fix` 建出来的分支叫 `fix/<task-id>`。
- 非法 `--type` 在建 Host 之前就被拒，错误信息列出六个可选值。
- 拿一个完全不含 task-id 的 `--branch` 建任务，删掉 Host，`task cleanup` 仍能找到并清掉分支。
- 用老规则建的任务（`task/<ws>/<id>` 和 `task-<id>` 两种形态）仍能 merge 和 cleanup。
- 五份文档里的分支名示例跟新规则一致。

## 未决问题

无。三个决策点用户已经拍板：词表用 `feature/fix/hotfix/refactor/docs/chore`，`--type` 可选
默认 `feature`，`--branch` 完全自由且由台账兜底。
