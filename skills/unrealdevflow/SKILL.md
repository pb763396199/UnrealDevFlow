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

The `udf` CLI should be in PATH. If `Get-Command udf` fails, use the release installer location:

```powershell
$udf = Get-Command udf -ErrorAction SilentlyContinue
if (-not $udf) {
    $udf = "$env:USERPROFILE\.unrealdevflow\bin\udf.exe"
}
```

If neither path exists, install the tool first:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pb763396199/UnrealDevFlow/releases/latest/download/unrealdevflow-installer.ps1 | iex"
```

## Beginner flow (preferred)

For first-time users, keep the flow to four commands:

```powershell
udf workspace init --project "<UE project dir>" [--workspace <name>]
udf task create "<user's original request>" --workspace <name> --primary <Plugin> --id <task-id> --yes
udf task next <workspace>/<task-id>
udf task finish <workspace>/<task-id>
```

- `init` detects project/plugins/engine/hosts, saves a named workspace, installs the AI skill, and runs doctor.
- Workspace names identify a UE project environment, not a task. Prefer the tool suggestion or a project name such as `neon-dev`; never use task names such as `sublevel-tweak` as the workspace.
- If more than one workspace exists, use full task refs: `workspace/task-id`. Never guess from a short id when ambiguous.
- `start` wraps `create` and preserves the original user request as the task prompt.
- `next` prints only the next action the user should take.
- `finish` is the merge guide; the user must still choose the merge strategy, with `rebase` as the recommended default.

## 5-step workflow (agent reference)

### 1. START/CREATE — create isolated task workspace

```powershell
udf task create "<task description>" `
    --workspace <workspace-name> `
    --id <task-id> `
    --primary AesWorld,AesWorld_AI `
    --override-dep PCG=project `
    --yes

# Low-level equivalent:
udf task create "<task description>" `
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
- `--id` must contain only lowercase ASCII letters, digits, and hyphens; never include `/`, `\`, `..`, spaces, or Chinese characters.
- Create/start requires each primary plugin's main checkout to be on a clean `dev` branch. Do not create from feature/task branches, detached HEAD, Host worktrees, or DEV project Junctions.
- `plugins_root` must be the stable main plugin repository root, not `<UE project>/Plugins`, a Host directory, a Junction/symlink/reparse point, or any path containing duplicate plugin names.
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
udf build task <task-ref>                    # default profile = light
udf build task <task-ref> --primary-only     # only primary plugin modules (-Module=...)
udf build task <task-ref> --profile medium   # + WarningsAsErrors
udf build task <task-ref> --profile heavy    # -Rebuild -DisableUnity -NoSharedPCH (slow)
udf build status <task-ref>
```

- Light profile: `-FailIfGeneratedCodeChanges -NoUBTMakefiles -DisableAdaptiveUnity`.
- Failure log: `<host>/Logs/UBT/Build_<profile>_<timestamp>.log`.

#### Controlled build: ask before you compile

One engine directory has exactly one UnrealBuildTool mutex. Two builds at once
corrupt each other's intermediates, so never hand-assemble a `Build.bat` line.

```powershell
udf build check <task-ref>                 # may a build start right now?
udf build check <task-ref> --format json   # same answer, machine readable
udf build check --workspace <name>         # no task ref = the main project
```

Four verdicts. The command itself always exits 0 — the verdict is the answer,
not a failure to answer.

| Verdict | Meaning | What to do |
|---|---|---|
| `ready` | a build may start | run `udf build task <task-ref>` |
| `deferred` | another UBT holds the mutex | wait, do not bypass it |
| `blocked` | refused by policy (e.g. `-NoMutex` present) | read the `reason` field |
| `needsUserInput` | engine/project could not be resolved | pass `--workspace`, or fix the workspace config |

```powershell
# Does a proposed command bypass the controlled path? Exit code 1 when refused.
udf build gate "<the full command you were about to run>"

# Build the workspace's main project instead of a task Host
udf build project [--workspace <name>] [--profile light|medium|heavy]
```

Engine root resolution order: the workspace's `engine_path` → the
`UNREALDEVFLOW_UE_ENGINE_ROOT` environment variable → `EngineAssociation` in
the `.uproject`.

### 4. SWITCH — NEVER run without explicit user authorization

**⛔ HARD RULE: You MUST NOT run `udf task switch` unless the user explicitly says to do so.**

This is not a suggestion. This is a hard prohibition. Reasons:
- `switch` rewrites NTFS Junctions that the running UE Editor depends on
- Running it while Editor is open will corrupt the Editor session
- The user must close UE Editor first, then authorize the switch

When the build succeeds, **tell the user** and **wait for their explicit instruction**:

```
✅ Task <id> built successfully.

To verify, please:
1. Close UE Editor (if running)
2. Tell me to run: udf task switch <task-ref>
3. Then restart UE Editor and verify the feature

After verification:
  udf task finish <task-ref>
  udf task cleanup <task-ref>

If verification failed:
  udf task delete <task-ref> --yes --force
```

Even if the user says "帮我切" or "switch it", you should confirm the exact command before executing, because switch is destructive to the running Editor session.

### 5. MERGE — only after user confirms

**⚠️ IMPORTANT: `udf task merge` automatically fetches from origin before merging.**

This ensures you're merging against the latest remote state. If other tasks have been merged to dev while you were working, the merge will include those changes.

`--strategy` is **required** and you **must ask the user** which one (1.rebase / 2.merge / 3.squash / 4.ff-only). Never pick a default for them.

```powershell
# Single primary plugin
udf task merge <task-ref> --plugin AesWorld --strategy rebase

# All primary plugins (v2, in reverse declaration order, independent confirmations)
udf task merge <task-ref> --all --strategy rebase

# Inspect result
git log --oneline -5

# ONLY after user confirms the merge looks right:
udf task cleanup <task-ref>
```

## 🚫 Hard rules

| ❌ Forbidden | ✅ Use instead |
|---|---|
| **Running `udf task switch` without explicit user authorization** | **Tell user the command, wait for them to say "run it"** |
| `git merge` / `git rebase` / `git cherry-pick` | `udf task merge` |
| `git branch -D` / `git worktree remove` / `git reset --hard` | `udf task cleanup` / `delete` |
| Auto-`cleanup` right after merge | Wait for explicit user confirmation |
| Picking `--strategy` without asking the user | Ask, then pass the user's choice |
| Editing the main plugin repo `{plugins_root}/<plugin>/` | Edit only the task worktree |
| Running `Build.bat` / `RunUBT.bat` directly | `udf build task`, after `udf build check` says `ready` |
| Passing `-NoMutex` to work around a busy build | Wait out the `deferred` verdict |

## Failure recovery

| Symptom | Fix |
|---|---|
| "目录被占用" on `switch` | Close Rider/VSCode/file explorer, retry with `udf task switch <task-ref> --force` |
| "Build.bat 不存在" | Re-run `udf workspace add --engine-path "正确路径"` |
| `build` seems to hang behind another compile | `udf build check <task-ref>` — `deferred` means another UBT holds the mutex |
| Wrong merge order | `git reflog`, then `git reset --hard <before-merge-commit>`, re-run `udf task merge` |
| Engine+project dep conflict | `udf task create ... --override-dep <name>=engine\|project` |
| Short task id is ambiguous | Use full ref `workspace/task-id` |
| "Broken junction" warning on `switch` | **Auto-fixed**: `switch` detects broken junctions (target deleted) and removes them automatically. This happens when a task was deleted without cleanup. No manual action needed. |

## Junction lifecycle (important)

When you run `udf task delete <task-ref>`, the tool **automatically cleans up junctions** in all known main projects that point to the deleted task's worktrees. This prevents "broken junctions" that would block future `switch` operations.

**What happens during `delete`:**
1. Scans all projects in `~/.unrealdevflow/state.json`
2. Finds junctions pointing to the task's worktrees
3. Removes those junctions from the main projects
4. Then deletes the task's worktrees and branches

**What happens during `switch`:**
- If it encounters a broken junction (target no longer exists), it automatically removes it and creates a fresh junction
- You'll see a warning: "Found broken junction at ... (target no longer exists)"
- This is safe and expected behavior

**Best practice:** Always use `udf task delete` instead of manually removing worktree directories. This ensures junctions are properly cleaned up.

## Full docs (in the tool's source repo)

- `AGENTS.md` — canonical workflow + v2 multi-plugin + commit format + troubleshooting
- `README.md` — user-facing install, init/start/next/finish, workspace docs
- `docs/plans/2026-06-12-002-multi-workspace-and-simple-commands.md` — workspace + beginner command design
- `docs/plans/2026-06-09-001-feat-multi-plugin-support-implementation-plan.md` — v2 design
- `docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md` — UBT strict flags reference
