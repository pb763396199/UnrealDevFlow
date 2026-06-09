---
name: UnrealDevFlow
description: Unreal Engine 插件多任务并行开发工作流。当用户提出开发任务、bug调查、功能验证时，使用此skill自动创建隔离的工作空间并推进任务。触发词：开发、调查、验证、修复、实现、测试功能。
argument-hint: '描述你要做的任务'
---

# UnrealDevFlow — UE 插件并行开发工作流

## When to Use

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
任务 Host (Hosts/T-xxx_Host)  ← 隔离的工作空间，独立编译
    ↓ NTFS Junction
UE 项目 (DEV/Plugins/AesWorld) ← 验收时切换指向
```

## 工具位置

Skill 通过 `unrealdevflow` 命令调用。确保工具已安装并在 PATH 中，或使用完整路径。

## 完整工作流

### 第一步：创建任务（保存用户原始意图）

```powershell
unrealdevflow create "任务描述" `
    --id task-id `
    --prompt "用户的原始prompt，完整保存" `
    --yes
```

**关键**：`--prompt` 必须保存用户的原始意图，后续 agent 需要读取它。

### 第二步：读取任务元信息

```powershell
# 从 config.toml 读取 hosts_root
$config = Get-Content "$env:USERPROFILE\.unrealdevflow\config.toml" | ConvertFrom-StringData
$hostsRoot = $config.hosts_root.Trim('"')

# 读取任务元信息
Get-Content "$hostsRoot\T-{task-id}_Host\.udf-meta.json"
```

获取 worktree 路径，后续所有文件操作都在这个路径下进行。

### 第三步：在 worktree 中工作

**所有代码修改必须在 worktree 路径下进行**：

```
{hosts_root}\T-{task-id}_Host\Plugins\AesWorld\Source\
```

- 读代码：从这个路径读
- 改代码：在这个路径改
- 不要碰主仓库 `{plugin_path}`

### 第四步：编译验证

```powershell
unrealdevflow build {task-id}
```

编译产物在 `T-{task-id}_Host\Plugins\AesWorld\Binaries\Win64\`

### 第五步：通知用户验收

告诉用户：
1. 任务已完成
2. 运行 `unrealdevflow switch {task-id}` 切换
3. 重启 UE Editor 验收

### 第六步：验收后处理

用户验收通过后：
```powershell
# 合并（不立即清理，保留 worktree 和分支供检查）
unrealdevflow merge {task-id} --strategy rebase

# 检查 merge 结果
git log --oneline -5

# 确认无误后清理
unrealdevflow cleanup {task-id}
```

或者一步到位（跳过检查，有风险）：
```powershell
unrealdevflow merge {task-id} --strategy rebase --cleanup
```

用户验收不通过：
```powershell
unrealdevflow delete {task-id} --yes --force
```

## 关键规则

1. **创建任务时必须带 --prompt** — 保存用户原始意图
2. **所有代码操作在 worktree 路径下** — 不要碰主仓库
3. **不要自动 switch** — switch 需要用户手动执行（涉及 Editor 重启）
4. **编译用 build 命令** — 不要自己调 Build.bat
5. **完成后通知用户** — 告诉用户怎么验收
6. **Commit message 必须是中文** — 遵循下方格式要求，包含反思内容

## Commit Message 格式要求

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
| `configure --hosts-root ... --plugin-path ... --default-project ...` | 配置 |
| `create "描述" --id xxx --prompt "原始 prompt" --yes` | 创建任务 |
| `build <id>` | 编译 |
| `switch <id> --force` | 切换 Junction |
| `list` | 列出任务 |
| `status` | 查看状态 |
| `merge <id> --strategy <策略>` | 合并（保留 worktree 和分支） |
| `merge <id> --strategy <策略> --cleanup` | 合并并立即清理 |
| `cleanup <id>` | 手动清理 worktree 和分支 |
| `delete <id> --yes --force` | 删除并清理（不合并） |
| `merge <id> --dry-run` | 预览合并 |
| `delete <id> --dry-run` | 预览删除 |

## 示例：完整任务流程

用户说："调查 EarthPrefabActor 保存后 Component 丢失的问题"

```powershell
# 1. 创建任务
unrealdevflow create "EarthPrefabActor保存后Component丢失问题调查" `
    --id prefab-save-bug `
    --prompt "调查EarthPrefabActor在关卡中保存后重新打开时Component丢失的问题，怀疑是EarthSplineComponent或EarthDataBase的同步逻辑导致" `
    --yes

# 2. 读取元信息（从 config.toml 获取 hosts_root）
$config = Get-Content "$env:USERPROFILE\.unrealdevflow\config.toml" | ConvertFrom-StringData
$hostsRoot = $config.hosts_root.Trim('"')
Get-Content "$hostsRoot\T-prefab-save-bug_Host\.udf-meta.json"

# 3. 在 worktree 中调查代码
# 读：$hostsRoot\T-prefab-save-bug_Host\Plugins\AesWorld\Source\...
# 改：同上

# 4. 编译
unrealdevflow build prefab-save-bug

# 5. 通知用户
# "任务完成，请运行 unrealdevflow switch prefab-save-bug 并重启 Editor 验收"
```
