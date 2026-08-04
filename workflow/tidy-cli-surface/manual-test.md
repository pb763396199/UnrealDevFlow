---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01KZ5G5351KXET1YXPYJ23ESP9
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
created_at: 2026-08-03T14:32:48.758407Z
producer: aes-validate
result: passed
supersedes: ar_01KZ40N9BPKNWQWGJANPPB2VXC
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ5DE0ZABZ72Y1SY2ZWA9BV0
      digest: sha256:be6b59fbe4c57191eb9289c168d7611fe911218eba32552747b11797a0ca80ad
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
## 怎么用这份清单

这些是机器验不了的，得你自己敲一遍看。按顺序做，前一条是后一条的前提。

过了在方括号里打 `x`，没过打 `!` 并在下面缩进写你看到了什么。改这个文件就是反馈，
不用回来跟我说。

**第二轮。** 上一轮你测了 1–10 条，3 条没过、2 条留了反馈，都改完了。下面标着
「重测」的四条是这次要你再看一眼的，其余保持你上轮的结论不动。重测大约十分钟。

## 换到新版

- [X] 1. 记下你现在的样子。开一个新的 PowerShell 窗口，敲 `unrealdevflow --version`，
  把版本号记下来；再敲 `unrealdevflow list`，把任务条数记下来。等下要对。
  上轮：0.1.2，12 个任务。
- [X] 2.（重测）装新版。在仓库目录里敲：
  `powershell -ExecutionPolicy Bypass -File scripts\install.ps1 -FromSource -SourceRoot .`
  这次要看两件事：一是第 4 步有没有打印「removed the superseded ...」，你机器上有
  `unrealdevflow.cmd` 和 `unrealdevflow.installed-20260630.exe` 两个，应该各打一行；
  二是第 5 步装 skill 有没有报错，上轮那里报了 `unrecognized subcommand 'skills'`
  却还说装好了，现在应该真的装上，报错的话整个安装会中断而不是假装成功。
  反馈，但第二步耗时非常长：[2/5] Installing binary and bundled skill source
    Building from source: F:\AiProject\UnrealDevFlow
     Compiling unrealdevflow v0.2.0 (F:\AiProject\UnrealDevFlow)
      Finished `release` profile [optimized] target(s) in 52.00s
- [X] 3.（重测）确认旧的没了、新的在。**关掉这个窗口，重开一个**，敲 `udf --version`，
  应该显示 0.2.0。再敲 `unrealdevflow --version`，应该提示找不到这个命令。
  上轮它还能跑出 0.1.2，就是第 2 条没删干净导致的。
  结果：过。`~/.unrealdevflow/bin` 只剩 `udf.exe` 和 `skills/`，`unrealdevflow.cmd`
  与 `unrealdevflow.installed-20260630.exe` 都被删了，`unrealdevflow` 已解析不到。
- [X] 4. 确认你的任务和配置都还在。敲 `udf task list`，条数应该跟第 1 条记的一样。
  再敲 `udf workspace list`，你的 workspace 应该还在，路径没变。
  上轮：12 条，对得上。

## 日常用一遍

- [X] 5.（重测）看看帮助现在是不是全中文。敲 `udf --help`，再随便挑两个叶子敲，
  比如 `udf task merge --help` 和 `udf build check --help`。
  上轮你说「我需要全是中文的注释，你现在写的都是英文」。现在 22 个叶子的说明、
  参数和取值都是中文了，连 clap 自带的「Usage/Commands/Options/Arguments」和
  「Print this message」也换成了「用法/命令/选项/参数/显示某条命令的帮助」。
  还剩一处没换：参数给错时 clap 吐的报错（比如 `error: the following required arguments were not provided`）还是英文，那串文案在 clap 库内部，换掉要自己
  重写它整套错误渲染，这次没做。你看看能不能接受。
  结果：过。用户明确表示 clap 那串英文报错可以接受，不用再改。
- [X] 6. 试着不带任务名查东西。敲 `udf task next`，再敲 `udf build status`。
  两条都应该自动挑你最近在弄的那个任务，不该报「缺少参数」。
- [X] 7. 试一条预览。挑一个你确定不想要的任务，敲
  `udf task delete <任务名> --dry-run`。看它列出来的东西对不对，
  然后确认它真的**没有**删掉任何东西——再敲一次 `udf task list` 看条数没变。
- [X] 8.（重测）让 AI 用一次。让你平时用的那个 AI 助手帮你建一个任务或者查一下状态。
  上轮你说「安装的技能中的描述应该用中文」。两份 SKILL.md 现在都是中文了，包括
  给 AI 读的那句 description。第 2 条装完之后四个 provider 目录里的说明书应该
  自动更新（上轮因为安装器报错，你是手动敲 `udf skill install --global` 才装上的）。
  这次不用手动敲，直接看 AI 用得对不对。
  结果：过。四个 provider 目录（.claude / .codex / .agents / .config/opencode）里的
  SKILL.md 都换成了新版中文，description 是「UE 插件并行开发工作流…」，安装器自动装的。
- [X] 9. 真编译一次。挑一个任务敲 `udf build task <任务名>`，看能不能编过。
  编之前可以先敲 `udf build check <任务名>` 问问现在能不能编。
- [X] 10.（重测）切到一个**你现在已经在上面的**任务，比如再敲一次
  `udf task switch neon-dev1/sublevel-distance-visibility`。
  现在开头应该多一行「项目 ... 已经在任务 ... 上，1 个 Junction 都指向正确目标。
  下面的重建是幂等的」。后面照旧清缓存、重建、重生成工程文件——你选的是
  「照做，但先说清楚」，所以行为没变，只是不再让你猜。
  上轮那条「未记账 Junction」警告不会再出现了：查过你的 state.json，DEV_1 本来
  就是空账本（旧版本留下的），那条警告报得对，你上轮这次 switch 已经把账写进去了。
  结果：过。提示行出现了，行为跟上轮一致。

## 移出清单的一条

原来的第 11 条是「等 0.2.0 发出来之后，找一台没装过的机器跑 README 那条一键安装命令」。
它必须等发布包才能做，而这次决定先把代码落到 dev、发布另说，所以它不该继续占着清单
挡住任务收口。已经转成交付记录里的遗留风险，发布 0.2.0 的时候再验。

清单到此闭合：10 条全过，0 条未测，0 条失败。
