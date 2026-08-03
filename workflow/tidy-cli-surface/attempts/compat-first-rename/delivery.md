---
schema_version: 1
artifact: delivery
artifact_id: ar_01KZ40PMBCBD4PEKNHMH9MB079
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
attempt_id: at_01KZ3FM1W8WB4J9MNKW8AQBXGK
created_at: 2026-08-03T14:33:32.780772Z
producer: aes-finish
outcome: ready_to_land
supersedes: null
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ40N1BGZ6GJRGHMTZM585QH
      digest: sha256:9b7f0923b723b3623f691094580ff6a3474de167619d5fa860971caa4879e2c0
      locator: validation.md
  subject:
    kind: change_set
    digest: sha256:abc7dc42bdb017d8c6ab99c0eb05d2100c3d87c71d16c26b3a441797065871d6
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: bad22632e66ec81138d8b082b4ee6d29c95fc9a7
    revision: 46fe42a0e91e8fcbbcf18c7d92106a8f70e814a7
    tree: d2ef02c83135ef13a6c070f76d4ffa9238825e11
    content_digest: sha256:abc7dc42bdb017d8c6ab99c0eb05d2100c3d87c71d16c26b3a441797065871d6
    branch_or_pr: refactor/tidy-cli-surface
    workflow_excluded: true
---

## 交的是什么

`refactor/tidy-cli-surface` 上的 13 个提交，基线 `bad2263`，末版 `46fe42a`。

可执行文件从 `unrealdevflow` 改名为 `udf`，20 个平铺命令收成 `workspace` / `task` /
`build` / `skill` 四个名词组，22 个叶子全部支持 `--format json` 并返回真实数据。
版本 `0.2.0`。这是破坏性变更，旧命令名一个不留。

## 现在能落地吗

代码这边可以：评审 `approved`，验收七条全 `passed`，三条门禁在末版退出码都是 0，
71 个用例全过。变更集和实现记录逐条对得上，工作区除了 `workflow/` 没有别的改动。

但**还不能宣告完成**，人工核对清单 11 条一条都没勾。这份改动换掉的是用户每天敲的命令，
机器能证明 `--help` 里写着什么，证明不了换过去之后用着顺不顺手、AI 助手会不会照着新命令
敲对。清单在
`workflow/tidy-cli-surface/attempts/compat-first-rename/manual-test.md`。

## 落地办法

合到 `master` 之前先在本地 `dev` 上快进验一遍，跟上一条路线一样。因为是破坏性变更，
落地时要同步发 `0.2.0`：`docs/releases/v0.2.0.md` 已经写好，包含完整的改名对照表。

发布之后用户必须做两件事，否则会撞墙：
1. 重跑一次安装器，把 `~/.unrealdevflow/bin` 里遗留的 `unrealdevflow.exe` 换成 `udf.exe`。
   安装器会自己删旧的，这一步验过。
2. 敲一次 `udf skill install --global`，把装在四个 AI provider 目录里的说明书换成新命令。
   不做这一步，AI 助手会一直敲旧命令，而旧命令现在直接报 `unrecognized subcommand`、不指路。

## 出了问题怎么退

分支还没合，直接不合就行。已经合了的话：`git revert` 整段区间，或者把 `master` 退回
`bad2263`。这次没有数据迁移、没有改元数据 schema、没有动配置文件格式，退回去不会留下
半截状态。用户那边只要重装一次旧版安装器就能拿回 `unrealdevflow.exe`。

## 已知还没覆盖的

- 完整的「下载 Release zip → 解压 → 安装」链路没跑过，因为还没有 `0.2.0` 的发布包。
  安装器删旧留新的行为用 `-FromSource` 在临时目录里实测过。
- `build task --background` 的 JSON 输出只有代码走查，前台路径在冒烟里真编译验过。
- `skill install` / `remove` 没有实跑，冒烟全程带 `--skip-skill-install`，没动用户的
  四个 provider 目录。
- 项目内依赖 junction 这条路径没有覆盖，四个可用主插件都没有本地项目插件依赖。
