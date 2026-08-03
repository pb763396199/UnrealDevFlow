---
schema_version: 1
artifact: manual-test
artifact_id: ar_01KZ40N9BPKNWQWGJANPPB2VXC
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
attempt_id: at_01KZ3FM1W8WB4J9MNKW8AQBXGK
created_at: 2026-08-03T14:32:48.758407Z
producer: aes-validate
result: pending
supersedes: ar_01KZ40JQ8GB00404CGTGJHXVZ9
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

## 怎么用这份清单

这些是机器验不了的，得你自己敲一遍看。按顺序做，前一条是后一条的前提。

过了在方括号里打 `x`，没过打 `!` 并在下面缩进写你看到了什么。改这个文件就是反馈，
不用回来跟我说。

第 1 到 4 条要换掉你现在装着的命令行工具，大约十分钟。第 5 条以后是日常用一遍，
看你想试多少，全做完大约二十分钟。

## 换到新版

- [ ] 1. 记下你现在的样子。开一个新的 PowerShell 窗口，敲 `unrealdevflow --version`，
      把版本号记下来；再敲 `unrealdevflow list`，把任务条数记下来。等下要对。

- [ ] 2. 装新版。在仓库目录里敲：
      `powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -FromSource -SourceRoot .`
      看它有没有打印一行「removed the superseded unrealdevflow.exe」。

- [ ] 3. 确认旧的没了、新的在。**关掉这个窗口，重开一个**，敲 `udf --version`，
      应该显示 0.2.0。再敲 `unrealdevflow --version`，应该提示找不到这个命令。
      找得到就是没删干净。

- [ ] 4. 确认你的任务和配置都还在。敲 `udf task list`，条数应该跟第 1 条记的一样。
      再敲 `udf workspace list`，你的 workspace 应该还在，路径没变。

## 日常用一遍

- [ ] 5. 看看顶层还剩几个命令。敲 `udf --help`。应该只有四个词加一个 help。
      问自己一句：不看说明的话，你猜得到自己要找的功能在哪个词底下吗。

- [ ] 6. 试着不带任务名查东西。敲 `udf task next`，再敲 `udf build status`。
      两条都应该自动挑你最近在弄的那个任务，不该报「缺少参数」。

- [ ] 7. 试一条预览。挑一个你确定不想要的任务，敲
      `udf task delete <任务名> --dry-run`。看它列出来的东西对不对，
      然后确认它真的**没有**删掉任何东西——再敲一次 `udf task list` 看条数没变。

- [ ] 8. 让 AI 用一次。让你平时用的那个 AI 助手帮你建一个任务或者查一下状态。
      看它敲出来的命令对不对。**注意**：装在四个 AI 目录里的说明书还是旧的，
      得先敲一次 `udf skill install --global` 才会更新。这一条就是在验这件事。

- [ ] 9. 真编译一次。挑一个任务敲 `udf build task <任务名>`，看能不能编过。
      编之前可以先敲 `udf build check <任务名>` 问问现在能不能编。

- [ ] 10. 换到任务上看一眼。敲 `udf task switch <任务名>`，然后打开 UE 编辑器，
      确认加载的是这个任务的插件。这条最花时间，不着急可以跳。

## 发布之后再补的

- [ ] 11. 等 0.2.0 发出来之后，找一台**没装过**这个工具的机器，敲 README 第一行那条
      一键安装命令，看能不能装上、装完 `udf --version` 有没有反应。
      现在还没有 0.2.0 的发布包，所以这条只能等。
