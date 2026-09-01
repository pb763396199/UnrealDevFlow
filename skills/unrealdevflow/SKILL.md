---
name: unrealdevflow
description: UE 插件并行开发工作流（git worktree + NTFS Junction + 严格 UBT）。当用户要开发、调查、修复或验证一个虚幻引擎插件时使用。触发词：开发、调查、验证、修复、实现、测试功能。
user-invocable: true
argument-hint: "<任务描述>"
---

# UnrealDevFlow — UE 插件多任务并行开发

## 什么时候用

- 用户要开发、调查、修复或验证一个虚幻引擎插件
- 这次改动需要用 `git worktree` 跟主插件仓库隔离开
- 改完必须对着真实的 UE 编辑器目标编一遍（带严格依赖检查）
- 主插件依赖别的插件（比如 `EarthPCG` 依赖 `AesWorld`），两个要一起编

## 可执行文件在哪

`udf` 应该已经在 PATH 里。`Get-Command udf` 找不到就用安装目录里的：

```powershell
$udf = Get-Command udf -ErrorAction SilentlyContinue
if (-not $udf) {
    $udf = "$env:USERPROFILE\.unrealdevflow\bin\udf.exe"
}
```

两个都没有就先装：

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pb763396199/UnrealDevFlow/releases/latest/download/unrealdevflow-installer.ps1 | Out-String | iex"
```

## 小白入口（优先走这条）

第一次用的人，四条命令就够：

```powershell
udf workspace init --project "<UE项目目录>" [--workspace <名字>]
udf task create "<用户的原始需求>" --workspace <名字> --primary <插件> --id <task-id> --yes
udf task next <workspace>/<task-id>
udf task finish <workspace>/<task-id>
```

- `init` 自动探测项目、插件、引擎和 hosts，存成一个具名 workspace，装上 AI skill，再跑一遍 doctor。
- workspace 名字标识的是一个 UE 项目环境，不是一个任务。用工具建议的名字，或者项目名比如
  `neon-dev`；千万别拿 `sublevel-tweak` 这种任务名当 workspace 名。
- 配了不止一个 workspace 时，任务引用必须写全：`workspace/task-id`。有歧义时不要靠短 id 猜。
- `next` 只告诉用户下一步该做什么。
- `finish` 是合并向导，合并策略仍然要用户自己选，推荐默认 `rebase`。

## 五步标准工作流（给 AI 助手看的）

### 1. CREATE — 建一个隔离的任务工作区

```powershell
udf task create "<任务描述>" `
    --workspace <workspace名> `
    --id <task-id> `
    --primary AesWorld,AesWorld_AI `
    --override-dep PCG=project `
    --yes

# 想显式保存用户原话时：
udf task create "<任务描述>" `
    --workspace <workspace名> `
    --id <task-id> `
    --prompt "<用户的原始需求，一字不改地存下来>" `
    --primary AesWorld,AesWorld_AI `
    --override-dep PCG=project `
    --yes
```

- `--id` 要短，kebab-case，比如 `prefab-save-bug`。
- `--type` 决定分支名前缀，取值 `feature`（默认）/ `fix` / `hotfix` / `refactor` / `docs` /
  `chore`。**读完需求你来判断是哪一类**，别一律用默认值。
- `--prompt` **必须存住用户原话**，后面每个 AI 助手都要读它。不给 `--prompt` 时，任务描述会被
  当成原始需求存进去。
- 配了多个 UE 项目 workspace 时，`--workspace` 必填。
- `--primary` 接逗号分隔的多个插件名（v2 多插件）。
- `--override-dep` 解决引擎和项目里同名插件的冲突（`<名字>=engine|project|<绝对路径>`）。
- `--id` 只能用小写英文字母、数字和连字符，不能有 `/`、`\`、`..`、空格或中文。
- 每个主插件的主检出必须停在干净的 `dev` 分支上才能建任务。不要从 feature/task 分支、
  游离 HEAD、Host worktree 或 DEV 项目的 Junction 上建。
- `plugins_root` 必须是稳定的主插件仓库根目录，不能是 `<UE项目>/Plugins`、Host 目录、
  Junction/符号链接/重解析点，也不能是任何含有重名插件的路径。
- 工具会自动解析每个主插件的 `.uplugin`，扫描引擎和项目的插件根目录，给项目内依赖建 Junction。
- 新任务落在 `<hosts_root>/W-<workspace>/T-<task-id>_Host` 下。
- 新元数据会写入 `workspace`、`task_uid` 和一份冻结的 `context`，这样后面的命令不依赖可变的全局默认值。

**分支名怎么来的**：默认 `<type>/<task-id>`，比如 `feature/prefab-save-bug`，**不带工程名**。
`--type` 取六个值之一，不给按 `feature`。给了 `--branch` 就整个用它，`--type` 被忽略。

分支名可以随便起，包括完全不含 task-id 的名字。工具在主插件仓库的
`branch.<分支名>.udftask` 里记着它属于哪个任务，所以 Host 目录丢了之后
`task cleanup` 照样找得回这个分支。这条记录在 `git branch -D` 时由 git 自己带走。

### 2. WORK — 只在 worktree 里改代码

```
任务 worktree：{hosts_root}/W-<workspace>/T-<id>_Host/Plugins/<插件名>/Source/...
任务元数据：  {hosts_root}/W-<workspace>/T-<id>_Host/.udf-meta.json
任务引用：    <workspace>/<id>
```

- ✅ 在 worktree 里读、改、`git commit`。
- ✅ 提交信息必须是**中文**，格式 `Task#XXX [内容] / 修改内容 / 过程反思 / 后续注意`。
- ❌ 绝不改 `{plugins_root}/<插件>/`（主插件仓库）。
- ❌ 绝不改别的任务的 worktree，也不改 DEV 项目。

### 3. BUILD — 用严格参数编译（默认就是严格的）

```powershell
udf build task <task-ref>                    # 默认档位 light
udf build task <task-ref> --primary-only     # 只编主插件的模块（-Module=...）
udf build task <task-ref> --profile medium   # 再加 WarningsAsErrors
udf build task <task-ref> --profile heavy    # -Rebuild -DisableUnity -NoSharedPCH（慢）
udf build status <task-ref>                  # 省略 task-ref 就按最近动过的任务
```

- light 档位带的参数：`-FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity`。
- 失败日志在 `<host>/Logs/UBT/Build_<profile>_<时间戳>.log`。

#### 受控编译：编之前先问能不能编

同一个引擎目录只有一把 UnrealBuildTool 互斥锁。两个编译同时跑会互相搞坏中间产物，
所以永远不要自己拼一条 `Build.bat` 命令。

```powershell
udf build check <task-ref>                 # 现在能不能开编？
udf build check <task-ref> --format json   # 同样的结论，机器可读
udf build check --workspace <名字>          # 不给任务引用就是查主项目
```

四种结论。这条命令自己永远返回 0——结论就是答案，不是失败。

| 结论 | 意思 | 该怎么做 |
|---|---|---|
| `ready` | 可以开编 | 跑 `udf build task <task-ref>` |
| `deferred` | 别的 UBT 占着锁 | 等它结束，不要绕过去 |
| `blocked` | 被策略拒了（比如命令里带了 `-NoMutex`） | 看返回里的 `reason` 字段 |
| `needsUserInput` | 引擎或项目解析不出来 | 补 `--workspace`，或者修一下 workspace 配置 |

```powershell
# 你打算跑的那条命令，是不是绕过了受控编译？拒绝时退出码 1。
udf build gate "<你打算执行的完整命令>"

# 编 workspace 的主项目，而不是任务宿主
udf build project [--workspace <名字>] [--profile light|medium|heavy]
```

引擎根目录的解析顺序：workspace 的 `engine_path` → 环境变量
`UNREALDEVFLOW_UE_ENGINE_ROOT` → `.uproject` 里的 `EngineAssociation`。

### 项目打包配置

项目 task 首次需要打包时，先保存固定配置。之后 `package plan`、`check` 和 `project` 使用同一份配置。
常用配置只包含 `--configuration`、`--container`、`--output`、`--name` 和重复的 `--disable-plugin`。
MCP 等不能打包的插件可以按用户要求加入禁用列表，修改必须带 `--reason`。

```powershell
udf package configure --task <workspace/task-id> --configuration Shipping --container pak `
  --output "C:\Package" --name "UGA-task-Win64-Shipping" --disable-plugin ModelContextProtocol --reason "验证 Shipping 包"
udf package plan project --task <workspace/task-id>
udf package project --task <workspace/task-id>
udf package recover <execution-id>
```

`--output` 指包根目录，`--name` 指包目录名；省略 name 时自动生成项目、任务、平台和配置组合名。高级地图、数据映射和既有文件接管使用严格候选文件：
`udf package configure --task <workspace/task-id> --file <profile.toml> --reason "..."`。
`disabled_plugins = []` 才表示清空禁用列表，省略 `--disable-plugin` 表示保持原列表。
`recover` 只处理有完整事务日志的未完成交付，不重新 Cook，也不会凭目录内容猜测删除文件。

插件包可用 `udf package plugin <插件> --output <包根目录>`，多个插件按插件名分目录；Installed Build 可用 `udf package engine --output <包根目录> --name <目录名>`。省略 Installed Build 的 name 时使用 `InstalledBuild-Win64`，避免平台默认目录重名。

### 4. SWITCH — 没有用户明确授权，绝不执行

**⛔ 硬规则：用户没明确说要切，你就不许跑 `udf task switch`。**

这不是建议，是禁令。理由：

- `switch` 会改写正在运行的 UE 编辑器依赖的 NTFS Junction
- 编辑器开着的时候切，会搞坏这个编辑器会话
- 必须用户先关掉 UE 编辑器，再授权切换

编译成功之后，**告诉用户**，然后**等他明确发话**：

```
✅ 任务 <id> 编译通过。

要验收的话，请：
1. 关掉 UE 编辑器（如果开着）
2. 告诉我执行：udf task switch <task-ref>
3. 然后重启 UE 编辑器验证功能

验收通过之后：
  udf task finish <task-ref>
  udf task cleanup <task-ref>

验收没过：
  udf task delete <task-ref> --yes --force
```

就算用户说了「帮我切」，执行前也要把完整命令念一遍确认，因为 switch 对正在运行的
编辑器会话是破坏性的。

### 5. MERGE — 用户确认之后才做

**⚠️ 注意：`udf task merge` 会先自动从 origin 拉一次再合。**

这是为了让你合的是最新的远端状态。你干活期间要是有别的任务合进了 dev，这次合并会把那些
改动一起带进来。

`--strategy` 是**必填**的，而且你**必须问用户**要哪一个（1.rebase / 2.merge / 3.squash /
4.ff-only）。绝不替他选默认值。

```powershell
# 单个主插件
udf task merge <task-ref> --plugin AesWorld --strategy rebase

# 全部主插件（v2，按声明逆序，每个单独确认）
udf task merge <task-ref> --all --strategy rebase

# 只看会合什么，不真的合
udf task merge <task-ref> --all --strategy rebase --dry-run

# 看结果
git log --oneline -5

# 用户确认合并结果没问题之后，才做这一步：
udf task cleanup <task-ref>
```

## 🚫 硬规则

| ❌ 禁止 | ✅ 改用 |
|---|---|
| **没有用户明确授权就跑 `udf task switch`** | **把命令告诉用户，等他说「跑吧」** |
| `git merge` / `git rebase` / `git cherry-pick` | `udf task merge` |
| `git branch -D` / `git worktree remove` / `git reset --hard` | `udf task cleanup` / `delete` |
| 合并完直接自动 `cleanup` | 等用户明确确认 |
| 不问用户就自己定 `--strategy` | 先问，再把用户的选择传进去 |
| 改主插件仓库 `{plugins_root}/<插件>/` | 只改任务的 worktree |
| 直接跑 `Build.bat` / `RunUBT.bat` | 等 `udf build check` 说 `ready`，再跑 `udf build task` |
| 用 `-NoMutex` 绕过别人正在编 | 老实等 `deferred` 结束 |

## 出问题了怎么办

| 症状 | 怎么修 |
|---|---|
| `switch` 报「目录被占用」 | 关掉 Rider/VSCode/资源管理器，再跑 `udf task switch <task-ref> --force` |
| 报「Build.bat 不存在」 | 重跑 `udf workspace add --engine-path "正确路径"` |
| `build` 像是卡在别人的编译后面 | `udf build check <task-ref>`——`deferred` 就是别的 UBT 占着锁 |
| 合并顺序搞错了 | `git reflog`，然后 `git reset --hard <合并前的提交>`，重跑 `udf task merge` |
| 引擎和项目的依赖冲突 | `udf task create ... --override-dep <名字>=engine\|project` |
| 短任务 id 有歧义 | 用完整引用 `workspace/task-id` |
| `switch` 报「Broken junction」警告 | **会自动修**：`switch` 发现断链 Junction（目标已删）会自己移除。任务被删但没 cleanup 时会出现这种情况，不用手动处理。 |

## Junction 的生命周期（重要）

跑 `udf task delete <task-ref>` 时，工具会**自动清理 Junction**——所有已知主项目里指向这个
任务 worktree 的 Junction 都会被清掉。这是为了不留下会挡住以后 `switch` 的断链 Junction。

**`delete` 的时候发生了什么：**

1. 扫 `~/.unrealdevflow/state.json` 里所有项目
2. 找出指向这个任务 worktree 的 Junction
3. 从主项目里移除这些 Junction
4. 然后才删任务的 worktree 和分支

**`switch` 的时候发生了什么：**

- 碰到断链 Junction（目标已经不存在），自动移除并重建一个新的
- 你会看到一行警告：「Found broken junction at ... (target no longer exists)」
- 这是安全的、预期内的行为
- 如果这个项目本来就已经在这个任务上，会打一行提示告诉你，重建是幂等的

**最佳实践：**永远用 `udf task delete`，不要手动删 worktree 目录。这样 Junction 才会被正确清掉。

## 完整文档（在工具的源码仓库里）

- `AGENTS.md` — 完整规范：工作流、v2 多插件、提交格式、故障排查
- `README.md` — 面向用户的安装、init/create/next/finish、workspace 说明
- `docs/releases/v0.2.0.md` — 0.2.0 的命令改名对照表
- `docs/plans/2026-06-12-002-multi-workspace-and-simple-commands.md` — workspace 与小白命令的设计
- `docs/plans/2026-06-09-001-feat-multi-plugin-support-implementation-plan.md` — v2 设计
- `docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md` — UBT 严格编译参数考据
