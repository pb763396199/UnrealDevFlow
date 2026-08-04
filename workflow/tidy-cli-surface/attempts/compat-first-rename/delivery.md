---
schema_version: 1
artifact: delivery
artifact_id: ar_01KZ5G537HG5Z0YWFEAQQWXT0Z
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
attempt_id: at_01KZ3FM1W8WB4J9MNKW8AQBXGK
created_at: 2026-08-03T14:33:32.780772Z
producer: aes-finish
outcome: delivered
landing_branch: "dev"
landing_revision: ea9fa55ac58c7ecfd6f6b2fb7e9a05f52047102a
supersedes: ar_01KZ40PMBCBD4PEKNHMH9MB079
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ5DE0ZABZ72Y1SY2ZWA9BV0
      digest: sha256:c107071800b6b3e21489adbdbf6b56bb287e435edb7914faf7b28db428919d88
      locator: validation.md
  subject:
    kind: change_set
    digest: sha256:74cf36a04a4116b1603a55ffb325f397709574ec851f759e5daff7f4f200a2d1
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: bad22632e66ec81138d8b082b4ee6d29c95fc9a7
    revision: ea9fa55ac58c7ecfd6f6b2fb7e9a05f52047102a
    tree: 924a56dfeb98558e7ff3bc46f743abfca25aebe2
    content_digest: sha256:74cf36a04a4116b1603a55ffb325f397709574ec851f759e5daff7f4f200a2d1
    branch_or_pr: refactor/tidy-cli-surface
    workflow_excluded: true
---
## 交了什么

`refactor/tidy-cli-surface` 上的 17 个提交，基线 `bad2263`，末版 `ea9fa55`，已经用
`--ff-only` 快进到本地 `dev`。

可执行文件从 `unrealdevflow` 改名为 `udf`，20 个平铺命令收成 `workspace` / `task` /
`build` / `skill` 四个名词组，22 个叶子全部支持 `--format json` 并返回真实数据，
帮助文本全中文。版本 `0.2.0`。这是破坏性变更，旧命令名一个不留。

## 落地核对

`dev` 的树和 `ea9fa55` 的树都是 `924a56d`，逐字节相同，不是等价而是同一份。
落地之后在 `dev` 上重跑了三条门禁：`cargo fmt --check`、
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、
`cargo test` 退出码都是 0，71 个用例全过，`udf --version` 报 `0.2.0 (git ea9fa55ac)`。

没有推到 `origin`，也没有合进 `master`。

## 人工核对的结果

清单 10 条全过，0 条未测、0 条失败。第一轮打回三条（安装器两处、重复 switch 无提示），
外加两条反馈（帮助和 skill 描述要中文），修完之后第二轮全部通过。用户明确表示 clap
库内部那串英文报错可以接受，不再改。

原来的第 11 条「找一台没装过的机器跑一键安装」必须等发布包，这次决定先落 dev、发布另说，
所以它从清单移到下面的遗留风险里，不再挡住收口。

## 发布 0.2.0 时用户必须做的两件事

1. 重跑一次安装器，把 `~/.unrealdevflow/bin` 里的旧入口换成 `udf.exe`。安装器会删掉
   `unrealdevflow.exe`、`unrealdevflow.cmd` 和 `unrealdevflow.installed-*.exe`，这一条在
   用户的真实机器上验过了。
2. 装完 skill 会自动更新到新命令。第一轮这一步是坏的（安装器敲的还是旧的 `skills install`
   还谎报成功），现在修好了，四个 provider 目录实测都换成了新版中文 SKILL.md。

不做这两步的话，旧命令会直接报 `unrecognized subcommand` 且不指路——这是本次明确选择的
行为，`docs/releases/v0.2.0.md` 里有完整的改名对照表。

## 出了问题怎么退

`dev` 还没推到 `origin`，`git reset --hard bad2263` 就能完全回到落地前。已经推出去的话
`git revert` 整段区间。这次没有数据迁移、没有改元数据 schema、没有改配置文件格式，退回去
不会留半截状态。用户那边重装一次旧版安装器就能拿回 `unrealdevflow.exe`。

## 遗留风险

- **完整的「下载 Release zip → 解压 → 安装」链路没跑过**，因为还没有 `0.2.0` 的发布包。
  安装器删旧留新和构建失败中断这两条行为，用 `-FromSource` 在临时目录里按用户机器的真实
  目录形态实测过。发布 `0.2.0` 时要在一台没装过的机器上补验这条。
- **`0.2.0` 的 Release notes 只做了内容审查**，没跑 `generate-release-notes.ps1` 验证它能
  过模板校验。
- **`build task --background` 的 JSON 输出只有代码走查**，前台路径在冒烟里真编译验过。
- **项目内依赖 junction 这条路径没有覆盖**，四个可用主插件都没有本地项目插件依赖。
- **clap 库内部的报错文案仍是英文**。用户已确认接受。
- **本次的工作流记录是手工维护的**。会话中途 `aes-using-workflow` 的 `workflow_tool.py`
  被换成了没有 attempt 概念的旧版本，`validate` 和 `finish-check` 已经用不了。依赖摘要链
  是用它的 `digest` 子命令重算的（先验过两个版本算法逐字节一致）。这个工具不在本仓库里，
  要单独处理。
