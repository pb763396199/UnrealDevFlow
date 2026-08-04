---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01KZ5DCTJAWBDSR926HPJGZ2DF
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
created_at: 2026-08-04T03:34:37.386367Z
producer: aes-review
verdict: approved
supersedes: ar_01KZ40MSSATP1D6K1K3CH4WW1A
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ5D544083RHJB4KHYDBNT42
      digest: sha256:548a2727f801f55e847d30f8d65fbf64348590ab945383ed7a222f20c563dd3e
      locator: ../implementation.md
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

## 审查范围

基线 `bad2263`，末版 `ea9fa55`，分支 `refactor/tidy-cli-surface`，16 个提交。

这份记录经过三轮。第一版 `changes_requested`，两条阻断在 `09ec1db` 修完。验收又发现两处，
补了 `e0b715d` 和 `46fe42a`。**人工核对第一轮打回三条**，补了 `adcc789`，复审那个提交时
又发现一处，补了 `ea9fa55`。本版结论 `approved`，但人工核对第二轮还没做完。
下面保留原问题描述，各自附上复核结果。

逐个提交核对过文件清单，没有范围外改动：`workflow/` 一次都没进过提交，`docs/plans`、
`docs/insights`、`docs/brainstorms`、`docs/reports` 这些历史记录没动，`dist/` 是 gitignore 的
生成产物没有提交。三条发布门禁在末版通过，71 个用例全过。

## 原来必须修的问题（已复核修复）

### 1. `build gate` 丢了配置豁免，未配置的机器上直接失败

`src/main.rs:65-71` 的豁免名单现在是 `AwStatus | Workspace | Skill`。上一条路线在
`0dd7479` 里特意把 `BuildGate` 加进这个名单——它只做字符串检查，不读任何配置。
S6 把五个平铺的 build 命令收进 `Build` 组时，`Commands::BuildGate { .. }` 这个变体没了，
豁免也跟着消失，而整个 `Build` 组不在名单里。

实测：`UNREALDEVFLOW_CONFIG_DIR=<空目录> udf build gate "Build.bat x"` 返回
「UnrealDevFlow 未配置。请先运行 `udf workspace init`」，而不是判定结果。

后果具体：`build gate` 的设计用途是放进 hook 或 wrapper 里拦命令，`README` 和两份 SKILL.md
都是这么写的。在全新机器、CI 容器、或者任何还没跑过 `workspace init` 的环境里，这个 hook
现在会挂掉——而它本该在那种环境里也能工作，因为它不需要知道任何 workspace。

**最小改法**：豁免名单里加一条针对叶子的匹配，例如
`Commands::Build { action: cli::BuildAction::Gate { .. } }`。组里其余四个叶子确实要配置，
不能整组豁免。

**复核（`09ec1db`）：已修。** `src/main.rs:97-105` 把豁免抽成 `needs_config`，`Build` 组只
豁免 `Gate` 这一个叶子。空配置目录实测：`udf build gate "Build.bat x"` 返回
`{"command":"build gate","ok":true,"data":{"status":"blocked"}}`；同一环境下
`udf build check` 仍返回未配置错误。组豁免与叶子豁免的边界是对的。

### 2. `--dry-run` 与取消路径没有结构化输出

AC-004 要求「每个叶子命令都接受 `--format json` 并输出结构化结果，写操作命令返回它做了什么」。
`task delete --dry-run --format json` 实测返回：

```json
{ "command": "task", "ok": true, "messages": [ "ℹ Dry run: would delete task ...", ... ] }
```

没有 `data`，命令名是组名 `task` 而不是叶子名。原因是这些路径在构造出结果对象之前就
`return Ok(())` 了，靠 `output::flush_unemitted` 的安全网兜底。同类路径至少有这些：

- `src/commands/delete.rs:194` 与 `:234`（dry-run 预览、用户取消）
- `src/commands/merge.rs:430`（dry-run 分支）
- `src/commands/cleanup.rs:168`、`:190`、`:297`（无匹配分支、用户取消）
- `src/commands/create.rs:230`（用户取消）
- `src/commands/switch.rs:38`（Editor 运行时用户选择不继续）

`--dry-run` 不是边角路径。它写在 `README` 的命令参考里，`skill/SKILL.md` 的速查表里有两条，
本次冒烟的 T4 也用它验证了多插件逆序。一个「预览要删什么」的命令，机器读不到要删什么，
等于没有覆盖到。

**最小改法**：给 dry-run 和取消这两类路径各定义一个结果对象再 `emit`。dry-run 的数据本来就
已经算出来了（`delete.rs` 里的 `reports` 就是），只是打成了日志行；取消路径可以复用现有的
`XxxOutcome` 结构，把布尔字段置成未执行。

**复核（`09ec1db`）：已修，六个文件全覆盖。** 逐条核对：

- `delete.rs` 新增 `DeletePreview`/`PluginPreview`，dry-run 走 `emit`；`DeleteOutcome` 加
  `cancelled` 位，取消路径也 emit。
- `merge.rs` 把 `merge_single_plugin` 的返回值从 `bool` 换成 `PluginMergeReport`，四个返回点
  全部经过同一个 `report()` 构造器，不会漏。`MergeOutcome` 加 `dryRun` 与 `plugins`。这比我
  建议的最小改法走得更远，但方向对：`--all --dry-run` 的逆序预览现在机器读得到每个插件的
  分支、仓库、worktree、待合并提交数、领先/落后数和未提交数。
- `cleanup.rs` 加 `untouched()` 构造器，覆盖三个出口；孤儿分支清理的收尾原来也没 emit，
  这次一并补了——这一条我上一版没点出来，是实现方自己找到的。
- `create.rs`、`switch.rs` 的取消路径复用各自 Outcome，`render_*` 前置判断分支。

实测：`task delete --dry-run --format json` 返回 `data.dryRun=true` 且 `data.plugins` 有 1 项；
`task merge --dry-run --format json` 带出 11 个待合并提交。四条命令逐个数过，各自只有一个
JSON 文档，安全网没有再叠加。

## 建议修但不阻断

### 3. `task finish --format json` 报的命令名是 `task merge`（已修）

`src/commands/finish.rs:23` 直接把工作交给 `merge::run`，而后者 `emit("task merge", ...)`。
调用方发起的是 `task finish`，拿回来的信封说自己是 `task merge`。单文档没问题，但命令名
对不上发起的命令。跟 S4/S7 处理 `workspace init` 的方式一致的话，应该拆一个 `merge_inner`，
由 `finish` 自己 emit。

**复核（`09ec1db`）：已修。** 拆出 `merge::run_inner` 返回 `MergeOutcome`，`merge::run` 和
`finish::run` 各自 emit 自己的命令名。跟 S4/S7 的模式一致。

### 4. 安全网报的是组名，不是叶子名（已修）

`src/main.rs:31` 把 `command_name` 的结果（`workspace` / `task` / `build` / `skill`）交给
`flush_unemitted` 和 `emit_failure`。所以任何走安全网或失败的命令，信封里的 `command` 只有
组名。问题 2 里那个 `"command": "task"` 就是这么来的；失败时同理——`udf task switch x` 报错，
信封说 `command: "task"`。调用方分不清是组里哪个叶子出的事。

**建议改法**：`command_name` 下探到叶子，返回 `"task switch"` 这样的两词名。

**复核（`09ec1db`）：已修。** `src/main.rs:46-80` 的 `command_name` 现在下探到全部 22 个叶子。
实测 `udf task switch nonexistent/x --format json` 的信封是 `"command": "task switch"`。

### 5. `workspace status` 把「读不出来」变成了「无效」

`src/commands/status.rs:30` 现在是 `junction::exists(...).unwrap_or(false)`。这修掉了单个
探测失败搞挂整条命令的问题（真问题，改对了），但代价是丢失了「不知道」这个状态：一个被
索引服务或 UE 占住的 junction（os error 32）会被报成 `junctionValid: false`，跟「这个
junction 真的没了」无法区分。

**建议改法**：`junctionValid` 改成三态，或者另加一个 `probeError` 字段把原始错误带出来。

**复核：未做，可接受。** 实现记录里明确写了不做及理由——这是精度问题不是缺陷，`unwrap_or(false)`
比原来「一个探测失败搞挂整条命令」严格更好。留作后续。

### 6. 装出去的 skill 与新命令之间有一个升级缺口

`skills/unrealdevflow/SKILL.md` 已经改到新命令，但装在四个 provider 目录里的副本不会自动
更新。`docs/releases/v0.2.0.md` 点明了这一条，也给了动作（重跑 `udf skill install --global`），
所以不算缺陷。但值得记下来：旧命令按本次设计是直接报 `unrecognized subcommand`、不指路，
所以没升级 skill 的 agent 会连撞几次才可能反应过来。

**复核：未做，符合预期。** 这本来就是发布动作不是代码缺陷，`docs/releases/v0.2.0.md` 已写明。

## 我确认过没有问题的地方

- **断链 junction 的修复是对的，而且补上了真正的根因。** `junction::exists` 在 crate 说
  「不是」时改问 `symlink_metadata` 的 `is_symlink`，这让 `is_broken` 第一次真的可能返回 true。
  `delete` 退回 `remove_dir` 在 Windows 上对重解析点只删链接不碰目标——这条有专门的回归测试
  `deleting_a_live_junction_leaves_the_target_alone` 守着，写得对。三个新测试都在测行为不是测实现。
- **受控命令识别跟着分组改成两词匹配是必须的，也改对了。** `CONTROLLED_BUILD_ACTIONS` 从
  `build-check` 这类单词换成 `build check` 这类两词，`udf buildxyz` 这种前缀相似的会被拦下
  （实测确认）。6 个单元测试全绿。
- **组合命令的 `*_inner` 模式解决了多文档问题。** `workspace init` 调 `add_inner`、
  `doctor_inner`、`install_inner`，一条命令只吐一个信封。这是对的抽象，不是权宜之计。
- **用户已有的东西确实一样没动。** `~/.unrealdevflow/`、`~/.unrealdevflow/bin`、User PATH 项、
  `UNREALDEVFLOW_CONFIG_DIR`、`UNREALDEVFLOW_UE_ENGINE_ROOT`、`unrealdevflow-installer.ps1`
  资产名、README 里的一行安装 URL——逐条核对过，全部保持原样。
- **S11 的修复没有夹带范围外改动。** `09ec1db` 只动 7 个源文件、261 增 50 删，全部集中在
  评审点名的出口上。三条门禁在这一版重跑，71 个用例仍然全过。修的过程中 clippy 抓到一次
  `#[allow(clippy::too_many_arguments)]` 被插错位置（挂到了新结构体上），当场改回。
- **端到端冒烟的收尾是真做了。** `config.toml` 与 `state.json` 与 T0 备份逐字节一致，
  `AesWorld` 仍是 13 个 worktree 且分支是 `new_vege_editor_clean`，`DEV/Plugins` 仍是 8 项。
  实现记录里如实写了「计划的 merge no-op 假设是错的」和「T2 的 merge 因主仓库不干净没跑成」，
  没有粉饰。

## 验收补的两个提交（`09ec1db..46fe42a`，6 个文件 16 增 8 删）

这两个提交是验收阶段发现问题后补的，没有经过第一轮评审，单独看一遍。

**`e0b715d` 改发布资产清单里的二进制名。** 三行改动，两个文件。`skills/unrealdevflow-release/SKILL.md`
的资产清单原来写 `unrealdevflow.exe`，跟 `package-release.ps1:68/81/86` 实际产出的 `udf.exe`
对不上；zip 名和安装器名按强约束保留原样，这次只动了二进制这一行，边界是对的。`.gitignore`
的那处是注释，不影响行为。

**`46fe42a` 让 `build status` 的任务引用可省略。** `Option<String>` 加一个 `match` 落到
`latest_task_ref`，跟 `finish.rs:15-18` 和 `build_policy.rs` 里 `build check` 的写法完全一致，
不是新发明的解析路径。`let task_id = task_id.as_str();` 这一行遮蔽写法看着别扭，但 `String`
必须先活到函数末尾才能借出 `&str`，写成 `&task_id` 会在下一行就被 drop——这是必要的，不是
凑合。行为上唯一的变化是省略参数时不再报 clap 缺参错误；带参数时走的还是原来那条路。
文档里两处 `<task-ref>` 跟着改成 `[task-ref]`，尖括号和方括号在这份文档里一直是区分必填
可选的，改对了。

这两个提交都没有碰输出层、受控构建策略和 Junction 逻辑，三条门禁在 `46fe42a` 重跑仍是 0，
71 个用例全过。

## 人工核对第一轮打回的三条（`adcc789`）

这三条全是我的验收没覆盖到的地方，值得单独说清楚为什么漏了。

**安装器的三处问题，我的 AC-001 验证方法本身有缺陷。** 我在临时目录里只塞了一个
`unrealdevflow.exe`——那正好是安装器代码里写着要删的那一种。等于拿被测代码的假设去
造测试数据，测了个寂寞。用户机器上真实的形态是 `unrealdevflow.cmd` 加
`unrealdevflow.installed-20260630.exe`，两个都没删。修法是按名字形态匹配而不是硬编码
一个文件名，`unrealdevflow-installer.ps1` 因为不匹配 `^unrealdevflow(\.installed-.*)?$`
而保住，这个边界是对的，实测确认。

`skills install` 那处更直接：S7 把命令单数化，`install.ps1:223` 没跟上，而脚本无条件
打印成功。我的 AC-003 全仓扫描排除了 `scripts/` 吗？没有——但我扫的是 `build-project`
这类旧命令名，没扫 `skills install` 这个组合。教训是改名之后应该反过来扫「所有调用自身
可执行文件的地方」，而不是只扫旧名字。

第三处（构建失败仍报成功）是我在复现用户的目录形态时撞到的，用户没提。同一类问题：
PowerShell 里调原生命令不检查 `$LASTEXITCODE` 就是静默吞错。三处现在都检查了。

**重复 switch 的提示。** 用户选了「照做，但先说清楚」，所以 `report_if_already_active`
只打印不改变控制流，放在所有诊断之后、真正动手之前。判定条件是账本的 `active_task` 对得上
**且**每个 Junction 的实际目标都对得上，两个都满足才说话——只对一半时保持沉默是对的，
那种情况下面的重建确实在做实事。`junction::get_target` 失败按「对不上」处理，也对。

我核对了用户报的「未记账 Junction」警告：S9 冒烟的 T0 备份里 DEV_1 是 `active_task: None`、
0 个 junction，磁盘上却有旧版本留下的活 Junction。警告报得对，不是缺陷，这一点实现记录里
写清楚了没有含糊过去。

**帮助文本中文化。** 22 个叶子的 doc comment 全改，这部分是机械的。有判断的是 `main.rs`
里那个 `localize`：不逐个给 22 个变体标注 `help_template`，而是建好命令树之后递归套一遍。
理由成立——derive 属性只作用于标注的那一层，逐个标既啰嗦又必漏。`mut_args` 按
`is_positional()` 分成「参数」和「选项」两个标题，没有混成一段。每一层自动生成的 `help`
子命令都单独换了说明，27 个帮助页扫过没有残留。

`Cli` 上新增的 `help` / `version` 两个字段是 clap 的惯用法（关掉自动 flag 换成自己声明的
同名参数），不是死代码，clippy 在 `-D warnings` 下也过了。

解析入口从 `Cli::parse()` 换成 `from_arg_matches`，这是行为面最大的改动，我单独验了运行时：
`-V`、`--version`、`udf help task`、全局 `--format` 放在命令前后两种位置、`-v`、隐藏的
`aw-status`、缺参数退出码 2、未知子命令退出码 2——都对。

## 复审 `adcc789` 时发现的（`ea9fa55`）

`aw-status --format json` 吐两个 JSON 文档：它自己打印的固定契约，加上 `flush_unemitted`
兜底补的信封。这是 S3 给 `list` 和 `status` 修过的同一类问题，当时没看这个隐藏命令。

修法我认可：它不能改成走 `emit`，AgentWatcher 读的就是那个固定形状。加一个 `mark_emitted`
让自己负责整个 stdout 的命令声明一声，注释里明写「只给已有外部消费方的固定契约用，新命令
一律走 emit」——这个口子有边界，不会变成绕过信封的后门。

## 没有覆盖的范围

- **完整的 Release 安装链路没跑过**。安装器删旧 exe 只用 `-FromSource` 验过，
  「下载 zip → 解压 → 安装」这条路要等 `0.2.0` 发出来才能验。
- **`build task --background` 的 JSON 输出只有代码走查**。前台路径在冒烟 T3 的真实编译里验过。
- **`skill install` / `remove` 没有实跑**。冒烟全程带 `--skip-skill-install`，T7 被跳过。
- **项目内依赖 junction 这条路径完全没有覆盖**，实现记录里说明了原因（四个可用主插件都没有
  本地项目插件依赖）。
- **S11、S12、S13 的修复都没有重跑端到端冒烟**。改动都在输出层与配置豁免上，用 JSON 输出逐条实测，
  没有再动用户的真实项目。
- **clap 内部的报错文案仍是英文**。参数给错时的 `error: the following required arguments
  were not provided` 在 clap 库里，要换得自己实现一套 `ErrorFormatter` 重写全部错误渲染。
  这次没做，已经写进人工清单第 5 条让用户判断能不能接受。
- **`0.2.0` 的 Release notes 只做了内容审查，没有跑 `generate-release-notes.ps1` 验证它能被
  模板校验通过**（那段脚本会拒绝含占位符的 notes）。
