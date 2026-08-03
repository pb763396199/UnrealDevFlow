# CLAUDE.md

> **Claude Code 入口文件**。读这个文件就是 UnrealDevFlow 的完整工作流。
> 详细文档见 `AGENTS.md`，本文件是「5 步标准工作流 + 禁止行为 + 关键命令」的精简版。

---

## 小白优先入口

```powershell
udf workspace init --project "<UE项目目录>" [--workspace <name>]
udf task create "用户原始需求" --workspace <name> --primary <Plugin> --id <task-id> --yes
udf task next <workspace>/<task-id>
udf task finish <workspace>/<task-id>
```

- `init` 自动探测 UE 项目、Plugins 根目录、Engine、Hosts，并安装 AI skill。
- workspace 名称可自动建议，也可用户自定义；多个 workspace 时任务引用必须写成 `workspace/task-id`。
- `task create` 不给 `--prompt` 时会把描述当原始需求存进元数据。
- `next` 只告诉用户下一步。
- `finish` 是合并向导，必须让用户选择策略，默认推荐 rebase。

## 5 步标准工作流

### 1. START/CREATE — 创建任务
```powershell
udf task create "任务描述" --workspace workspace-name --id task-id --primary AesWorld --yes

# 专业模式
udf task create "任务描述" --workspace workspace-name --id task-id --prompt "用户原始prompt" --primary AesWorld --yes
```
- `--id` 短英文 kebab-case
- `--prompt` **必须完整保存用户原始需求**
- `--workspace` 多项目并行时必须显式指定
- 多主插件：`--primary AesWorld,AesWorld_AI`
- 解决引擎/项目冲突：`--override-dep PCG=project|engine|<path>`

### 2. WORK — 在 worktree 中工作
```
任务路径：{hosts_root}/W-{workspace}/T-{id}_Host/Plugins/<plugin>/Source/...
任务引用：{workspace}/{id}
```
- ✅ 改 worktree；commit 用中文格式（含反思）
- ❌ 不动主仓库 `{plugins_root}/`；不动其他 worktree；不动 DEV 项目

### 3. BUILD — 编译验证
```powershell
udf build task <task-ref>                    # 严格模式默认
udf build task <task-ref> --background
udf build task <task-ref> --primary-only     # 只编主插件
udf build status <task-ref>
```
- 严格模式自动启用：`-FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity`
- 失败看 `Build_<time>.log`

**编之前先问能不能编**（同一引擎只有一把 UBT 互斥锁）：
```powershell
udf build check <task-ref>              # ready / deferred / blocked / needsUserInput
udf build gate "<完整命令>"              # 这条命令绕过受控构建了吗，拦下时退出码 1
udf build project [--workspace <name>]  # 编主项目而不是任务宿主
```
- `deferred` 表示别人正在编，等它结束；**不要**改用 `--mutex no-mutex` 绕过去。
- `build check` 本身永远返回 0，结论就是答案。

### 4. SWITCH — 让用户验收
**不要自己执行 switch！** 告诉用户：
```
✅ 任务 <id> 已完成
1. udf task switch <task-ref>
2. 重启 UE Editor
3. 验证
```

### 5. MERGE → CLEANUP（用户确认后）
**`--strategy` 是必填参数，必须询问用户**：
```
请选择合并策略：1.rebase（推荐） 2.merge 3.squash 4.ff-only
```
然后：
```powershell
udf task merge <task-ref> --strategy rebase
git log --oneline -5
# ⚠️ 等用户确认后再 cleanup
udf task cleanup <task-ref>
```

**多主插件任务**：
```powershell
udf task merge <task-ref> --plugin AesWorld --strategy rebase
udf task merge <task-ref> --all --strategy rebase   # 逆序逐个
```

---

## 严禁行为

| ❌ 禁止 | ✅ 改用 |
|---|---|
| `git merge` / `git rebase` / `git cherry-pick` | `udf task merge` |
| `git branch -D` / `git worktree remove` / `git reset --hard` | `udf task cleanup` / `delete` |
| 自动 cleanup（merge 后未确认） | 等用户确认才 cleanup |
| 不询问就指定 `--strategy rebase` | 询问 4 选 1 |
| 修改主仓库 `{plugins_root}/<plugin>/` | 只改 worktree |
| 跑 `Build.bat` / `RunUBT.bat` | `udf build task` |

---

## 关键命令速查

| 场景 | 命令 |
|---|---|
| 初始化 | `udf workspace init --project <UE项目> [--workspace <name>]` |
| 创建任务 | `udf task create "..." --workspace <w> --primary <Plugin> --id xxx --yes` |
| 下一步 | `udf task next <workspace/task>` |
| 编译 | `udf build task <workspace/task> [--background] [--primary-only]` |
| 编译状态 | `udf build status <workspace/task>` |
| 能不能编 | `udf build check <workspace/task> [--format json]` |
| 命令是否绕过受控构建 | `udf build gate "<完整命令>"` |
| 编主项目 | `udf build project [--workspace <w>]` |
| 通知用户验收 | 告诉用户 `udf task switch <workspace/task>` + 重启 Editor |
| 合并向导 | `udf task finish <workspace/task>` |
| 合并（必问策略） | `udf task merge <workspace/task> --strategy <s> [--plugin <name> \| --all]` |
| 清理 | `udf task cleanup <workspace/task>`（用户确认后） |
| 删除（不合并） | `udf task delete <workspace/task>` |
| 列任务 | `udf task list` |
| 列 workspace | `udf workspace list` |
| 看状态 | `udf workspace status` |

---

## Commit Message 格式（必须中文）

```
Task#[number] [内容摘要]

修改内容：
- [具体修改 1]
- [具体修改 2]

过程反思：
- [偏差/教训/范式总结，1-3 条]

后续注意：
- [避免再犯的点，1-3 条]
```

---

## 详细文档
- `AGENTS.md` — 完整规范（含 v2 多插件、错误处理、配置参考、故障排查）
- `skill/SKILL.md` — opencode skill 系统的等价内容
- `docs/plans/2026-06-09-001-feat-multi-plugin-support-implementation-plan.md` — v2 多插件设计
- `docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md` — UBT 严格编译参数考据
