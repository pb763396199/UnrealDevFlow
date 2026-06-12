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

The `unrealdevflow` CLI should be in PATH. If `Get-Command unrealdevflow` fails, use the release installer location:

```powershell
$udf = Get-Command unrealdevflow -ErrorAction SilentlyContinue
if (-not $udf) {
    $udf = "$env:USERPROFILE\.unrealdevflow\bin\unrealdevflow.exe"
}
```

If neither path exists, install the tool first:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pb763396199/UnrealDevFlow/releases/latest/download/unrealdevflow-installer.ps1 | iex"
```

## Beginner flow (preferred)

For first-time users, keep the flow to four commands:

```powershell
unrealdevflow init --project "<UE project dir>" [--workspace <name>]
unrealdevflow start "<user's original request>" --workspace <name> --primary <Plugin> --id <task-id> --yes
unrealdevflow next <workspace>/<task-id>
unrealdevflow finish <workspace>/<task-id>
```

- `init` detects project/plugins/engine/hosts, saves a named workspace, installs the AI skill, and runs doctor.
- Workspace names can be suggested by the tool or user-provided; they are normalized to safe kebab-case.
- If more than one workspace exists, use full task refs: `workspace/task-id`. Never guess from a short id when ambiguous.
- `start` wraps `create` and preserves the original user request as the task prompt.
- `next` prints only the next action the user should take.
- `finish` is the merge guide; the user must still choose the merge strategy, with `rebase` as the recommended default.

## 5-step workflow (agent reference)

### 1. START/CREATE — create isolated task workspace

```powershell
unrealdevflow start "<task description>" `
    --workspace <workspace-name> `
    --id <task-id> `
    --primary AesWorld,AesWorld_AI `
    --override-dep PCG=project `
    --yes

# Low-level equivalent:
unrealdevflow create "<task description>" `
    --workspace <workspace-name> `
    --id <task-id> `
    --prompt "<user's original prompt — preserve verbatim>" `
    --primary AesWorld,AesWorld_AI `
    --override-dep PCG=project `
    --yes
```

- `--id` must be a short kebab-case identifier (e.g. `prefab-save-bug`).
- `--prompt` is **mandatory** — every later agent reads it.
- `--workspace` is required when multiple UE project workspaces exist.
- `--primary` accepts comma-separated plugin names (v2 multi-plugin).
- `--override-dep` resolves engine-vs-project conflict (`<name>=engine|project|<absolute-path>`).
- The tool auto-parses each primary plugin's `.uplugin`, scans engine+project plugin roots, and creates Junctions for project-local dependencies.
- New workspace tasks live under `<hosts_root>/W-<workspace>/T-<task-id>_Host`.
- New metadata writes `workspace`, `task_uid`, and a frozen `context` so later commands do not depend on mutable global defaults.

### 2. WORK — edit code in the worktree only

```
Task worktree: {hosts_root}/W-<workspace>/T-<id>_Host/Plugins/<plugin-name>/Source/...
Task meta:     {hosts_root}/W-<workspace>/T-<id>_Host/.udf-meta.json
Task ref:      <workspace>/<id>
```

- ✅ Read, edit, and `git commit` inside the worktree.
- ✅ Commit messages must be **Chinese**, format `Task#XXX [内容] / 修改内容 / 过程反思 / 后续注意`.
- ❌ Never modify `{plugins_root}/<plugin>/` (the main plugin repo).
- ❌ Never modify other tasks' worktrees or the DEV project.

### 3. BUILD — compile with strict flags (default)

```powershell
unrealdevflow build <task-ref>                    # default profile = light
unrealdevflow build <task-ref> --primary-only     # only primary plugin modules (-Module=...)
unrealdevflow build <task-ref> --profile medium   # + WarningsAsErrors
unrealdevflow build <task-ref> --profile heavy    # -Rebuild -DisableUnity -NoSharedPCH (slow)
unrealdevflow build-status <task-ref>
```

- Light profile: `-FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity`.
- Failure log: `<host>/Logs/UBT/Build_<profile>_<timestamp>.log`.

### 4. SWITCH — NEVER run without explicit user authorization

**⛔ HARD RULE: You MUST NOT run `unrealdevflow switch` unless the user explicitly says to do so.**

This is not a suggestion. This is a hard prohibition. Reasons:
- `switch` rewrites NTFS Junctions that the running UE Editor depends on
- Running it while Editor is open will corrupt the Editor session
- The user must close UE Editor first, then authorize the switch

When the build succeeds, **tell the user** and **wait for their explicit instruction**:

```
✅ Task <id> built successfully.

To verify, please:
1. Close UE Editor (if running)
2. Tell me to run: unrealdevflow switch <task-ref>
3. Then restart UE Editor and verify the feature

After verification:
  unrealdevflow finish <task-ref>
  unrealdevflow cleanup <task-ref>

If verification failed:
  unrealdevflow delete <task-ref> --yes --force
```

Even if the user says "帮我切" or "switch it", you should confirm the exact command before executing, because switch is destructive to the running Editor session.

### 5. MERGE — only after user confirms

**⚠️ IMPORTANT: `unrealdevflow merge` automatically fetches from origin before merging.**

This ensures you're merging against the latest remote state. If other tasks have been merged to dev while you were working, the merge will include those changes.

`--strategy` is **required** and you **must ask the user** which one (1.rebase / 2.merge / 3.squash / 4.ff-only). Never pick a default for them.

```powershell
# Single primary plugin
unrealdevflow merge <task-ref> --plugin AesWorld --strategy rebase

# All primary plugins (v2, in reverse declaration order, independent confirmations)
unrealdevflow merge <task-ref> --all --strategy rebase

# Inspect result
git log --oneline -5

# ONLY after user confirms the merge looks right:
unrealdevflow cleanup <task-ref>
```

## 🚫 Hard rules

| ❌ Forbidden | ✅ Use instead |
|---|---|
| **Running `unrealdevflow switch` without explicit user authorization** | **Tell user the command, wait for them to say "run it"** |
| `git merge` / `git rebase` / `git cherry-pick` | `unrealdevflow merge` |
| `git branch -D` / `git worktree remove` / `git reset --hard` | `unrealdevflow cleanup` / `delete` |
| Auto-`cleanup` right after merge | Wait for explicit user confirmation |
| Picking `--strategy` without asking the user | Ask, then pass the user's choice |
| Editing the main plugin repo `{plugins_root}/<plugin>/` | Edit only the task worktree |
| Running `Build.bat` / `RunUBT.bat` directly | `unrealdevflow build` |

## Failure recovery

| Symptom | Fix |
|---|---|
| "目录被占用" on `switch` | Close Rider/VSCode/file explorer, retry with `unrealdevflow switch <task-ref> --force` |
| "Build.bat 不存在" | Re-run `unrealdevflow configure --engine-path "正确路径"` |
| Wrong merge order | `git reflog`, then `git reset --hard <before-merge-commit>`, re-run `unrealdevflow merge` |
| Engine+project dep conflict | `unrealdevflow create ... --override-dep <name>=engine\|project` |
| Short task id is ambiguous | Use full ref `workspace/task-id` |
| "Broken junction" warning on `switch` | **Auto-fixed**: `switch` detects broken junctions (target deleted) and removes them automatically. This happens when a task was deleted without cleanup. No manual action needed. |

## Junction lifecycle (important)

When you run `unrealdevflow delete <task-ref>`, the tool **automatically cleans up junctions** in all known main projects that point to the deleted task's worktrees. This prevents "broken junctions" that would block future `switch` operations.

**What happens during `delete`:**
1. Scans all projects in `~/.unrealdevflow/state.json`
2. Finds junctions pointing to the task's worktrees
3. Removes those junctions from the main projects
4. Then deletes the task's worktrees and branches

**What happens during `switch`:**
- If it encounters a broken junction (target no longer exists), it automatically removes it and creates a fresh junction
- You'll see a warning: "Found broken junction at ... (target no longer exists)"
- This is safe and expected behavior

**Best practice:** Always use `unrealdevflow delete` instead of manually removing worktree directories. This ensures junctions are properly cleaned up.

## Full docs (in the tool's source repo)

- `AGENTS.md` — canonical workflow + v2 multi-plugin + commit format + troubleshooting
- `README.md` — user-facing install, init/start/next/finish, workspace docs
- `docs/plans/2026-06-12-002-multi-workspace-and-simple-commands.md` — workspace + beginner command design
- `docs/plans/2026-06-09-001-feat-multi-plugin-support-implementation-plan.md` — v2 design
- `docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md` — UBT strict flags reference
