# UnrealDevFlow Agent 指南

本文档面向 AI agent（如 Codex、Copilot、Claude Code、Cursor、opencode），说明如何使用 UnrealDevFlow 工具协助用户完成 UE 插件开发任务。

## 本仓库自身的开发流程：AES Workflow

本仓库采用 AES Workflow。**改本仓库的代码之前，先走 `aes-using-workflow`**，由它找到当前任务并决定下一步交给哪个 Skill。用户没点名也要走。

- 流程状态就是 `workflow/` 目录里的文件，没有数据库也没有后台服务。
- 干活期间只提交代码，`workflow/` 下的记录留在工作区，等任务收口时一次性提交。两者混在一次提交里工具会拒绝。
- 提交信息用 `workflow_tool.py commit-message` 生成骨架，任务编号和标题工具自己填。

这一节只管**开发 UnrealDevFlow 这个工具本身**。用 UnrealDevFlow 工具做 UE 插件开发的流程，看下面的 5 步标准工作流，两者互不替代。

## 发布流程硬规则

当用户要求发布、打包、创建 GitHub Release、准备版本或修复安装分发流程时，必须先读取并遵守 `skills/unrealdevflow-release/SKILL.md` 与 `docs/RELEASE.md`。

- 禁止跳过 `scripts/release-preflight.ps1 -Strict`。
- 禁止在 `cargo fmt`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test` 或 release build 失败时打 tag。
- 禁止手写 GitHub Release 资产；必须通过 `scripts/package-release.ps1` 生成。
- GitHub Release 必须先创建 draft，资产齐全并验证 installer 后才能 publish。
- 默认安装入口必须是 GitHub Release installer，不是 clone 仓库后本地编译。

---

## 🚀 0 压力入口（新用户 / 新 agent 优先）

新用户不用先理解所有底层命令，优先走：

```powershell
udf workspace init --project "<UE项目目录>" [--workspace <name>]
udf task create "用户原始需求" --workspace <name> --primary <Plugin> --id <task-id> --yes
udf task next <workspace>/<task-id>
udf task finish <workspace>/<task-id>
```

规则：
- `init` 自动探测 UE 项目、Plugins 根目录、Engine 路径、Hosts 目录，保存为 workspace，并安装 AI skill。
- workspace 名称可由工具建议，也可由用户用 `--workspace` 指定；内部会规范成 kebab-case。
- 只有一个 workspace 时可省略 `--workspace`；多个 workspace 时必须显式指定，避免 session 串项目。
- `task create` 不给 `--prompt` 时会把描述当原始需求存进元数据。
- `task create` 的 `--type` 决定分支名前缀，不给按 `feature`。
- `next` 只告诉用户下一步该执行什么，不输出长说明书。
- `finish` 是验收通过后的合并向导，仍必须由用户选择合并策略，默认推荐 rebase。

## 🚀 5 步标准工作流（任何 AI agent 必读）

> **这一节是给 agent 看的速查版本**。完整规范、详细解释、FAQ 在本文后半部分。
> 如果你是 opencode，看到本文件等于已加载 `skill/SKILL.md` 的全部内容。
> 如果你是 Claude Code / Copilot / Cursor，看到本文件即获得同等知识。

### 第 1 步：START/CREATE，创建任务

```powershell
udf task create "任务描述" --workspace workspace-name --id task-id --primary AesWorld --yes

# 修缺陷的任务，分支会是 fix/task-id
udf task create "任务描述" --workspace workspace-name --id task-id --type fix --primary AesWorld --yes

# 专业模式：等价底层命令
udf task create "任务描述" --workspace workspace-name --id task-id --prompt "用户原始prompt" --primary AesWorld --yes
```

工具自动：
- 解析主插件 `.uplugin` 的依赖关系
- 在 `Hosts/W-{workspace}/T-{id}_Host/` 下建 worktree + Host
- 引擎自带依赖自动 enable，**项目内依赖**创建 Junction 链回主仓库（只读）
- 写入 `.udf-meta.json`（v3 schema，含 `workspace` / `task_uid` / `context`）

**分支名怎么来的**：默认 `<type>/<task-id>`，比如 `feature/prefab-save-bug`，**不带工程名**。
`--type` 取六个值之一，不给按 `feature`。给了 `--branch` 就整个用它，`--type` 被忽略。

分支名可以随便起，包括完全不含 task-id 的名字。工具在主插件仓库的
`branch.<分支名>.udftask` 里记着它属于哪个任务，所以 Host 目录丢了之后
`task cleanup` 照样找得回这个分支。这条记录在 `git branch -D` 时由 git 自己带走。

**关键参数**：
- `--id`：短英文 kebab-case
- `--type`：变更类型，决定分支名前缀。`feature`（默认）/ `fix` / `hotfix` / `refactor` / `docs` / `chore`
- `--prompt`：**必须完整保存用户原始需求**，后续 agent 都要读
- `--primary AesWorld,AesWorld_AI`：v2 多主插件
- `--workspace neon-dev`：多 UE 项目并行时必须指定
- `--override-dep PCG=project` / `=engine` / `=<path>`：解决引擎/项目同名冲突
- `--yes`：agent 自动化场景跳过确认

**任务引用**：
- 单 workspace：`task-id`
- 多 workspace：`workspace/task-id`
- 若同名 task 存在于多个 workspace，短 id 必须报错，不允许猜测。

### 第 2 步：WORK，在 worktree 中工作

```
任务路径：{hosts_root}/W-{workspace}/T-{id}_Host/Plugins/<plugin-name>/Source/...
元信息：  {hosts_root}/W-{workspace}/T-{id}_Host/.udf-meta.json
```

**所有代码操作都在 worktree 路径下**：
- ✅ 读、改 worktree 中的代码
- ✅ 在 worktree 里 `git add` + `git commit`（commit message 必须中文 + 反思格式）
- ❌ **不要动主仓库**（`{plugins_root}/<plugin-name>/`）
- ❌ **不要动其他任务的 worktree**
- ❌ **不要动 DEV 项目**（Junction 切换由 switch 命令管）

### 第 3 步：BUILD，编译验证

```powershell
udf build task <task-ref>                          # 前台编译（编全 Host .uproject）
udf build task <task-ref> --background             # 后台编译
udf build task <task-ref> --primary-only           # 只编主插件模块（增量）
udf build task <task-ref> --profile medium|heavy   # 加严严格度（PR/merge 前）
udf build task <task-ref> --mutex wait|no-mutex     # 强制 mutex 模式
udf build task <task-ref> --validator              # 标记为 CI/Validator 调用
udf build status [task-ref]                   # 查状态，省略时按最近任务
```

**编之前先问一句。** 同一个引擎目录只有一把 UnrealBuildTool 互斥锁，两个编译同时开工会互相
破坏中间产物。`build check` 只回答能不能编，永远不启动编译，命令本身永远返回 0：

```powershell
udf build check <task-ref>                    # 四种结论之一
udf build check <task-ref> --format json      # 同样的结论，机器可读
udf build check --workspace <name>            # 不给任务时检查主项目
udf build gate "<完整命令>"                    # 这条命令绕过受控构建了吗，拦下时退出码 1
udf build project [--workspace <name>]        # 编主项目而不是任务宿主
```

| 结论 | 意思 | 该怎么办 |
|---|---|---|
| `ready` | 可以开始编译 | 直接 `build` |
| `deferred` | 另一个 UBT 正拿着锁 | 等它结束，**不要**改用 `--mutex no-mutex` 绕过去 |
| `blocked` | 被策略拒绝，比如命令里带 `-NoMutex` | 看 `reason` 字段 |
| `needsUserInput` | 引擎或项目解析不出来 | 补 `--workspace`，或修 workspace 配置 |

引擎路径解析顺序：workspace 的 `engine_path` → 环境变量 `UNREALDEVFLOW_UE_ENGINE_ROOT`
→ `.uproject` 里的 `EngineAssociation`。

工具自动用严格模式（Task#006 修复 + Task#009 profile 化）：
- **light（默认）**：` -FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity`
- **medium**：light + `-WarningsAsErrors`
- **heavy**：完整重建 + 强制 UHT 重生成

**Mutex 三态**（`--mutex auto|wait|no-mutex`）：auto 模式根据 engine 中间产物是否就绪自动选择，validator hint 会偏向 no-mutex。

**遇到失败**：先看 `Build_<profile>_<时间>.log`（默认在 `<host>/Logs/UBT/`）；依赖路径 dirty 会警告但**不阻塞**（依赖按 v2 设计是只读的）。

### 第 4 步：SWITCH，用户提到就执行

用户表达对当前任务执行 `switch` 的意图，就视为已经授权。agent 直接运行命令，不再复述命令或询问确认：

```
udf task switch <task-ref>
```

命令会清 UBT 缓存，并把多组 Junction 切到 Host 下的主插件和依赖插件。纯粹询问命令含义、引用示例或讨论别的任务不触发执行。编辑器仍在运行时，CLI 会警告切换在下次启动后生效，但不会再询问一次。遇到“目录被占用”时，让用户关闭 Rider、VSCode 或资源管理器后重试。

### 第 5 步：MERGE → CLEANUP（用户确认后才执行）

**⚠️ merge 命令的 `--strategy` 是必填参数，不询问用户就报错。**

**⚠️ IMPORTANT: `udf task merge` automatically fetches from origin before merging.**

This ensures you're merging against the latest remote state. If other tasks have been merged to dev while you were working, the merge will include those changes.

```powershell
# 1) 询问用户选择策略
"请选择合并策略：1.rebase 2.merge 3.squash 4.ff-only"
# 2) 执行 merge（仅合并，不删任何东西！）
udf task merge <task-ref> --strategy rebase
# 3) 展示 git log 供用户检查
git log --oneline -5
# 4) ⚠️ 等用户明确确认后，再 cleanup
udf task cleanup <task-ref>
```

**多主插件任务**（v2）：
```powershell
udf task merge <task-ref> --plugin AesWorld --strategy rebase    # 单插件
udf task merge <task-ref> --all --strategy rebase               # 全部逆序
```

---

## 🛑 严格禁止行为（违者任务失败）

| ❌ 禁止 | ✅ 改用 |
|---|---|
| `git merge` / `git rebase` / `git cherry-pick` | `udf task merge <task-ref> --strategy <s>` |
| `git branch -D` / `git worktree remove` / `git reset --hard` | `udf task cleanup <task-ref>` / `udf task delete <task-ref>` |
| 自动调 cleanup（merge 后未确认就删） | merge 后等用户确认才 cleanup |
| 不询问就指定 `--strategy rebase` | 询问 4 选 1，让用户选 |
| 修改主仓库（`{plugins_root}/<plugin-name>/`） | 只在 worktree 路径下改 |
| 自己跑 `Build.bat` / `RunUBT.bat` | `udf build task <task-ref>` |

---

## 📜 命令速查

顶层有六个组：`workspace`、`task`、`build`、`run`、`package`、`skill`。

| 命令 | 必须询问用户 | 用途 |
|---|---|---|
| `workspace init` | 无需询问 | 首次初始化 workspace + 安装 skill + doctor |
| `workspace add` | 无需询问 | 显式登记一个 UE 项目环境（`--plugins-root` 是关键） |
| `workspace list` / `doctor` / `remove` | 无需询问 | 管理多个 UE 项目环境 |
| `workspace status` | 无需询问 | 每个 UE 项目当前挂着哪个任务 |
| `task create <desc> --workspace <w> --id <id> --primary <P> --yes` | 无需询问 | 创建任务；不给 `--prompt` 时用描述当原始需求 |
| `task list` | 无需询问 | 列出所有任务，含损坏任务 |
| `task next [task-ref]` | 无需询问 | 根据状态告诉用户下一步 |
| `task switch <task-ref>` | 用户提出后不再询问 | 切 Junction（多 Junction 自动） |
| `task merge <task-ref> --strategy <s>` | **✅ 策略必问** | 合并（多主插件加 `--plugin` 或 `--all`） |
| `task finish [task-ref]` | **✅ 策略必问** | 验收通过后的合并向导 |
| `task cleanup <task-ref>` | **✅ 清理前确认** | 合并后用户确认才执行 |
| `task delete <task-ref>` | 无需询问 | 验收不通过时删除 |
| `build task <task-ref>` | 无需询问 | 编译（严格模式默认，可加 `--background` / `--primary-only`） |
| `build project` | 无需询问 | 编主项目而不是任务宿主 |
| `build check [task-ref]` | 无需询问 | 只回答现在能不能编，不启动编译 |
| `build gate "<命令>"` | 无需询问 | 检查某条命令有没有绕过受控构建 |
| `build status [task-ref]` | 无需询问 | 查编译状态，省略时按最近任务 |
| `run list/configure/check/plan/start` | 无需询问 | 复用已登记的 UE 原生 Editor、Commandlet、Gauntlet 入口 |
| `run status [execution-id]` | 无需询问 | 查原生日志、报告、UAT/UE 退出码和有限结果 |
| `run compare <before> <after>` | 无需询问 | 对照两次执行；可选 `--expect pass-after-fail` |
| `skill install` / `list` / `remove` | 无需询问 | 管理装到四个 AI provider 的 skill |

完整版（参数、返回值、错误码）见后文「命令速查」章节。

---

## 🧠 核心架构（一图流）

```
{plugins_root}/                  ← 永远不动的主仓库们
├─ AesWorld/         (独立 git)
├─ AesWorld_AI/      (独立 git)
└─ EarthPCG/         (独立 git)
       ↓ git worktree
{hosts_root}/W-{workspace}/T-{id}_Host/  ← 任务隔离工作空间
├─ T-{id}_Host.uproject                  ← 动态生成，enable 全插件
├─ .udf-meta.json                        ← v3 schema，记录 primary/dependency/context
├─ Logs/Build_<time>.log
└─ Plugins/
   ├─ AesWorld/      (worktree + branch task-{id}, 可写)    ← primary
   ├─ AesWorld_AI/   (worktree + branch task-{id}, 可写)    ← primary
   └─ EarthPCG/      (Junction → 主仓库, 只读)             ← dependency
       ↓ udf task switch
{default_project}/Plugins/        ← UE DEV 项目
└─ AesWorld/        (Junction → W-{workspace}/T-{id}_Host/Plugins/AesWorld)
```

**主/依赖插件区分**（v2）：
- **primary**：可写 worktree+branch，参与 merge
- **dependency**：只读 Junction 直链主仓库，build/switch/merge 前会 dirty 检查（警告不阻塞）

---

## 📖 详细章节索引

下面进入完整规范。如果你只读上面的"5 步标准工作流"已经能开箱即用，下面是参考细节：

1. [完整职责清单](#核心职责)
2. [v2 多插件工作流](#v2-多插件工作流自-task007)
3. [Commit Message 格式](#commit-message-格式)
4. [详细命令速查](#命令速查-1)
5. [错误处理](#错误处理)
6. [最佳实践](#最佳实践)
7. [配置参考](#配置参考)
8. [故障排查](#故障排查)

---

## 核心职责

作为 AI agent，你的职责是：
1. **理解用户意图**：从用户描述中提取任务需求
2. **创建隔离环境**：使用 `udf task create` 创建独立 worktree
3. **在 worktree 中工作**：所有代码修改必须在 worktree 路径下
4. **编译验证**：使用 `udf build task` 编译
5. **通知用户验收**：告诉用户如何切换和验收
6. **合并清理**：用户确认后执行 merge 和 cleanup

## ⚠️ 严格禁止行为

**以下行为绝对禁止，违反将导致任务失败：**

### ❌ 禁止直接运行 raw git 命令

**禁止**：
- ❌ `git merge`
- ❌ `git rebase`
- ❌ `git branch -D`
- ❌ `git worktree remove`
- ❌ `git cherry-pick`
- ❌ `git reset --hard`
- ❌ `git commit`（除非在 worktree 中提交代码修改）

**必须使用**：
- ✅ `udf task merge <task-id> --strategy <策略>`
- ✅ `udf task cleanup <task-id>`
- ✅ `udf task delete <task-id>`

### ❌ 禁止 merge 后自动删除分支

**错误流程**：
```powershell
# ❌ 绝对禁止！
git rebase dev
git branch -D task-xxx
git worktree remove ...
```

**正确流程**：
```powershell
# ✅ 必须使用工具
udf task merge <task-id> --strategy rebase  # 合并（保留分支）
git log --oneline -5                              # 用户检查
udf task cleanup <task-id>                   # 用户确认后清理
```

### ❌ 禁止跳过策略询问

**错误**：
```powershell
# ❌ 直接指定策略，不询问用户
udf task merge task-xxx --strategy rebase
```

**正确**：
```
请选择合并策略：
1. rebase（推荐，线性历史）
2. merge（保留任务历史）
3. squash（压缩成一个提交）
4. ff-only（仅快进）

请输入策略名称或编号：
```

## ⚠️ 严格禁止行为

**以下行为绝对禁止，违反将导致任务失败：**

### ❌ 禁止直接运行 raw git 命令

**禁止**：
- ❌ `git merge`
- ❌ `git rebase`
- ❌ `git branch -D`
- ❌ `git worktree remove`
- ❌ `git cherry-pick`
- ❌ `git reset --hard`
- ❌ `git commit`（除非在 worktree 中提交代码修改）

**必须使用**：
- ✅ `udf task merge <task-id> --strategy <策略>`
- ✅ `udf task cleanup <task-id>`
- ✅ `udf task delete <task-id>`

### ❌ 禁止 merge 后自动删除分支

**错误流程**：
```powershell
# ❌ 绝对禁止！
git rebase dev
git branch -D task-xxx
git worktree remove ...
```

**正确流程**：
```powershell
# ✅ 必须使用工具
udf task merge <task-id> --strategy rebase  # 合并（保留分支）
git log --oneline -5                              # 用户检查
udf task cleanup <task-id>                   # 用户确认后清理
```

### ❌ 禁止跳过策略询问

**错误**：
```powershell
# ❌ 直接指定策略，不询问用户
udf task merge task-xxx --strategy rebase
```

**正确**：
```
请选择合并策略：
1. rebase（推荐，线性历史）
2. merge（保留任务历史）
3. squash（压缩成一个提交）
4. ff-only（仅快进）

请输入策略名称或编号：
```

## 工作流

### 1. 接收任务

用户说："调查 EarthPrefabActor 保存后 Component 丢失的问题"

**你的行动**：
```powershell
# 创建任务，保存用户原始意图
udf task create "EarthPrefabActor 保存后 Component 丢失问题调查" `
    --workspace neon-dev `
    --id prefab-save-bug `
    --primary AesWorld `
    --yes
```

**关键点**：
- `--id` 使用简短的英文标识符（如 `prefab-save-bug`）
- `--prompt` 必须完整保存用户的原始需求描述
- `--yes` 跳过确认（agent 自动化场景）

### 2. 读取任务元信息

```powershell
# 从 config.toml 获取 hosts_root
$config = Get-Content "$env:USERPROFILE\.unrealdevflow\config.toml" | ConvertFrom-StringData
$hostsRoot = $config.hosts_root.Trim('"')

# 读取任务元信息
Get-Content "$hostsRoot\T-prefab-save-bug_Host\.udf-meta.json"
```

**元信息包含**：
- `id`: 任务 ID
- `name`: 任务名称
- `branch`: Git 分支名
- `prompt`: 用户原始需求
- `based_on`: 基于的 commit hash
- `status`: 任务状态

### 3. 在 worktree 中工作

**所有代码操作必须在 worktree 路径下进行**：

```
F:\ShanghaiP4\neon\Hosts\W-neon-dev\T-prefab-save-bug_Host\Plugins\AesWorld\Source\
```

**禁止**：
- ❌ 修改主仓库 `F:\ShanghaiP4\neon\Plugins\AesWorld`
- ❌ 修改其他任务的 worktree
- ❌ 直接操作 DEV 项目的 Plugins 目录

**允许**：
- ✅ 读取 worktree 中的代码
- ✅ 修改 worktree 中的代码
- ✅ 在 worktree 中执行 git 命令

### 4. 编译验证

```powershell
# 前台编译
udf build task neon-dev/prefab-save-bug

# 后台编译（不阻塞）
udf build task neon-dev/prefab-save-bug --background

# 查看编译状态
udf build status neon-dev/prefab-save-bug
```

**编译产物位置**：
```
F:\ShanghaiP4\neon\Hosts\W-neon-dev\T-prefab-save-bug_Host\Plugins\AesWorld\Binaries\Win64\
```

### 5. 通知用户验收

编译成功后，告诉用户：

```
✅ 任务 prefab-save-bug 已完成

验收步骤：
1. 运行：udf task switch neon-dev/prefab-save-bug
2. 重启 UE Editor
3. 验证功能是否正常

验收通过后：
  udf task merge neon-dev/prefab-save-bug --strategy rebase
  udf task cleanup neon-dev/prefab-save-bug

验收不通过：
  udf task delete neon-dev/prefab-save-bug --yes --force
```

### 6. 合并清理

用户确认验收通过后：

**⚠️ 重要：merge 命令只合并，永不删除！删除必须用 cleanup 命令！**

```powershell
# 第一步：向用户展示策略选项，等待用户选择
# 第二步：使用用户选择的策略执行 merge（只合并，不删除）
udf task merge <task-ref> --strategy <用户选择的策略>

# 第三步：检查 merge 结果
git log --oneline -5

# 第四步：⚠️ 必须等待用户明确确认后，才能执行 cleanup
udf task cleanup <task-ref>
```

**禁止行为**：
- ❌ 在 merge 后自动调用 cleanup
- ❌ 假设用户想要立即清理
- ❌ 使用 `git branch -D` 或 `git worktree remove` 直接删除
- ❌ 跳过 cleanup 的用户确认步骤

**正确流程**：
1. 用户确认验收通过
2. **Agent 询问用户选择合并策略**
3. 用户选择策略
4. Agent 执行 `udf task merge <task-ref> --strategy <用户选择>`
5. 展示 merge 结果
6. **Agent 询问用户是否清理 worktree 和分支**
7. 用户确认清理
8. Agent 执行 `udf task cleanup <task-ref>`

**merge 命令保证**：
- ✅ 只执行合并操作
- ✅ 保留 worktree 目录
- ✅ 保留 Git 分支
- ✅ 保留 Host 目录
- ❌ 永不删除任何内容

**cleanup 命令**：
- 必须用户明确确认后才能执行
- 删除 worktree、分支、Host 目录
- 不可逆操作

## 命令速查

| 命令 | 说明 | Agent 使用场景 | **必须询问用户** |
|---|---|---|---|
| `init` | 小白首次初始化 workspace | 用户首次使用时引导 | ❌ |
| `task create` | 创建任务 | 接收用户任务时 | ❌ |
| `next` | 提示下一步 | 用户不知道下一步时 | ❌ |
| `finish` | 合并向导 | 用户确认验收通过后 | **✅ 必须询问策略** |
| `workspace` | 管理多个 UE 项目环境 | 多项目并行/doctor | ❌ |
| `workspace add` | 显式登记环境 | 脚本化配置 | ❌ |
| `create` | 专业创建任务 | agent 需要显式 prompt/override 时 | ❌ |
| `build` | 编译任务 | 代码修改完成后 | ❌ |
| `build status` | 查看编译状态 | 后台编译后检查 | ❌ |
| `switch` | 切换 Junction | 用户表达当前任务的切换意图后直接执行 | ❌ |
| `list` | 列出任务 | 用户询问有哪些任务时 | ❌ |
| `status` | 查看状态 | 用户询问当前状态时 | ❌ |
| `merge` | 合并任务 | 用户确认验收通过后 | **✅ 必须询问策略** |
| `cleanup` | 清理 worktree | merge 后用户确认清理时 | **✅ 必须用户确认** |
| `delete` | 删除任务 | 用户验收不通过时 | ❌ |

## v2 多插件工作流（自 Task#007）

UnrealDevFlow 已支持一个任务跨多个插件协同开发：

- **主插件 (primary)**：可写，独立 Git worktree+branch，参与 merge
- **依赖插件 (dependency)**：只读，Junction 直链主仓库，跟随主仓库 dev

### 创建多插件任务

```powershell
# 多个主插件
udf task create "调查 EarthPrefab 保存丢失" `
    --workspace neon-dev `
    --id prefab-bug `
    --primary AesWorld,AesWorld_AI `
    --yes

# 当依赖在引擎和项目同时存在时，必须用 --override-dep 解决冲突
udf task create "..." --workspace neon-dev --id xxx `
    --primary AesWorld `
    --override-dep PCG=project `
    --override-dep GeometryProcessing=engine
```

依赖自动扫描流程：
1. 解析每个主插件的 `.uplugin` 的 `Plugins` 数组
2. 对每个依赖在 `<engine>/Engine/Plugins` 和 `plugins_root` 两处查找
3. 引擎自带 → 自动 enable，不创建 Junction
4. 项目内 → 在 Host 下创建 Junction 链回主仓库
5. 引擎+项目同时有 → 必须 `--override-dep <name>=engine|project` 选边
6. 都没有 → 必须 `--override-dep <name>=<absolute-path>` 提供

### 多插件编译

`build` 编译整个 Host `.uproject`，所有主插件 + 项目依赖 + 引擎依赖一起编。
依赖插件 dirty 时只警告不阻塞（依赖按 v2 设计是只读的）。

```powershell
udf build task neon-dev/prefab-bug                # 全编（默认，跨插件依赖能 100% 暴露）
udf build task neon-dev/prefab-bug --primary-only # 只编主插件模块（增量验证）
```

### 多插件 merge

每个主插件都是独立 Git 仓库，必须明确指定操作目标：

```powershell
# 指定单个主插件
udf task merge neon-dev/prefab-bug --plugin AesWorld --strategy rebase

# 全部主插件按逆序逐个 merge（每个独立确认）
udf task merge neon-dev/prefab-bug --all --strategy rebase
```

**依赖插件永远不参与 merge**。如果依赖代码确实改了，用户应该单独提交到主仓库。

### v1 任务自动迁移

工具读旧版 `.udf-meta.json` 时会自动升级为 v2，同时备份原文件为
`.udf-meta.json.v1.bak`。无需手动操作。

## Commit Message 格式

**所有 git commit 必须使用中文 message**，格式如下：

```
Task#[number] [内容摘要]

修改内容：
- [具体修改 1]
- [具体修改 2]
- [具体修改 3]

过程反思：
- [偏差/教训/范式总结，1-3 条]

后续注意：
- [避免再犯的点，1-3 条]
```

**示例**：
```
Task#001 添加建筑轮廓线拍平功能

修改内容：
- 新增 FlattenContour 右键菜单命令
- 实现按平均高度拍平建筑轮廓线
- 添加命令可见性判断逻辑

过程反思：
- 初始方案用 git rebase 方向反了，应该用 cherry-pick 三步法
- 任务分支的提交应该放在 dev 历史的最上面，而不是中间

后续注意：
- rebase 策略必须：reset 到 based_on → cherry-pick dev 新提交 → cherry-pick 任务提交
- 合并前检查分支分歧，确保提交顺序正确
```

**规则**：
- ✅ **必须中文**：subject 和 body 都用中文
- ✅ **简洁具体**：直接描述做了什么，不要废话
- ✅ **必须包含反思**：记录偏差、教训、范式总结
- ✅ **后续注意**：提炼可复用的开发范式或需要避免的错误
- ❌ **不要 Co-authored-by**：禁止包含协作作者信息

## 错误处理

### 常见错误及解决方案

| 错误信息 | 原因 | 解决方案 |
|---|---|---|
| "目录被占用" | Rider/VSCode 锁定了目录 | 提示用户关闭 IDE 后重试 |
| "不是 Git 仓库" | 路径错误 | 检查 plugins_root / workspace 配置 |
| "Build.bat 不存在" | 引擎路径错误 | 检查 engine_path 配置 |
| "合并冲突" | 代码冲突 | 提示用户手动解决冲突 |
| "任务已存在" | ID 冲突 | 使用不同的 --id |

### 错误恢复

**场景**：merge 后发现提交顺序错了

**解决方案**：
```powershell
# 1. 找到 merge 前的 commit
git reflog

# 2. 回退到 merge 前
git reset --hard <merge-before-commit>

# 3. 重新执行 merge
udf task merge <task-id> --strategy rebase
```

## 最佳实践

### 1. 任务 ID 命名

- ✅ 使用简短英文：`prefab-save-bug`、`flatten-contour`
- ✅ 使用连字符分隔：`ai-nav-system`
- ❌ 避免中文：`保存 bug`
- ❌ 避免过长：`earth-prefab-actor-save-component-loss-investigation`

### 2. Prompt 保存

- ✅ 完整保存用户原始需求
- ✅ 包含上下文信息
- ❌ 不要简化或改写用户意图

### 3. 编译策略

- 小改动：前台编译（快速反馈）
- 大改动：后台编译（不阻塞工作）
- 多任务：使用 `--no-mutex` 并行编译

### 4. 合并策略

- **默认使用 rebase**：保持线性历史
- 需要保留任务历史：使用 merge
- 任务提交很零散：使用 squash
- 严格线性工作流：使用 ff-only

### 5. 清理时机

- merge 后**不要立即清理**
- 提示用户检查 merge 结果
- 用户确认后再执行 cleanup
- 不提供一步清理捷径；merge 后先检查结果，再由用户确认 cleanup

## 配置参考

```toml
# ~/.unrealdevflow/config.toml
hosts_root = "F:\\ShanghaiP4\\neon\\Hosts"
plugins_root = "F:\\ShanghaiP4\\neon\\Plugins"
default_project = "F:\\ShanghaiP4\\neon\\UGA\\DEV"
engine_path = "D:\\Unreal Engine\\UE_5.5"

[workspaces.neon-dev]
hosts_root = "F:\\ShanghaiP4\\neon\\Hosts"
plugins_root = "F:\\ShanghaiP4\\neon\\Plugins"
default_project = "F:\\ShanghaiP4\\neon\\UGA\\DEV"
engine_path = "D:\\Unreal Engine\\UE_5.5"
```

## 目录结构

```
F:\ShanghaiP4\neon\
├── Plugins\
│   └── AesWorld\                    ← 主仓库（永远不动）
│
├── Hosts\
│   └── W-neon-dev\
│       ├── T-prefab-save-bug_Host\      ← 任务 worktree
│       │   ├── T-prefab-save-bug_Host.uproject
│       │   ├── .udf-meta.json           ← 含 workspace/task_uid/context
│       │   ├── Logs\                    ← 编译日志
│       │   └── Plugins\
│       │       └── AesWorld\            ← Git worktree
│       │
│       └── T-flatten-contour_Host\      ← 同 workspace 的另一个任务
│           └── ...
│
└── UGA\DEV\
    └── Plugins\
        └── AesWorld\ → Junction → Hosts\W-neon-dev\T-xxx_Host\Plugins\AesWorld
```

## 注意事项

1. **用户提到当前任务的 switch 就执行**：该消息已经授权，agent 不再要求第二次确认
2. **不要碰主仓库**：所有代码操作在 worktree 路径下
3. **编译用 build 命令**：不要自己调 Build.bat
4. **完成后通知用户**：告诉用户怎么验收
5. **merge 后保留 worktree**：让用户检查后再 cleanup

## 故障排查

### 问题：switch 失败，提示"目录被占用"

**原因**：Rider、VSCode 或文件资源管理器锁定了目录

**解决**：
1. 关闭占用目录的程序
2. 运行：`udf task switch <task-id> --force`

### 问题：编译失败，提示"Build.bat 不存在"

**原因**：engine_path 配置错误

**解决**：
1. 检查配置：`Get-Content ~/.unrealdevflow/config.toml`
2. 重新配置：`udf workspace add --engine-path "正确路径"`

### 问题：merge 后想回退

**解决**：
```powershell
git reflog  # 找到 merge 前的 commit
git reset --hard <commit>
```

### 问题：switch 时遇到"Broken junction"警告

**现象**：
```
⚠ Found broken junction at F:\...\UGA\DEV\Plugins\AesWorld (target no longer exists)
Removing broken junction...
```

**原因**：之前的任务被删除时，主项目中的 Junction 没有被清理，指向了已不存在的 worktree 目录。

**解决**：
- **自动修复**：`switch` 命令会自动检测并删除断开的 Junction，然后创建新的 Junction。无需手动操作。
- **预防措施**：始终使用 `udf task delete <task-id>` 删除任务，而不是手动删除 worktree 目录。`delete` 命令会自动清理所有指向该任务 worktree 的 Junction。

**Junction 生命周期说明**：
1. **创建时**：`switch <task-id>` 在主项目的 `Plugins/` 目录下创建 Junction，指向任务的 worktree
2. **删除时**：`delete <task-id>` 会扫描 `~/.unrealdevflow/state.json` 中所有已知项目，找到并删除指向该任务 worktree 的 Junction
3. **切换时**：`switch` 如果遇到断开的 Junction（目标已删除），会自动清理并重新创建

**最佳实践**：永远使用 `udf task delete` 而不是手动删除目录，这样可以确保 Junction 被正确清理。

## 版本信息

- UnrealDevFlow v0.1.1
- 最后更新：2026-06-12
