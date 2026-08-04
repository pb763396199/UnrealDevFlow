---
schema_version: 1
protocol: 1.3.0
artifact: plan
artifact_id: ar_01KZ3MVK2Z8BJ2RWJBS7G1338P
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
created_at: 2026-08-03T11:06:32.415991Z
producer: aes-plan
result: ready
supersedes: ar_01KZ3MDPWA7AVPBWNX13E3HA3M
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ3K7ZXZF9KGDQ7DDP6Q2WSB
      digest: sha256:901d16d8c3d7cded1d561265cfe118cdb5fdf8215fba42340bee68fc69dcac9c
      locator: design.md
---

## 基线

仓库 `F:\AiProject\UnrealDevFlow`，分支 `refactor/tidy-cli-surface`，基线版本
`bad2263`（上一条路线的收口提交）。设计 `ar_01KZ3K7ZXZF9KGDQ7DDP6Q2WSB`，已 `accepted`。

**改动规模**（开工前实测）：可执行名在代码、脚本、CI 里有约 114 处引用；
`AGENTS.md` 65 条、`README.md` 42 条、`skills/unrealdevflow/SKILL.md` 38 条、
`CLAUDE.md` 37 条、`skill/SKILL.md` 18 条命令示例；15 个命令模块里只有 5 个收 `format` 参数。

**每步都要过的门**：`cargo build` 成功。S1 之后每步还要过 `cargo test`——
改名会让三个集成测试文件立刻失败，那是响亮的失败，不许带着走。
最后一步之前要过完整三件套。

**测试先行**：S4 到 S7 每一步都先改集成测试里的命令写法（此时测试会红），
再改 `cli.rs` 和 `main.rs` 让它变绿。红-绿是这几步的验证手段。

## 步骤

### S1 可执行名改成 udf（只动代码）

**用到**：`Cargo.toml`、`tests/*.rs`、`src/commands/skills.rs`、`src/config.rs`、`src/error.rs`、`src/cli.rs`。

**改**：`[[bin]] name` 改成 `udf`。三个测试文件里 19+15+6 处 `Command::cargo_bin("unrealdevflow")`
改成 `cargo_bin("udf")`。`skills.rs` 里 14 处路径推断注释和 `SKILL_NAME` 之外的可执行名引用同步。
源码里出现的示例命令文案一并改。

**不改**：`SKILL_NAME` 仍是 `unrealdevflow`；`CONFIG_DIR_ENV` 仍是 `UNREALDEVFLOW_CONFIG_DIR`；
`ENGINE_ROOT_ENV` 仍是 `UNREALDEVFLOW_UE_ENGINE_ROOT`；`~/.unrealdevflow` 两处路径常量不动。

**变成什么样**：`cargo build` 产出 `target/debug/udf.exe`，`udf --help` 能跑，
命令结构还是旧的 20 个平铺命令。

**验证**：`cargo test` 65 个用例全绿。`ls target/debug/udf.exe` 存在。
`grep -rn 'cargo_bin("unrealdevflow")' tests/` 零命中。

**证据**：`cargo test` 输出、`udf --version` 输出。

**回退**：`git revert` 这一步。

### S2 发布链路跟着改名

**用到**：`scripts/install.ps1` 18 处、`dist/unrealdevflow-installer.ps1` 18 处、
`scripts/package-release.ps1` 6 处、`scripts/release-preflight.ps1` 3 处、
`.github/workflows/release.yml` 3 处、`scripts/install-skill.ps1` 2 处、
`scripts/generate-release-notes.ps1` 2 处、`aw/Get-AgentWatcherModule.ps1` 1 处。

**改**：打包与安装器改为寻找、校验、安装 `udf.exe`。安装器在安装成功之后
**删掉同目录下遗留的 `unrealdevflow.exe`**。

**不改**：资产名 `unrealdevflow-installer.ps1` 和 zip 名保持原样，
安装目录仍是 `~/.unrealdevflow/bin`，PATH 写入逻辑一行不动。

**变成什么样**：装完之后 `~/.unrealdevflow/bin` 里只有 `udf.exe`；老用户升级后
旧的 `unrealdevflow.exe` 不会留在 PATH 里冒充新版本。README 那条一行安装命令的 URL 仍然有效。

**验证**：`pwsh scripts/release-preflight.ps1 -Strict` 通过。用 `-FromSource` 跑一次
`scripts/install.ps1` 到临时目录，确认落地的是 `udf.exe`；再手工放一个假的
`unrealdevflow.exe` 进去重跑，确认它被删掉。

**证据**：preflight 退出码、安装前后的目录清单。

**回退**：`git revert` 这一步。已经装出去的目录不受影响。

### S3 输出层地基

**用到**：`src/output.rs`、`src/cli.rs`。

**改**：给 `output` 加一个统一的命令结果形状，让写操作命令也能用同一套方式产出 JSON，
而不是 15 个模块各写各的。`--format` 从「全局但多数命令忽略」改成「全局且每个叶子命令都实现」。

**为什么排在前面**：S4 到 S7 每一步都要给自己那组补 JSON 输出。地基不先立，
四组会长出四套不一样的输出结构，AC-004 就成了四个人各自解释。

**变成什么样**：命令实现只需要构造一个结果对象交给 `output`，人类文案和 JSON 由同一处产出。

**验证**：`cargo test` 仍全绿（这一步不改任何命令行为）。新增单元测试覆盖结果对象的
两种渲染路径。

**证据**：`cargo test output` 输出。

**回退**：`git revert` 这一步，S1、S2 不受影响。

### S4 workspace 组

**用到**：`src/cli.rs`、`src/main.rs`、`src/commands/workspace.rs`、`src/commands/init.rs`、
`src/commands/status.rs`、`src/commands/configure.rs`（删除）。

**改**：新增 `Workspace` 组，含 `init`、`add`、`list`、`doctor`、`remove`、`status`。
顶层 `init`、`configure`、`status` 三个命令删除，`configure` 的参数并入 `workspace add`。

**变成什么样**：`udf workspace init --project ...` 等价于原来的 `unrealdevflow init`；
原 `configure` 的能力通过 `workspace add` 达成；原顶层 `status` 变成 `workspace status`。
六个叶子命令都支持 `--format json`。

**验证**：先改测试里这几条命令的写法，跑一次确认变红；改完实现再跑，
`tests/create_sources.rs` 里涉及 workspace 的用例全绿。手工跑
`udf workspace list --format json` 与 `udf workspace status --format json` 拿到结构化输出。

**证据**：红-绿两次 `cargo test` 输出、两条 JSON 输出。

**回退**：`git revert` 这一步。

### S5 task 组

**用到**：`src/cli.rs`、`src/main.rs`、`src/commands/create.rs`、`list.rs`、`simple.rs`、
`switch.rs`、`merge.rs`、`finish.rs`、`cleanup.rs`、`delete.rs`。

**改**：新增 `Task` 组，含 `create`、`list`、`next`、`switch`、`merge`、`finish`、
`cleanup`、`delete`。顶层同名命令与 `start` 一并删除。`task create` 不给 `--prompt` 时
默认用描述当 prompt，这样 `start` 就没有存在理由了。位置参数统一改成 `<TASK_REF>`，
`switch`、`merge`、`cleanup`、`delete` 必填，`next` 可省。

**这是本计划最大的一步。** 八个叶子命令、四个测试文件都受影响。

**验证**：先改 `tests/create_sources.rs`、`tests/merge_yes.rs`、`tests/multi_plugin.rs`
里的命令写法，确认变红；改完实现再跑，34 个集成用例全绿。
手工跑 `udf task list --format json` 与 `udf task next --format json`。

**证据**：红-绿两次 `cargo test` 输出、两条 JSON 输出、`udf task --help`。

**回退**：`git revert` 这一步。

### S6 build 组

**用到**：`src/cli.rs`、`src/main.rs`、`src/commands/build.rs`、`build_status.rs`、
`src/commands/build_policy.rs`。

**改**：新增 `Build` 组，含 `task`、`project`、`check`、`gate`、`status`。
顶层 `build`、`build-project`、`build-check`、`build-gate`、`build-status` 全部删除。
`build task` 与 `build status` 的位置参数统一成 `<TASK_REF>`，
`build check` 与 `build status` 省略时按最近任务解析。

**注意**：`build_policy.rs` 里 `CONTROLLED_BUILD_ACTIONS` 与
`is_direct_controlled_command` 认的是旧的平铺动作名。分组之后，
`udf build task x` 的第一个参数变成 `build`、第二个才是 `task`，
识别逻辑必须跟着改，否则 `build-gate` 会把自己人当外人拦下。
这一处有现成的 6 个单元测试，改完必须仍然全绿。

**变成什么样**：五个叶子命令都支持 `--format json`；`build gate` 仍然在拦下时退出码 1。

**验证**：`cargo test build_policy` 6 个用例全绿。手工跑 `udf build check --format json`
拿到四种结论之一；`udf build gate "udf build task x --mutex no-mutex"` 仍被拦下、退出码 1。

**证据**：`cargo test` 输出、`build check` 的 JSON、`build gate` 两个方向的退出码。

**回退**：`git revert` 这一步。

### S7 skill 组

**用到**：`src/cli.rs`、`src/main.rs`、`src/commands/skills.rs`。

**改**：`Skills` 组改名成 `Skill`（单数，跟其他三组一致），三个子命令名不变，
补上 `--format json`。

**不改**：`SKILL_NAME` 仍是 `unrealdevflow`，装出去的目录名不变。

**验证**：`udf skill list --format json` 输出结构化的四个 provider 安装状态。

**证据**：JSON 输出。

**回退**：`git revert` 这一步。

### S8 文档、版本与发布说明

**用到**：`AGENTS.md`、`CLAUDE.md`、`README.md`、`skill/SKILL.md`、
`skills/unrealdevflow/SKILL.md`、`docs/RELEASE.md`、`Cargo.toml`。

**改**：约 200 条命令示例全部改成新形态。版本从 `0.1.2` 跳到 `0.2.0`。
Release notes 模板里加一张完整的改名对照表——这是唯一允许出现旧命令名的地方。

**变成什么样**：五份文档里每条命令都能照着敲通。`init` 装出去的 SKILL.md 教的是新命令。

**验证**：把五份文档里的命令示例逐条抄出来跑一遍（只读命令实跑，写操作命令用 `--help` 确认
参数存在）。`grep` 确认旧命令名只剩 Release notes 对照表里那一处。

**证据**：`grep` 结果、只读命令的实跑输出。

**回退**：`git revert` 这一步。

### S9 端到端冒烟：在真实项目上把新命令跑一遍

**这一步是本计划唯一能证明命令真的可用的环节。** 前面八步的验证都是单元测试和
`--help` 文案，证明不了 `udf task create` 在真实 UE 项目上建得出 worktree。

#### 固定参数

| 项 | 值 |
| --- | --- |
| 测试 workspace 名 | `cli-smoke`（与现有 `neon-dev1`、`neon-ecdev` 不冲突） |
| 主项目 | `F:\ShanghaiP4\neon\UGA\DEV`（`UGA.uproject`，EngineAssociation 5.5） |
| hosts_root | `F:\ShanghaiP4\neon\Hosts`（测试产物全部落在 `W-cli-smoke\` 下） |
| plugins_root | `F:\ShanghaiP4\neon\Plugins` |
| 引擎 | `C:\Program Files\Epic Games\UE_5.5` |

#### 可用主插件

四个插件，覆盖从最小到真实规模的排列。全部是 `DEV\Plugins` 里本来就在用的。

| 插件 | 在 `DEV\Plugins` 的形态 | 模块数 | `.uplugin` 依赖 | 源仓库状态 | 承担哪些用例 |
| --- | --- | --- | --- | --- | --- |
| `ArtCommon` | junction → 源库 | 0 | 无 | `dev`、干净、无附属 worktree | T1、T6 |
| `AesWorld` | junction → 源库 | **44** | 14 个，**全是引擎插件** | 分支 `new_vege_editor_clean`、有 `?? workflow/`、**13 个活 worktree** | T2 |
| `WdpCamera` | 实体目录，306M | 1 | EnhancedInput、ProceduralMeshComponent（引擎插件） | `dev`、干净、无附属 worktree | T3、T4 |
| `WdpEnvironment` | 实体目录，70M | 1 | 无 | `dev`、干净、无附属 worktree | T4 |

**`AesWorld` 的三条额外约束**，不遵守就会出事：

1. **必须给 `--base-ref dev`。** `create.rs:646-662` 的「当前分支必须是 `dev`」和
   「主仓工作区必须干净」两个检查整个包在 `if base_ref.is_none()` 里。`AesWorld` 当前在
   `new_vege_editor_clean` 且有未跟踪的 `workflow/`，不给 `--base-ref` 会被连拒两次。
   给了就两个都跳过，基线取 `dev`（当前 `8d8c2dabb`）。
2. **不许切换它的当前分支，不许碰已有的 13 个 worktree。** 给它建一个新 worktree 是它的
   日常用法——现有 13 个 Host 每个都这么干；不日常的是动它正在工作的分支。
3. **不在它身上做真实编译。** 44 个模块意味着 `--primary-only` 会传 44 个 `-Module=`，
   等于全量构建。编译验证交给 T3 的 `WdpCamera`（1 个模块）。

**明确排除，不许当主插件**：

| 插件 | 排除原因 |
| --- | --- |
| `AesRuntimeCore` | 分支是 `5.5` 不是 `dev`；用 `--base-ref` 能绕过，但它不承担任何独有用例，不值得引入风险 |
| `EarthArtAsset` | 8 个未提交改动 |
| `SkyCreatorPlugin` | 分支 `dev_project` 且有未提交改动 |
| `Developer` | 是 RiderLink，不是插件仓库 |

#### 开跑前的前置条件

- **DEV 的 UE Editor 必须关闭。** 测试会反复改 `DEV\Plugins` 下的 junction，
  Editor 开着会锁住目录，`switch` 会报冲突。
- **磁盘至少留 20GB。** `AesWorld` 的 worktree 约 10GB（工作区 12G、`.git` 1.8G）。
  当前 `F:` 剩余约 172GB，够用，但跑之前确认一次。
- 单次完整跑一遍预计 30 到 60 分钟，其中 `AesWorld` 建 worktree 和 `WdpCamera` 真实编译各占大头。

#### T0 准备与基线记录（不是测试，但漏了就没法还原）

1. 记录 `DEV\Plugins` 当前 8 个条目的清单，以及 `ArtCommon` 和 `AesWorld` 两个 junction 的目标路径。
2. **用改名而不是复制**把两个实体目录挪开：
   `DEV\Plugins\WdpCamera` → `DEV\Plugins\WdpCamera.bak-cli-smoke`，`WdpEnvironment` 同理。
   306M 和 70M，改名是瞬时的，复制不是。
3. 对 `ArtCommon`、`WdpCamera`、`WdpEnvironment` 三个源仓库各记一份基线：
   `git worktree list`、`git branch --list`、`git status --porcelain`、`git rev-parse dev`。
4. 对 `AesWorld` 源仓库单独记一份，这份最要紧：`git worktree list`（**应为 13 条**）、
   `git branch --show-current`（应为 `new_vege_editor_clean`）、`git status --porcelain`
   （应只有 `?? workflow/`）、`git rev-parse dev`。
5. 备份 `~/.unrealdevflow/config.toml` 和 `state.json`。
6. `udf workspace init --workspace cli-smoke --project F:\ShanghaiP4\neon\UGA\DEV
   --hosts-root F:\ShanghaiP4\neon\Hosts --plugins-root F:\ShanghaiP4\neon\Plugins
   --skip-skill-install -y`
   —— `--skip-skill-install` 是必须的，否则它会去改用户四个 provider 的全局目录。

#### 测试组合

**T1 单主插件 · 最小 · `ArtCommon`**

| 步 | 命令 | 看什么 |
| --- | --- | --- |
| 1 | `udf workspace list --format json` | 输出里有 `cli-smoke` |
| 2 | `udf workspace doctor cli-smoke --deep` | 通过 |
| 3 | `udf workspace status --format json` | 结构化输出，`DEV` 当前无 active task |
| 4 | `udf task create "冒烟：最小主插件" --workspace cli-smoke --id t1-artcommon --primary ArtCommon -y` | Host 建出、worktree 建出 |
| 5 | `udf task list --format json` | 能看到 `cli-smoke/t1-artcommon` |
| 6 | `udf task next cli-smoke/t1-artcommon --format json` | 给出下一步 |
| 7 | `udf build check cli-smoke/t1-artcommon --format json` | `ready`/`deferred`/`blocked`/`needsUserInput` 之一 |
| 8 | `udf task switch cli-smoke/t1-artcommon` | `DEV\Plugins\ArtCommon` 的 junction 目标变成 Host 里的 worktree；**这一组不加 `--skip-regen-project-files`**，顺带验证 UBT 重生成工程文件那条路 |
| 9 | `udf build status cli-smoke/t1-artcommon --format json` | 结构化输出 |
| 10 | `udf task merge cli-smoke/t1-artcommon --strategy rebase` | 任务分支上没有提交，所以是 no-op；**跑完 `git rev-parse dev` 必须和 T0 记录一致** |
| 11 | `udf task cleanup cli-smoke/t1-artcommon` | worktree、分支、Host 全部回收 |

**T2 单主插件 · 真实规模 · `AesWorld`**

这一组是对「工具在真实生产插件上还能不能用」的检验，也顺带在一个 10GB 的 worktree 上
验证上一条路线修的 `git::worktree::remove(repo, worktree)`——在 Windows 上从拥有它的仓库
驱动删除，而不是从被删目录里驱动。

| 步 | 命令 | 看什么 |
| --- | --- | --- |
| 1 | `udf task create "冒烟：真实规模" --workspace cli-smoke --id t2-aesworld --primary AesWorld --base-ref dev -y` | **必须带 `--base-ref dev`**；建出后 `AesWorld` 源仓库的 worktree 变成 14 条 |
| 2 | 立刻 `git -C <AesWorld源库> branch --show-current` | 仍是 `new_vege_editor_clean`，没被工具切走 |
| 3 | 读 Host 的 `.uproject` | 14 个引擎依赖被 enable，**没有为它们建任何 junction** |
| 4 | `udf build check cli-smoke/t2-aesworld --format json` | 拿到结论；`validationCommand` 里的路径不带 `\\?\` 前缀 |
| 5 | `udf task switch cli-smoke/t2-aesworld --skip-regen-project-files` | `DEV\Plugins\AesWorld` 指向新 worktree；**加 `--skip-regen-project-files`**，否则 UBT 会为 12G 插件重生成工程文件，很慢 |
| 6 | `udf task merge cli-smoke/t2-aesworld --strategy rebase` | no-op；`git rev-parse dev` 与 T0 一致 |
| 7 | `udf task cleanup cli-smoke/t2-aesworld` | 10GB worktree 被干净删除；`git worktree list` **回到 13 条**；`task/cli-smoke/t2-aesworld` 分支消失 |
| 8 | `udf task switch main` 或手工复位 | `DEV\Plugins\AesWorld` 指回 `F:\ShanghaiP4\neon\Plugins\AesWorld` |

**不做**：任何形式的真实编译。44 个模块的构建不属于冒烟测试。

**T3 单主插件 · 有模块 · 有引擎依赖 · `WdpCamera`**

| 步 | 命令 | 看什么 |
| --- | --- | --- |
| 1 | `udf task create "冒烟：引擎依赖" --workspace cli-smoke --id t3-wdpcamera --primary WdpCamera -y` | 不需要 `--base-ref`，它本来就在干净的 `dev` 上 |
| 2 | 读 Host 的 `.uproject` | `EnhancedInput` 与 `ProceduralMeshComponent` 被 enable，且没有为它们建 junction |
| 3 | `udf task switch cli-smoke/t3-wdpcamera` | `DEV\Plugins\WdpCamera` 变成 junction（原实体目录已在 T0 挪走） |
| 4 | `udf build task cli-smoke/t3-wdpcamera --primary-only` | **本计划唯一一次真实编译**；只编 `WdpCamera` 一个模块。看输出实时滚动、`-Module=WdpCamera` 生效 |
| 5 | `udf build status cli-smoke/t3-wdpcamera --format json` | 状态是 `success` 或 `failed`，不是永远 `building` |
| 6 | `udf task merge cli-smoke/t3-wdpcamera --strategy rebase` 然后 `udf task cleanup` | 回收；`git rev-parse dev` 不变 |

**T4 多主插件 · `WdpCamera` + `WdpEnvironment`**

| 步 | 命令 | 看什么 |
| --- | --- | --- |
| 1 | `udf task create "冒烟：多主插件" --workspace cli-smoke --id t4-multi --primary WdpCamera,WdpEnvironment -y` | 两个 worktree、两个分支、同一个分支名 |
| 2 | `udf task switch cli-smoke/t4-multi` | `DEV\Plugins` 下**两个**都变成 junction，不是只切第一个 |
| 3 | `udf task merge cli-smoke/t4-multi --strategy rebase`（不给 `--plugin`） | 应报错要求选边 |
| 4 | `udf task merge cli-smoke/t4-multi --all --strategy rebase --dry-run` | 预览按逆序列出两个插件 |
| 5 | `udf task merge cli-smoke/t4-multi --plugin WdpCamera --strategy rebase`，再 `--plugin WdpEnvironment` | 逐个合并；两个源仓库的 `dev` HEAD 都不变 |
| 6 | 先做 T5，再 `udf task cleanup cli-smoke/t4-multi` | 两个 worktree、两个分支、Host 全部回收 |

**T5 拒绝路径（复用 T4 的任务，在它 cleanup 之前做，不新建任务）**

| 步 | 命令 | 看什么 |
| --- | --- | --- |
| 1 | `udf task switch cli-smoke/t4-multi --project F:\ShanghaiP4\neon\UGA\DEV_1` | 被拒绝，提示任务已绑定 `DEV`，且**明确说未修改任何 Junction** |
| 2 | 立刻看 `DEV\Plugins` 和 `DEV_1\Plugins` | 两边的 junction 都和第 1 步之前一模一样 |
| 3 | `udf build gate "Build.bat UGAEditor Win64 Development"` | 拦截，退出码 1 |
| 4 | `udf build gate "udf build task cli-smoke/t4-multi --mutex no-mutex"` | 拦截，退出码 1，原因 `no_mutex_forbidden` |
| 5 | `udf build gate "udf build check cli-smoke/t4-multi"` | 放行，退出码 0 |
| 6 | 一边跑 `udf build task cli-smoke/t4-multi`，另一边跑 `udf build check cli-smoke/t4-multi` | 第二条返回 `deferred`；编译结束后再跑返回 `ready` |

**T6 delete 路径与空主插件守卫 · `ArtCommon` 一次性任务**

| 步 | 命令 | 看什么 |
| --- | --- | --- |
| 1 | `udf task create "冒烟：删除路径" --workspace cli-smoke --id t6-scrap --primary ArtCommon -y` | 建出 |
| 2 | 手工把 Host 的 `.udf-meta.json` 里 `primary_plugins` 改成 `[]`（先复制一份原文件） | —— |
| 3 | `udf task cleanup cli-smoke/t6-scrap` | 被拒绝，说明元数据缺主插件身份 |
| 4 | `udf task delete cli-smoke/t6-scrap`（不加 `--force`） | 被拒绝 |
| 5 | 把 `.udf-meta.json` 还原 | —— |
| 6 | `udf task delete cli-smoke/t6-scrap --yes` | worktree、分支、Host 全部回收，不经过 merge |

**T7 skill 组（只读加临时目录，不碰全局）**

| 步 | 命令 | 看什么 |
| --- | --- | --- |
| 1 | `udf skill list --format json` | 四个 provider 的安装状态，结构化 |
| 2 | `udf skill install --project <一个空临时目录>` | 四个目录里都出现 `SKILL.md` |
| 3 | 打开其中一份搜 `build check` | 教的是新命令，不是 `build-check` |
| 4 | `udf skill remove --project <同一临时目录>` | 清干净；删掉临时目录 |

**T8 项目内依赖 junction（可选，需要用户点头才做）**

四个可用主插件**都没有**声明对本地项目插件的依赖——`AesWorld` 的 14 个依赖查过了，
全是引擎插件——所以这条路径盖不到。要覆盖得用 `AesArtAsset`（`.uplugin` 依赖 `ArtCommon`，
源仓库在 `dev`、干净、无附属 worktree），但它**不在 `DEV\Plugins` 里**，
`task switch` 会临时给主项目加一个 `AesArtAsset` junction。收尾时删掉即可，
但这违反了「只用本来就在用的插件」这条约束，所以默认不做。

#### 收尾还原清单（逐条打勾，缺一条这一步都不算完）

1. `DEV\Plugins\ArtCommon` 是 junction，目标是 `F:\ShanghaiP4\neon\Plugins\ArtCommon`，与 T0 记录一致。
2. `DEV\Plugins\AesWorld` 是 junction，目标是 `F:\ShanghaiP4\neon\Plugins\AesWorld`，与 T0 记录一致。
3. `DEV\Plugins\WdpCamera`、`WdpEnvironment` 是实体目录（`.bak-cli-smoke` 改名移回），
   `.bak-cli-smoke` 后缀的目录不存在。
4. `DEV\Plugins` 的条目清单和 T0 记录的 8 个完全一致，不多不少。
5. `F:\ShanghaiP4\neon\Hosts\W-cli-smoke` 不存在。
6. `ArtCommon`、`WdpCamera`、`WdpEnvironment` 三个源仓库：`git worktree list` 只有 1 条、
   `git branch --list` 里没有 `task/cli-smoke/*`、`git status` 干净、仍在 `dev`、
   `git rev-parse dev` 与 T0 记录一致。
7. **`AesWorld` 源仓库**：`git worktree list` 回到 **13 条**、
   `git branch --list "task/cli-smoke/*"` 为空、
   `git branch --show-current` 仍是 `new_vege_editor_clean`、
   `git status --porcelain` 仍只有 `?? workflow/`、`git rev-parse dev` 与 T0 一致。
   这条最容易出事，也最容易被忽略——10GB 的 worktree 删不干净会留下 stale 注册。
8. `udf workspace remove cli-smoke` 执行过，`udf workspace list` 里没有 `cli-smoke`。
9. `~/.unrealdevflow/config.toml` 与 `state.json` 和 T0 备份一致（或至少不含 `cli-smoke`
   与指向 `DEV` 的路由）。
10. `F:\ShanghaiP4\neon\Hosts` 下 `T-prefab-web-sync_Host`、`W-neon-dev1`、`W-neon-ecdev`
    三个原有条目仍在，内容未变；这三个 workspace 里 12 个任务的 Host 目录数量与 T0 一致。

**任何一条对不上就停下来查清楚，不要接着往下走。** 残留的 worktree 注册和分支会让下一次
`create` 撞名字失败，残留的 junction 会让主项目编译时装载错误的插件版本。

**验证**：T1 到 T7 全部走完，收尾清单 10 条全部打勾。

**证据**：每组命令的实际输出（`--format json` 的存原文）、T0 与收尾两次的
`git worktree list` / `git branch` / `DEV\Plugins` 清单对照，其中 `AesWorld` 的
13 → 14 → 13 三个时点各记一次。

**回退**：这一步不改仓库代码。真实环境的残留按收尾清单逐条手工清理，
清单本身就是回退步骤。

### S10 收尾核对

**用到**：全仓库。

**改**：不改代码，除非发现遗漏。

**变成什么样**：`udf --help` 顶层只列出四个组加 `help`；全仓库搜不到旧命令名。

**验证**：`udf --help` 数顶层条目，应为 5（四组加 `help`，`aw-status` 隐藏）。
`grep -rn` 搜六个旧命令名，除 Release notes 对照表外零命中。
跑完整三件套：`cargo fmt --check`、
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test`。

**证据**：`--help` 输出、`grep` 结果、三条门禁的退出码。

**回退**：这一步只读，没有回退需求。

## 顺序和并行

S1 → S2 可以并行，但 S2 的验证需要 S1 产出的 `udf.exe`，所以建议串行。

S3 是 S4 到 S7 的共同前提，必须先做完，否则四组会长出四套输出结构。

S4、S5、S6、S7 之间**不能并行**：四步都写 `cli.rs` 和 `main.rs` 这两个共用文件。
按依赖从少到多串行：workspace、task、build、skill。

S8 要等 S4 到 S7 全部完成，否则文档会写到一半的形态上。

S9 的端到端冒烟要等 S1 到 S8 全部完成：它要用真实的 `udf` 二进制和新命令形态跑，少一步都跑不成。S10 最后。

## 风险

**最大的一处**：S6 里 `build_policy.rs` 的受控命令识别。分组把动作词从第一个参数
推到第二个，`is_direct_controlled_command` 不跟着改，`build gate` 就会把自己人拦下。
它有 6 个单元测试兜底，但测试里的命令字符串也要一起改——两边同时改容易一起改错。
改完要手工验一次真实命令。

**第二处**：S5 的 `task` 组一步动八个叶子命令和四个测试文件。如果中途发现某个命令的
参数语义需要调整，会牵连整步。真遇到就停下来单独处理，不要边改边扩大范围。

**第三处**：S8 的两百条文档示例靠人眼核对容易漏。只读命令必须实跑，不能只看着像对。

**第四处，也是唯一会弄脏真实环境的**：S9 在用户的生产项目上建 worktree、改 junction。其中 T2 直接在 `AesWorld` 上建一个约 10GB 的 worktree，它同时还挂着 13 个生产任务的 worktree。回收不干净就会留下 stale 注册，所以收尾清单把它单列一条，并且要求 13 → 14 → 13 三个时点各记一次。`AesWorld` 有 13 个活 worktree，一旦误操作影响面很大，所以它被明确列入禁止清单，并且收尾清单里用它的 worktree 数量作为「没碰过生产插件」的证据。两个实体目录 306M 和 70M，备份必须用改名不能用复制。

## 不做什么

不改任何命令的实际行为。不碰受控构建策略逻辑、Junction 切换逻辑、元数据 schema。
不改配置目录、安装目录、PATH 项、两个环境变量名。不改 `SKILL_NAME`。
不给旧命令名留别名。不做 shell 补全。不做配置迁移工具。
