---
name: UnrealDevFlow
description: Unreal Engine 插件多任务并行开发工作流。当用户提出开发任务、bug调查、功能验证时，使用此skill自动创建隔离的工作空间并推进任务。触发词：开发、调查、验证、修复、实现、测试功能。
argument-hint: '描述你要做的任务'
---

# UnrealDevFlow — UE 插件并行开发工作流

## 跨 Agent 通用入口

> **opencode skill 加载入口**。如果你不是 opencode，请改读仓库根目录的 `AGENTS.md`：
> - **Claude Code**: `CLAUDE.md`（更精简版本）
> - **GitHub Copilot**: `.github/copilot-instructions.md`
> - **Cursor / Codex / 其他**: `AGENTS.md`
>
> 本 SKILL.md 与 AGENTS.md 顶部「5 步标准工作流」保持一致，下方是详细展开。

## 🚀 小白优先入口

优先使用这 4 个命令，让用户不用理解底层配置表：

```powershell
udf workspace init --project "<UE项目目录>" [--workspace <name>]
udf task create "用户原始需求" --workspace <name> --primary <Plugin> --id <task-id> --yes
udf task next <workspace>/<task-id>
udf task finish <workspace>/<task-id>
```

- `init` 自动探测 UE 项目、Plugins 根目录、Engine 路径、Hosts 目录，保存 workspace，并安装 AI skill。
- workspace 名称可以由工具建议，也可以用户自定义；内部会规范成 kebab-case。
- 多 workspace 时任务引用必须使用 `workspace/task-id`；短 id 歧义时必须报错，不能猜测。
- `task create` 不给 `--prompt` 时会把描述当原始需求写进任务元数据。
- `next` 只告诉用户当前最该做的一步。
- `finish` 是合并向导，仍必须让用户选择合并策略，默认推荐 rebase。

## 🚀 5 步标准工作流

| 步骤 | 命令 | 关键点 |
|---|---|---|
| 1. CREATE | `udf task create "<desc>" --workspace <w> --id <id> --primary <Plugin> --yes` | 想显式保存原始需求就加 `--prompt` |
| 2. WORK | 编辑 `{hosts_root}/W-<workspace>/T-<id>_Host/Plugins/<plugin>/Source/...` | 绝不动主仓库；commit 必须中文 + 反思 |
| 3. BUILD | `udf build task <workspace>/<id>` （严格模式自动启用） | 严格 flag: `-FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity` |
| 4. SWITCH | 告诉用户运行 `udf task switch <workspace>/<id>` + 重启 Editor | agent 不自己执行 switch |
| 5. MERGE → CLEANUP | `udf task finish <workspace>/<id>` 或 `merge ... --strategy <s>` → 用户确认后 `cleanup` | `--strategy` 必须由用户选择；merge 后不要立即 cleanup |

完整说明见后文「完整工作流」章节。

## 什么时候用

当用户提出以下类型的任务时触发：
- 开发新功能
- 调查 bug
- 验证想法/方案
- 修复问题
- 代码重构

## 核心概念

```
主仓库 (Plugins/AesWorld)     ← 用户日常开发，永远不动
    ↓ git worktree
任务 Host (Hosts/W-<workspace>/T-xxx_Host)  ← 隔离的工作空间，独立编译
    ↓ NTFS Junction
UE 项目 (DEV/Plugins/AesWorld) ← 验收时切换指向
```

## 工具位置

Skill 通过 `udf` 命令调用。确保工具已安装并在 PATH 中，或使用完整路径。

## 完整工作流

### 第一步：创建任务（保存用户原始意图）

小白入口：

```powershell
udf task create "任务描述" `
    --workspace workspace-name `
    --id task-id `
    --primary AesWorld `
    --yes
```

专业入口：

```powershell
udf task create "任务描述" `
    --workspace workspace-name `
    --id task-id `
    --prompt "用户的原始prompt，完整保存" `
    --primary AesWorld `
    --yes
```

**关键**：`--prompt` 必须保存用户的原始意图，后续 agent 需要读取它。

### 第二步：读取任务元信息

```powershell
# 优先直接读取任务元信息；多 workspace 路径如下
Get-Content "{hosts_root}\W-{workspace}\T-{task-id}_Host\.udf-meta.json"
```

获取 worktree 路径，后续所有文件操作都在这个路径下进行。

### 第三步：在 worktree 中工作

**所有代码修改必须在 worktree 路径下进行**：

```
{hosts_root}\W-{workspace}\T-{task-id}_Host\Plugins\AesWorld\Source\
```

- 读代码：从这个路径读
- 改代码：在这个路径改
- 不要碰主仓库 `{plugins_root}\AesWorld`

### 第四步：编译验证

```powershell
udf build task {workspace}/{task-id}
```

编译产物在 `W-{workspace}\T-{task-id}_Host\Plugins\AesWorld\Binaries\Win64\`

### 第五步：通知用户验收

告诉用户：
1. 任务已完成
2. 运行 `udf task switch {workspace}/{task-id}` 切换
3. 重启 UE Editor 验收

### 第六步：验收后处理

用户验收通过后：
```powershell
# 合并（不立即清理，保留 worktree 和分支供检查）
udf task merge {workspace}/{task-id} --strategy rebase

# 检查 merge 结果
git log --oneline -5

# 确认无误后清理
udf task cleanup {workspace}/{task-id}
```

用户验收不通过：
```powershell
udf task delete {workspace}/{task-id} --yes --force
```

## 关键规则

1. **创建任务时必须带 --prompt** — 保存用户原始意图
2. **所有代码操作在 worktree 路径下** — 不要碰主仓库
3. **不要自动 switch** — switch 需要用户手动执行（涉及 Editor 重启）
4. **编译用 build 命令** — 不要自己调 Build.bat
5. **完成后通知用户** — 告诉用户怎么验收
6. **Commit message 必须是中文** — 遵循下方格式要求，包含反思内容

## 提交信息格式要求

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

### 示例

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

### 规则

- **必须中文**：subject 和 body 都用中文
- **简洁具体**：直接描述做了什么，不要废话
- **必须包含反思**：记录偏差、教训、范式总结，避免以后犯错
- **后续注意**：提炼可复用的开发范式或需要避免的错误
- **不要 Co-authored-by**：禁止包含协作作者信息
- **可粘贴**：格式要适合直接粘贴到 git commit -m ""

### 反思要素（精简版）

从任务执行过程中提炼以下要点：

| 要素 | 说明 | 示例 |
|---|---|---|
| **偏差** | 实际执行与原计划的差异 | "初始方案用 git rebase 方向反了" |
| **教训** | 从错误中学到的东西 | "任务提交应该放在 dev 历史最上面" |
| **范式** | 可复用的开发模式 | "rebase 策略必须用三步法" |
| **避免** | 以后不要做的事 | "不要直接调用 git rebase <branch>" |

## 命令速查

| 命令 | 说明 |
|---|---|
| `workspace init --project <UE项目> [--workspace <name>]` | 初始化 workspace |
| `workspace add/list/doctor/remove/status` | 管理 UE 项目环境 |
| `task create "描述" --workspace <w> --id xxx --primary <Plugin> --yes` | 创建任务 |
| `task next <workspace/task>` | 告诉用户下一步 |
| `task finish <workspace/task>` | 验收通过后的合并向导 |
| `task list` | 列出任务 |
| `task switch <workspace/task> --force` | 切换 Junction |
| `task merge <workspace/task> --strategy <策略>` | 合并（保留 worktree 和分支） |
| `task cleanup <workspace/task>` | 手动清理 worktree 和分支 |
| `task delete <workspace/task> --yes --force` | 删除并清理（不合并） |
| `task merge <workspace/task> --dry-run` | 预览合并 |
| `task delete <workspace/task> --dry-run` | 预览删除 |
| `build task <workspace/task>` | 编译 |
| `build status <workspace/task>` | 查编译状态 |
| `build check [workspace/task]` | 只回答现在能不能编；`ready`/`deferred`/`blocked`/`needsUserInput` |
| `build gate "<完整命令>"` | 检查命令有没有绕过受控构建，拦下时退出码 1 |
| `build project [--workspace <w>]` | 编主项目而不是任务宿主 |
| `skill install/list/remove` | 管理 AI skill |

## 示例：完整任务流程

用户说："调查 EarthPrefabActor 保存后 Component 丢失的问题"

```powershell
# 1. 创建任务
udf task create "EarthPrefabActor保存后Component丢失问题调查" `
    --workspace neon-dev `
    --id prefab-save-bug `
    --primary AesWorld `
    --yes

# 2. 读取元信息
Get-Content "{hosts_root}\W-neon-dev\T-prefab-save-bug_Host\.udf-meta.json"

# 3. 在 worktree 中调查代码
# 读：{hosts_root}\W-neon-dev\T-prefab-save-bug_Host\Plugins\AesWorld\Source\...
# 改：同上

# 4. 编译
udf build task neon-dev/prefab-save-bug

# 5. 通知用户
# "任务完成，请运行 udf task switch neon-dev/prefab-save-bug 并重启 Editor 验收"
```
