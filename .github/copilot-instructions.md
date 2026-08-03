# Copilot Instructions for UnrealDevFlow

> **GitHub Copilot 入口文件**（`.github/copilot-instructions.md`）。
> 详细规范见仓库根目录的 `AGENTS.md`。
> 本文件是「5 步标准工作流 + 严禁行为 + 关键命令」的精简速查。

## Project context

UnrealDevFlow is a UE plugin parallel-development CLI: it creates isolated Git-worktree-based task hosts, switches NTFS Junctions into a UE project for verification, and merges per-plugin. Source repos under `F:\ShanghaiP4\neon\Plugins\` are NEVER modified — only worktrees under `Hosts/T-<id>_Host/Plugins/<plugin>/`.

**Any code work happens inside a worktree. Do not run raw `git merge` / `rebase` / `branch -D` / `worktree remove` / `reset --hard`. Use `udf` commands only.**

---

## 5-step workflow (mandatory)

| Step | Command | When |
|---|---|---|
| 1. CREATE | `udf task create "<desc>" --id <id> --prompt "<raw prompt>" --yes` | When user gives a dev task |
| 2. WORK | Edit only `{hosts_root}/T-<id>_Host/Plugins/<plugin>/...` | During implementation |
| 3. BUILD | `udf build task <id>` (or `--background` / `--primary-only`) | After code changes |
| 4. SWITCH | Tell user to run `udf task switch <id>` + restart UE Editor | For manual verification |
| 5. MERGE | Ask user for strategy, then `udf task merge <id> --strategy <s>` | After user confirms verification |

**Step 5 detail**: `merge` REQUIRES `--strategy`. The 4 options are `rebase | merge | squash | ff-only`. **Always ask the user first** before running.

**Multi-primary tasks (v2)**: pass `--plugin <name>` for a single plugin, or `--all` to iterate in reverse order with independent confirmations.

---

## Hard rules

- ❌ Never run raw `git merge` / `rebase` / `cherry-pick` / `branch -D` / `worktree remove` / `reset --hard`
- ❌ Never auto-run `udf task cleanup` — wait for explicit user confirmation after merge
- ❌ Never pick a merge strategy without asking the user
- ❌ Never modify files under `{plugins_root}/<plugin>/` (the main repo)
- ❌ Never run `Build.bat` / `RunUBT.bat` directly — use `udf build task`
- ✅ All code edits happen inside the task's worktree
- ✅ Commit messages must be **Chinese** with the `Task#XXX` + 反思 format (see AGENTS.md)

---

## Commit message format (Chinese required)

```
Task#[number] [内容摘要]

修改内容：
- ...
过程反思：
- ...
后续注意：
- ...
```

---

## Quick command reference

```
udf workspace add --hosts-root X --plugins-root Y --default-project Z
udf task create "desc" --id <id> --prompt "..." --yes
udf task create "desc" --id <id> --primary AesWorld,AesWorld_AI --yes
udf build task <id> [--background] [--primary-only]
udf build status <id>
udf task switch <id>           # user runs, not agent
udf task merge <id> --strategy <rebase|merge|squash|ff-only> [--plugin <name>|--all]
udf task cleanup <id>          # after user confirms
udf task delete <id>           # if verification failed
udf task list
udf workspace status
```

---

## Detailed docs

- `AGENTS.md` — full canonical workflow (v2 multi-plugin, errors, troubleshooting)
- `CLAUDE.md` — Claude Code entry point (same content as this file)
- `skill/SKILL.md` — opencode skill system equivalent
- `docs/plans/2026-06-09-001-feat-multi-plugin-support-implementation-plan.md` — v2 design
- `docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md` — UBT strict flags reference
