# CLAUDE.md

> **Claude Code 入口文件**。读这个文件就是 UnrealDevFlow 的完整工作流。
> 详细文档见 `AGENTS.md`，本文件是「5 步标准工作流 + 禁止行为 + 关键命令」的精简版。

---

## 5 步标准工作流

### 1. CREATE — 创建任务
```powershell
unrealdevflow create "任务描述" --id task-id --prompt "用户原始prompt" --yes
```
- `--id` 短英文 kebab-case
- `--prompt` **必须完整保存用户原始需求**
- 多主插件：`--primary AesWorld,AesWorld_AI`
- 解决引擎/项目冲突：`--override-dep PCG=project|engine|<path>`

### 2. WORK — 在 worktree 中工作
```
任务路径：{hosts_root}/T-{id}_Host/Plugins/<plugin>/Source/...
```
- ✅ 改 worktree；commit 用中文格式（含反思）
- ❌ 不动主仓库 `{plugins_root}/`；不动其他 worktree；不动 DEV 项目

### 3. BUILD — 编译验证
```powershell
unrealdevflow build <id>                    # 严格模式默认
unrealdevflow build <id> --background
unrealdevflow build <id> --primary-only     # 只编主插件
unrealdevflow build-status <id>
```
- 严格模式自动启用：`-FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity`
- 失败看 `Build_<time>.log`

### 4. SWITCH — 让用户验收
**不要自己执行 switch！** 告诉用户：
```
✅ 任务 <id> 已完成
1. unrealdevflow switch <id>
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
unrealdevflow merge <id> --strategy rebase
git log --oneline -5
# ⚠️ 等用户确认后再 cleanup
unrealdevflow cleanup <id>
```

**多主插件任务**：
```powershell
unrealdevflow merge <id> --plugin AesWorld --strategy rebase
unrealdevflow merge <id> --all --strategy rebase   # 逆序逐个
```

---

## 严禁行为

| ❌ 禁止 | ✅ 改用 |
|---|---|
| `git merge` / `git rebase` / `git cherry-pick` | `unrealdevflow merge` |
| `git branch -D` / `git worktree remove` / `git reset --hard` | `unrealdevflow cleanup` / `delete` |
| 自动 cleanup（merge 后未确认） | 等用户确认才 cleanup |
| 不询问就指定 `--strategy rebase` | 询问 4 选 1 |
| 修改主仓库 `{plugins_root}/<plugin>/` | 只改 worktree |
| 跑 `Build.bat` / `RunUBT.bat` | `unrealdevflow build` |

---

## 关键命令速查

| 场景 | 命令 |
|---|---|
| 创建任务 | `unrealdevflow create "..." --id xxx --prompt "..." --yes` |
| 编译 | `unrealdevflow build xxx [--background] [--primary-only]` |
| 编译状态 | `unrealdevflow build-status xxx` |
| 通知用户验收 | 告诉用户 `unrealdevflow switch xxx` + 重启 Editor |
| 合并（必问策略） | `unrealdevflow merge xxx --strategy <s> [--plugin <name> \| --all]` |
| 清理 | `unrealdevflow cleanup xxx`（用户确认后） |
| 删除（不合并） | `unrealdevflow delete xxx` |
| 列任务 | `unrealdevflow list` |
| 看状态 | `unrealdevflow status` |

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
