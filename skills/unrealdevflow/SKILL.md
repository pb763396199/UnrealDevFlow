---
name: unrealdevflow
description: UE plugin parallel development workflow (git worktree + NTFS junction + strict UBT). Use when user asks to develop / investigate / fix / verify an Unreal Engine plugin. Triggers: 开发、调查、验证、修复、实现、测试功能.
user-invocable: true
argument-hint: "<task description>"
---

# UnrealDevFlow — UE 插件多任务并行开发

## When to use

- User asks to develop / investigate / fix / verify an Unreal Engine plugin
- A task needs `git worktree` isolation from the main plugin repo
- The change must be compiled against a real UE Editor target (with strict dependency checks)
- A primary plugin depends on other plugins (e.g. `EarthPCG` depends on `AesWorld`) — both need to compile together

## Binary location

The `unrealdevflow` CLI should be in PATH. If `Get-Command unrealdevflow` fails, use the absolute path:

```
F:\AiProject\UnrealDevFlow\target\release\unrealdevflow.exe
```

You can set a session alias: `Set-Alias unrealdevflow "F:\AiProject\UnrealDevFlow\target\release\unrealdevflow.exe"`

## 5-step workflow (the only correct one)

### 1. CREATE — create isolated task workspace

```powershell
unrealdevflow create "<task description>" `
    --id <task-id> `
    --prompt "<user's original prompt — preserve verbatim>" `
    --primary AesWorld,AesWorld_AI `
    --override-dep PCG=project `
    --yes
```

- `--id` must be a short kebab-case identifier (e.g. `prefab-save-bug`).
- `--prompt` is **mandatory** — every later agent reads it.
- `--primary` accepts comma-separated plugin names (v2 multi-plugin).
- `--override-dep` resolves engine-vs-project conflict (`<name>=engine|project|<absolute-path>`).
- The tool auto-parses each primary plugin's `.uplugin`, scans engine+project plugin roots, and creates Junctions for project-local dependencies.

### 2. WORK — edit code in the worktree only

```
Task worktree: {hosts_root}/T-<id>_Host/Plugins/<plugin-name>/Source/...
Task meta:     {hosts_root}/T-<id>_Host/.udf-meta.json
```

- ✅ Read, edit, and `git commit` inside the worktree.
- ✅ Commit messages must be **Chinese**, format `Task#XXX [内容] / 修改内容 / 过程反思 / 后续注意`.
- ❌ Never modify `{plugins_root}/<plugin>/` (the main plugin repo).
- ❌ Never modify other tasks' worktrees or the DEV project.

### 3. BUILD — compile with strict flags (default)

```powershell
unrealdevflow build <id>                    # default profile = light
unrealdevflow build <id> --primary-only     # only primary plugin modules (-Module=...)
unrealdevflow build <id> --profile medium   # + WarningsAsErrors
unrealdevflow build <id> --profile heavy    # -Rebuild -DisableUnity -NoSharedPCH (slow)
unrealdevflow build-status <id>
```

- Light profile: `-FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity`.
- Failure log: `<host>/Logs/UBT/Build_<profile>_<timestamp>.log`.

### 4. SWITCH — let user verify in UE Editor (do NOT run yourself)

Tell the user:

```
✅ Task <id> built successfully.

1. unrealdevflow switch <id>   ← user runs this
2. Restart UE Editor
3. Verify the feature

After verification:
  unrealdevflow merge <id> --strategy <user picks>
  unrealdevflow cleanup <id>

If verification failed:
  unrealdevflow delete <id> --yes --force
```

`switch` clears UBT cache and rebuilds Junctions for every primary + dependency plugin.

### 5. MERGE — only after user confirms

`--strategy` is **required** and you **must ask the user** which one (1.rebase / 2.merge / 3.squash / 4.ff-only). Never pick a default for them.

```powershell
# Single primary plugin
unrealdevflow merge <id> --plugin AesWorld --strategy rebase

# All primary plugins (v2, in reverse declaration order, independent confirmations)
unrealdevflow merge <id> --all --strategy rebase

# Inspect result
git log --oneline -5

# ONLY after user confirms the merge looks right:
unrealdevflow cleanup <id>
```

## 🚫 Hard rules

| ❌ Forbidden | ✅ Use instead |
|---|---|
| `git merge` / `git rebase` / `git cherry-pick` | `unrealdevflow merge` |
| `git branch -D` / `git worktree remove` / `git reset --hard` | `unrealdevflow cleanup` / `delete` |
| Auto-`cleanup` right after merge | Wait for explicit user confirmation |
| Picking `--strategy` without asking the user | Ask, then pass the user's choice |
| Editing the main plugin repo `{plugins_root}/<plugin>/` | Edit only the task worktree |
| Running `Build.bat` / `RunUBT.bat` directly | `unrealdevflow build` |

## Failure recovery

| Symptom | Fix |
|---|---|
| "目录被占用" on `switch` | Close Rider/VSCode/file explorer, retry with `unrealdevflow switch <id> --force` |
| "Build.bat 不存在" | Re-run `unrealdevflow configure --engine-path "正确路径"` |
| Wrong merge order | `git reflog`, then `git reset --hard <before-merge-commit>`, re-run `unrealdevflow merge` |
| Engine+project dep conflict | `unrealdevflow create ... --override-dep <name>=engine\|project` |

## Full docs (in the tool's source repo)

- `AGENTS.md` — canonical workflow + v2 multi-plugin + commit format + troubleshooting
- `docs/plans/2026-06-09-001-feat-multi-plugin-support-implementation-plan.md` — v2 design
- `docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md` — UBT strict flags reference
