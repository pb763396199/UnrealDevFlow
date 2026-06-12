# UnrealDevFlow Agent 指南

本文档面向 AI agent（如 Codex、Copilot、Claude Code、Cursor、opencode），说明如何使用 UnrealDevFlow 工具协助用户完成 UE 插件开发任务。

---

## 🚀 5 步标准工作流（任何 AI agent 必读）

> **这一节是给 agent 看的速查版本**。完整规范、详细解释、FAQ 在本文后半部分。
> 如果你是 opencode，看到本文件等于已加载 `skill/SKILL.md` 的全部内容。
> 如果你是 Claude Code / Copilot / Cursor，看到本文件即获得同等知识。

### 第 1 步：CREATE — 创建任务

```powershell
unrealdevflow create "任务描述" --id task-id --prompt "用户原始prompt" --yes
```

工具自动：
- 解析主插件 `.uplugin` 的依赖关系
- 在 `Hosts/T-{id}_Host/` 下建 worktree + Host
- 引擎自带依赖自动 enable，**项目内依赖**创建 Junction 链回主仓库（只读）
- 写入 `.udf-meta.json`（v2 schema）

**关键参数**：
- `--id`：短英文 kebab-case
- `--prompt`：**必须完整保存用户原始需求**，后续 agent 都要读
- `--primary AesWorld,AesWorld_AI`：v2 多主插件
- `--override-dep PCG=project` / `=engine` / `=<path>`：解决引擎/项目同名冲突
- `--yes`：agent 自动化场景跳过确认

### 第 2 步：WORK — 在 worktree 中工作

```
任务路径：{hosts_root}/T-{id}_Host/Plugins/<plugin-name>/Source/...
元信息：  {hosts_root}/T-{id}_Host/.udf-meta.json
```

**所有代码操作都在 worktree 路径下**：
- ✅ 读、改 worktree 中的代码
- ✅ 在 worktree 里 `git add` + `git commit`（commit message 必须中文 + 反思格式）
- ❌ **不要动主仓库**（`{plugins_root}/<plugin-name>/`）
- ❌ **不要动其他任务的 worktree**
- ❌ **不要动 DEV 项目**（Junction 切换由 switch 命令管）

### 第 3 步：BUILD — 编译验证

```powershell
unrealdevflow build <id>                          # 前台编译（编全 Host .uproject）
unrealdevflow build <id> --background             # 后台编译
unrealdevflow build <id> --primary-only           # 只编主插件模块（增量）
unrealdevflow build <id> --profile medium|heavy   # 加严严格度（PR/merge 前）
unrealdevflow build <id> --mutex wait|nomutex     # 强制 mutex 模式
unrealdevflow build <id> --validator              # 标记为 CI/Validator 调用
unrealdevflow build-status <id>                   # 查状态
```

工具自动用严格模式（Task#006 修复 + Task#009 profile 化）：
- **light（默认）**：` -FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity`
- **medium**：light + `-WarningsAsErrors`
- **heavy**：完整重建 + 强制 UHT 重生成

**Mutex 三态**（`--mutex auto|wait|nomutex`）：auto 模式根据 engine 中间产物是否就绪自动选择，validator hint 会偏向 no-mutex。

**遇到失败**：先看 `Build_<profile>_<时间>.log`（默认在 `<host>/Logs/UBT/`）；依赖路径 dirty 会警告但**不阻塞**（依赖按 v2 设计是只读的）。

### 第 4 步：SWITCH — 通知用户验收

**这一步 agent 不要自己执行**！告诉用户：

```
✅ 任务 <id> 已完成，请验收：
1. unrealdevflow switch <id>     ← 用户运行
2. 重启 UE Editor
3. 验证功能
```

switch 会：清 UBT 缓存 → 多 Junction 切到 Host 下所有主+依赖插件。
遇到"目录被占用"让用户关 Rider/VSCode 后重试或加 `--force`。

### 第 5 步：MERGE → CLEANUP（用户确认后才执行）

**⚠️ merge 命令的 `--strategy` 是必填参数，不询问用户就报错。**

**⚠️ IMPORTANT: `unrealdevflow merge` automatically fetches from origin before merging.**

This ensures you're merging against the latest remote state. If other tasks have been merged to dev while you were working, the merge will include those changes.

```powershell
# 1) 询问用户选择策略
"请选择合并策略：1.rebase 2.merge 3.squash 4.ff-only"
# 2) 执行 merge（仅合并，不删任何东西！）
unrealdevflow merge <id> --strategy rebase
# 3) 展示 git log 供用户检查
git log --oneline -5
# 4) ⚠️ 等用户明确确认后，再 cleanup
unrealdevflow cleanup <id>
```

**多主插件任务**（v2）：
```powershell
unrealdevflow merge <id> --plugin AesWorld --strategy rebase    # 单插件
unrealdevflow merge <id> --all --strategy rebase               # 全部逆序
```

---

## 🛑 严格禁止行为（违者任务失败）

| ❌ 禁止 | ✅ 改用 |
|---|---|
| `git merge` / `git rebase` / `git cherry-pick` | `unrealdevflow merge <id> --strategy <s>` |
| `git branch -D` / `git worktree remove` / `git reset --hard` | `unrealdevflow cleanup <id>` / `unrealdevflow delete <id>` |
| 自动调 cleanup（merge 后未确认就删） | merge 后等用户确认才 cleanup |
| 不询问就指定 `--strategy rebase` | 询问 4 选 1，让用户选 |
| 修改主仓库（`{plugins_root}/<plugin-name>/`） | 只在 worktree 路径下改 |
| 自己跑 `Build.bat` / `RunUBT.bat` | `unrealdevflow build <id>` |

---

## 📜 命令速查

| 命令 | 必须询问用户 | 用途 |
|---|---|---|
| `configure` | — | 首次配置（`--plugins-root` 是 v2 关键） |
| `create <desc> --id <id> --prompt <p> --yes` | — | 创建任务 |
| `build <id>` | — | 编译（严格模式默认） |
| `build <id> --background` | — | 后台编译 |
| `build <id> --primary-only` | — | 只编主插件模块 |
| `build-status <id>` | — | 查编译状态 |
| `switch <id>` | — | 切 Junction（多 Junction 自动） |
| `list` / `status` | — | 看任务/状态 |
| `merge <id> --strategy <s>` | **✅ 策略必问** | 合并（多主插件加 `--plugin` 或 `--all`） |
| `cleanup <id>` | — | 合并后用户确认才执行 |
| `delete <id>` | — | 验收不通过时删除 |

完整版（参数、返回值、错误码）见后文「命令速查」章节。

---

## 🧠 核心架构（一图流）

```
{plugins_root}/                  ← 永远不动的主仓库们
├─ AesWorld/         (独立 git)
├─ AesWorld_AI/      (独立 git)
└─ EarthPCG/         (独立 git)
       ↓ git worktree
{hosts_root}/T-{id}_Host/        ← 任务隔离工作空间
├─ T-{id}_Host.uproject          ← 动态生成，enable 全插件
├─ .udf-meta.json                ← v2 schema，记录 primary/dependency 列表
├─ Logs/Build_<time>.log
└─ Plugins/
   ├─ AesWorld/      (worktree + branch task-{id}, 可写)    ← primary
   ├─ AesWorld_AI/   (worktree + branch task-{id}, 可写)    ← primary
   └─ EarthPCG/      (Junction → 主仓库, 只读)             ← dependency
       ↓ unrealdevflow switch
{default_project}/Plugins/        ← UE DEV 项目
└─ AesWorld/        (Junction → T-{id}_Host/Plugins/AesWorld)
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
2. **创建隔离环境**：使用 `unrealdevflow create` 创建独立 worktree
3. **在 worktree 中工作**：所有代码修改必须在 worktree 路径下
4. **编译验证**：使用 `unrealdevflow build` 编译
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
- ✅ `unrealdevflow merge <task-id> --strategy <策略>`
- ✅ `unrealdevflow cleanup <task-id>`
- ✅ `unrealdevflow delete <task-id>`

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
unrealdevflow merge <task-id> --strategy rebase  # 合并（保留分支）
git log --oneline -5                              # 用户检查
unrealdevflow cleanup <task-id>                   # 用户确认后清理
```

### ❌ 禁止跳过策略询问

**错误**：
```powershell
# ❌ 直接指定策略，不询问用户
unrealdevflow merge task-xxx --strategy rebase
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
- ✅ `unrealdevflow merge <task-id> --strategy <策略>`
- ✅ `unrealdevflow cleanup <task-id>`
- ✅ `unrealdevflow delete <task-id>`

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
unrealdevflow merge <task-id> --strategy rebase  # 合并（保留分支）
git log --oneline -5                              # 用户检查
unrealdevflow cleanup <task-id>                   # 用户确认后清理
```

### ❌ 禁止跳过策略询问

**错误**：
```powershell
# ❌ 直接指定策略，不询问用户
unrealdevflow merge task-xxx --strategy rebase
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
unrealdevflow create "EarthPrefabActor 保存后 Component 丢失问题调查" `
    --id prefab-save-bug `
    --prompt "调查 EarthPrefabActor 在关卡中保存后重新打开时 Component 丢失的问题，怀疑是 EarthSplineComponent 或 EarthDataBase 的同步逻辑导致" `
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
F:\ShanghaiP4\neon\Hosts\T-prefab-save-bug_Host\Plugins\AesWorld\Source\
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
unrealdevflow build prefab-save-bug

# 后台编译（不阻塞）
unrealdevflow build prefab-save-bug --background

# 查看编译状态
unrealdevflow build-status prefab-save-bug
```

**编译产物位置**：
```
F:\ShanghaiP4\neon\Hosts\T-prefab-save-bug_Host\Plugins\AesWorld\Binaries\Win64\
```

### 5. 通知用户验收

编译成功后，告诉用户：

```
✅ 任务 prefab-save-bug 已完成

验收步骤：
1. 运行：unrealdevflow switch prefab-save-bug
2. 重启 UE Editor
3. 验证功能是否正常

验收通过后：
  unrealdevflow merge prefab-save-bug --strategy rebase
  unrealdevflow cleanup prefab-save-bug

验收不通过：
  unrealdevflow delete prefab-save-bug --yes --force
```

### 6. 合并清理

用户确认验收通过后：

**⚠️ 重要：merge 命令只合并，永不删除！删除必须用 cleanup 命令！**

```powershell
# 第一步：向用户展示策略选项，等待用户选择
# 第二步：使用用户选择的策略执行 merge（只合并，不删除）
unrealdevflow merge <task-id> --strategy <用户选择的策略>

# 第三步：检查 merge 结果
git log --oneline -5

# 第四步：⚠️ 必须等待用户明确确认后，才能执行 cleanup
unrealdevflow cleanup <task-id>
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
4. Agent 执行 `unrealdevflow merge <task-id> --strategy <用户选择>`
5. 展示 merge 结果
6. **Agent 询问用户是否清理 worktree 和分支**
7. 用户确认清理
8. Agent 执行 `unrealdevflow cleanup <task-id>`

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
| `configure` | 首次配置 | 用户首次使用时引导 | ❌ |
| `create` | 创建任务 | 接收用户任务时 | ❌ |
| `build` | 编译任务 | 代码修改完成后 | ❌ |
| `build-status` | 查看编译状态 | 后台编译后检查 | ❌ |
| `switch` | 切换 Junction | 通知用户验收时 | ❌ |
| `list` | 列出任务 | 用户询问有哪些任务时 | ❌ |
| `status` | 查看状态 | 用户询问当前状态时 | ❌ |
| `merge` | 合并任务 | 用户确认验收通过后 | **✅ 必须询问策略** |
| `cleanup` | 清理 worktree | merge 后用户确认清理时 | ❌ |
| `delete` | 删除任务 | 用户验收不通过时 | ❌ |

## v2 多插件工作流（自 Task#007）

UnrealDevFlow 已支持一个任务跨多个插件协同开发：

- **主插件 (primary)**：可写，独立 Git worktree+branch，参与 merge
- **依赖插件 (dependency)**：只读，Junction 直链主仓库，跟随主仓库 dev

### 创建多插件任务

```powershell
# 多个主插件
unrealdevflow create "调查 EarthPrefab 保存丢失" `
    --id prefab-bug `
    --primary AesWorld,AesWorld_AI `
    --yes

# 当依赖在引擎和项目同时存在时，必须用 --override-dep 解决冲突
unrealdevflow create "..." --id xxx `
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
unrealdevflow build prefab-bug                # 全编（默认，跨插件依赖能 100% 暴露）
unrealdevflow build prefab-bug --primary-only # 只编主插件模块（增量验证）
```

### 多插件 merge

每个主插件都是独立 Git 仓库，必须明确指定操作目标：

```powershell
# 指定单个主插件
unrealdevflow merge prefab-bug --plugin AesWorld --strategy rebase

# 全部主插件按逆序逐个 merge（每个独立确认）
unrealdevflow merge prefab-bug --all --strategy rebase
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
| "不是 Git 仓库" | 路径错误 | 检查 plugin_path 配置 |
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
unrealdevflow merge <task-id> --strategy rebase
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
- 或者使用 `--cleanup` 参数一步到位（有风险）

## 配置参考

```toml
# ~/.unrealdevflow/config.toml
hosts_root = "F:\\ShanghaiP4\\neon\\Hosts"
plugin_path = "F:\\ShanghaiP4\\neon\\Plugins\\AesWorld"
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
│   ├── T-prefab-save-bug_Host\      ← 任务 worktree
│   │   ├── T-prefab-save-bug_Host.uproject
│   │   ├── .udf-meta.json           ← 任务元信息
│   │   ├── Logs\                    ← 编译日志
│   │   └── Plugins\
│   │       └── AesWorld\            ← Git worktree
│   │
│   └── T-flatten-contour_Host\      ← 另一个任务
│       └── ...
│
└── UGA\DEV\
    └── Plugins\
        └── AesWorld\ → Junction → Hosts\T-xxx_Host\Plugins\AesWorld
```

## 注意事项

1. **不要自动 switch**：switch 需要用户手动执行（涉及 Editor 重启）
2. **不要碰主仓库**：所有代码操作在 worktree 路径下
3. **编译用 build 命令**：不要自己调 Build.bat
4. **完成后通知用户**：告诉用户怎么验收
5. **merge 后保留 worktree**：让用户检查后再 cleanup

## 故障排查

### 问题：switch 失败，提示"目录被占用"

**原因**：Rider、VSCode 或文件资源管理器锁定了目录

**解决**：
1. 关闭占用目录的程序
2. 运行：`unrealdevflow switch <task-id> --force`

### 问题：编译失败，提示"Build.bat 不存在"

**原因**：engine_path 配置错误

**解决**：
1. 检查配置：`Get-Content ~/.unrealdevflow/config.toml`
2. 重新配置：`unrealdevflow configure --engine-path "正确路径"`

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
- **预防措施**：始终使用 `unrealdevflow delete <task-id>` 删除任务，而不是手动删除 worktree 目录。`delete` 命令会自动清理所有指向该任务 worktree 的 Junction。

**Junction 生命周期说明**：
1. **创建时**：`switch <task-id>` 在主项目的 `Plugins/` 目录下创建 Junction，指向任务的 worktree
2. **删除时**：`delete <task-id>` 会扫描 `~/.unrealdevflow/state.json` 中所有已知项目，找到并删除指向该任务 worktree 的 Junction
3. **切换时**：`switch` 如果遇到断开的 Junction（目标已删除），会自动清理并重新创建

**最佳实践**：永远使用 `unrealdevflow delete` 而不是手动删除目录，这样可以确保 Junction 被正确清理。

## 版本信息

- UnrealDevFlow v0.1.0
- 最后更新：2026-06-08
