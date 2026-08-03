---
schema_version: 1
artifact: review
artifact_id: ar_01KZ40MSSATP1D6K1K3CH4WW1A
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
attempt_id: at_01KZ3FM1W8WB4J9MNKW8AQBXGK
created_at: 2026-08-03T14:32:32.810351Z
producer: aes-review
verdict: approved
supersedes: ar_01KZ402F0VD41DPCXVQHX6WCQC
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ40KNHCMHJFZHTY17CGTA2T
      digest: sha256:296dca774469053bafcb9a675fe3fcdad17390a7d1a9f09b8d570b6af690a060
      locator: ../implementation.md
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

## 审查范围

基线 `bad2263`，末版 `46fe42a`，分支 `refactor/tidy-cli-surface`，13 个提交。

第一版结论是 `changes_requested`，两条阻断在 `09ec1db` 修完并逐条复核过。之后验收又发现
两处问题，补了 `e0b715d` 和 `46fe42a`，这两个提交也单独看过（见文末）。本版结论 `approved`。
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

## 没有覆盖的范围

- **完整的 Release 安装链路没跑过**。安装器删旧 exe 只用 `-FromSource` 验过，
  「下载 zip → 解压 → 安装」这条路要等 `0.2.0` 发出来才能验。
- **`build task --background` 的 JSON 输出只有代码走查**。前台路径在冒烟 T3 的真实编译里验过。
- **`skill install` / `remove` 没有实跑**。冒烟全程带 `--skip-skill-install`，T7 被跳过。
- **项目内依赖 junction 这条路径完全没有覆盖**，实现记录里说明了原因（四个可用主插件都没有
  本地项目插件依赖）。
- **S11 与 S12 的修复都没有重跑端到端冒烟**。改动都在输出层与配置豁免上，用 JSON 输出逐条实测，
  没有再动用户的真实项目。
- **`0.2.0` 的 Release notes 只做了内容审查，没有跑 `generate-release-notes.ps1` 验证它能被
  模板校验通过**（那段脚本会拒绝含占位符的 notes）。
