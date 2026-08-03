---
schema_version: 1
artifact: design
artifact_id: ar_01KZ3K7ZXZF9KGDQ7DDP6Q2WSB
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
attempt_id: at_01KZ3FM1W8WB4J9MNKW8AQBXGK
created_at: 2026-08-03T10:38:21.631657Z
producer: aes-brainstorm
result: accepted
supersedes: ar_01KZ3K6ECGB9CBB8CJT7BNCG64
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts: []
---

## 目标

让人和 AI 都不用记住 20 个命令名，也不用每次敲 13 个字母。

## 背景

`unrealdevflow` 现在有 20 个平铺的顶层命令，而且命令名本身有 13 个字符。三处具体毛病都能看见：

- `build <task>` 编任务宿主，`build-project` 编主项目。「project」在这个工具的其他地方一律指 UE 项目，
  所以名字读起来是反的——新人会把 `build-project` 当通用动作、把 `build` 当主入口。
- 位置参数一半写 `<TASK_ID>` 一半写 `<TASK_REF>`，实际全都接受 `workspace/task-id`。
  `build-check [TASK_REF]` 可选而 `build-status <TASK_ID>` 必填，两个都是查任务状态。
- `--format` 是全局参数，20 个命令里只有 5 个真的读它。

还有两处冗余，清点时才发现：`start` 的实现就是调 `create` 并把描述当 prompt 传一遍
（`simple.rs:20-31`），没有任何别的行为；`configure` 和 `workspace add` 写的是同一份配置，
后者的参数是前者的超集。

真正的问题不是这几处，是**平铺加长名**。20 个名字已经超过靠记忆覆盖的规模，只能靠翻 `--help`。
而这个工具的第一使用者是 AI agent——`init` 会把 SKILL.md 装到四个 provider。
agent 不在乎多敲一个词，只在乎有没有歧义；人则相反，在乎每次都要敲 `unrealdevflow`。

**`udf` 不是新起的缩写，是仓库里早就在用的约定。** 现在已经有 `.udf-meta.json`、
`.udf-meta.json.bak`、`.udf-failed`、`.udf-backup`、`UDF_VERSION_LONG`、`UDF_BUILD_GIT_SHA`，
连受控构建门禁的白名单里都已经认 `udf` 和 `udf.exe`（`build_policy.rs:413-414`）。
只有可执行文件名还叫 `unrealdevflow`。

## 非目标

- 不改任何命令的实际行为。这次只动命令行表面：可执行名、分组、参数占位符、输出格式。
- 不碰受控构建的策略逻辑、Junction 切换逻辑、元数据 schema。
- 不改产品名和仓库名，它们仍然是 UnrealDevFlow。
- 不动用户已有的配置与状态。

## 方案对比

**方案一：改名为 `udf`，顶层收成四个名词组。** 删掉 `start` 和 `configure` 两个冗余命令，
所有叶子命令支持 `--format json`。代价是所有现有脚本和 SKILL.md 都要改，
`0.1.2` 的用户升级后旧命令直接报错。选它。

**方案二：只改可执行名，命令保持平铺。** 敲得短了，但 20 个平铺名字和三处毛病一个没解决。
没选，因为改名这次动了发布链路，不顺带把结构理顺等于白付一次迁移成本。

**方案三：只分组，不改可执行名。** 结构干净了但每次仍要敲 13 个字母，
而 `udf` 这个前缀代码里已经在用，不扶正就是继续两套叫法。没选。

**方案四：保留平铺和长名，旧名做隐藏别名。** 零破坏，但代码里长期并存两套名字，
`--help` 说一套、老脚本跑另一套。没选，因为它把成本从「改一次」变成「永远解释」。

**方案五：什么都不做。** 帮助文案上一条任务已经改成不说谎了，功能上没有缺陷。
没选，因为命令还会继续加——受控构建这次就加了三个——平铺的代价只会越来越大。

## 选定方案

### 可执行文件改名为 `udf`

产品名、仓库名、skill 名都不变。只有 `Cargo.toml` 的 `[[bin]] name` 和它的所有引用点改。

**用户已有的东西一律不动**：配置目录仍是 `~/.unrealdevflow/`，安装目录仍是
`~/.unrealdevflow/bin`，所以 User PATH 项完全不用改，也就没有 PATH 操作出错的风险。
环境变量 `UNREALDEVFLOW_CONFIG_DIR` 和 `UNREALDEVFLOW_UE_ENGINE_ROOT` 也保持原名——
命令改名是响亮的失败，敲错了立刻报错；环境变量改名是静默的失败，脚本失效后会退回默认目录，
让工具在错误的 workspace 集合上干活。这两种失败不是一个量级。

发布资产名 `unrealdevflow-installer.ps1` 保持不变，README 里那条一行安装命令的 URL 才不会失效。
安装器内部改为寻找并安装 `udf.exe`，同时**删掉同目录下遗留的 `unrealdevflow.exe`**——
否则它留在 PATH 里还能跑，跑的是旧版本，用户以为升级了其实没有。

### 顶层收成四个名词组

```
udf workspace  init | add | list | doctor | remove | status
udf task       create | list | next | switch | merge | finish | cleanup | delete
udf build      task | project | check | gate | status
udf skill      install | list | remove
udf aw-status                       # 隐藏，AgentWatcher 集成
```

`build task <ref>` 和 `build project` 并列之后，语义反转的问题自然消失——
不需要给谁改名，只需要把它们放到同一层级。

**对应关系**（旧名一律删除，不保留别名）：

| 现在 | 改成 |
| --- | --- |
| `unrealdevflow init` | `udf workspace init` |
| `unrealdevflow configure` | 删除，参数并入 `udf workspace add` |
| `unrealdevflow workspace add/list/doctor/remove` | `udf workspace ...`（子命令名不变） |
| `unrealdevflow status` | `udf workspace status` |
| `unrealdevflow start` | 删除，`udf task create` 不给 `--prompt` 时默认用描述 |
| `unrealdevflow create` | `udf task create` |
| `unrealdevflow list` | `udf task list` |
| `unrealdevflow next` | `udf task next` |
| `unrealdevflow switch` | `udf task switch` |
| `unrealdevflow merge` / `finish` / `cleanup` / `delete` | `udf task merge` / `finish` / `cleanup` / `delete` |
| `unrealdevflow build <ref>` | `udf build task <ref>` |
| `unrealdevflow build-project` | `udf build project` |
| `unrealdevflow build-check` | `udf build check` |
| `unrealdevflow build-gate` | `udf build gate` |
| `unrealdevflow build-status` | `udf build status` |
| `unrealdevflow skills install/list/remove` | `udf skill install/list/remove` |

### 同步做掉的三件事

一、位置参数统一叫 `<TASK_REF>`，语义统一为「`workspace/task-id`，只有一个 workspace 时可省略前缀」。
可选性也统一：查询类（`build check`、`build status`、`task next`）省略时按最近任务解析，
动作类（`task switch`、`build task`、`task merge`）必填。

二、`--format json` 每个叶子命令都支持。写操作命令返回它做了什么，不是只返回成功。

三、`init` 装出去的 SKILL.md、`AGENTS.md`、`CLAUDE.md`、`README.md`、`skill/SKILL.md` 全部跟着改。
文档不同步就等于没改。

## 边界与失败

**这是一次真正的破坏性变更。** `0.1.2` 已经发了三个 Release，装了的用户升级之后旧命令全不认。
版本要跳到 `0.2.0`，Release notes 必须带完整的改名对照表。

**旧名不保留任何别名。** 好处是代码里只有一套名字；代价是老脚本报的是 clap 的
`unrecognized subcommand`，不会告诉人新名字叫什么。这一点在发布说明里补，不在代码里补。

**AI skill 是最先受影响的。** 装在四个 provider 目录里的 SKILL.md 不会自动更新，
用户不重跑 `udf skill install`，agent 会继续用旧命令。这是本方案最可能出问题的地方。

**三个测试文件用 `Command::cargo_bin("unrealdevflow")` 拿二进制**，改 `[[bin]] name` 后
不同步就会全部失败。这是改名时最容易漏的一处，但它会立刻响亮地失败，不会静默。

**回退办法。** 命令行表面集中在 `cli.rs` 和 `main.rs` 两个文件的接线上，业务逻辑函数签名不动，
整体 `git revert` 即可。发布脚本和文档改动同理。配置与状态没动过，回退不涉及数据。

## 怎么算做对

- `udf --help` 顶层只列出 4 个组加 `help`。
- 每个叶子命令都能吃 `--format json` 并输出结构化结果。
- 全仓库搜不到 `build-project`、`build-check`、`build-gate`、`build-status`、`skills `、
  `configure`、`start` 这些旧命令名（Release notes 的对照表除外）。
- 装完之后 `~/.unrealdevflow/bin` 里只有 `udf.exe`，没有 `unrealdevflow.exe`。
- 现有 65 个测试全绿，`cargo fmt` / `clippy -D warnings` / `cargo test` 三件套通过。

## 未决问题

**一、`task build` 要不要作为 `build task` 的同义词存在。**
用户在 `task` 组里找编译是很自然的动作。选项是：只留 `build task`、两个都留、
或者在 `task --help` 里加一行指路。倾向第三个，零代码成本。下一步：写计划时定。

**二、`workspace status` 和 `build status` 两个 status 会不会混。**
前者答「哪个项目挂着哪个任务」，后者答「这个任务编到哪一步」。选项是保持现名、
把前者改成 `workspace active`、或者把后者改成 `build result`。
倾向保持现名——名词组已经消除了歧义。下一步：写计划时定。

**三、`0.2.0` 之外要不要同时发一个 `0.1.3` 只修文档。**
给还没准备好升级的用户一个过渡版本。取决于实际有没有这样的用户。
下一步：发布前问一次，本设计不做决定。
