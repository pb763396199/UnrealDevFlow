# Copilot Instructions for UnrealDevFlow

> **GitHub Copilot 入口文件**（`.github/copilot-instructions.md`）。
> 详细规范见仓库根目录的 `AGENTS.md`。
> 本文件是「5 步标准工作流 + 严禁行为 + 关键命令」的精简速查。

## Project context

UnrealDevFlow is a UE plugin parallel-development CLI: it creates isolated Git-worktree-based task hosts, switches NTFS Junctions into a UE project for verification, and merges per-plugin. Source repos under `F:\ShanghaiP4\neon\Plugins\` are NEVER modified — only worktrees under `Hosts/T-<id>_Host/Plugins/<plugin>/`.

**Any code work happens inside a worktree. Do not run raw `git merge` / `rebase` / `branch -D` / `worktree remove` / `reset --hard`. Use `unrealdevflow` commands only.**

---

## 5-step workflow (mandatory)

| Step | Command | When |
|---|---|---|
| 1. CREATE | `unrealdevflow create "<desc>" --id <id> --prompt "<raw prompt>" --yes` | When user gives a dev task |
| 2. WORK | Edit only `{hosts_root}/T-<id>_Host/Plugins/<plugin>/...` | During implementation |
| 3. BUILD | `unrealdevflow build <id>` (or `--background` / `--primary-only`) | After code changes |
| 4. SWITCH | Tell user to run `unrealdevflow switch <id>` + restart UE Editor | For manual verification |
| 5. MERGE | Ask user for strategy, then `unrealdevflow merge <id> --strategy <s>` | After user confirms verification |

**Step 5 detail**: `merge` REQUIRES `--strategy`. The 4 options are `rebase | merge | squash | ff-only`. **Always ask the user first** before running.

**Multi-primary tasks (v2)**: pass `--plugin <name>` for a single plugin, or `--all` to iterate in reverse order with independent confirmations.

---

## Hard rules

- ❌ Never run raw `git merge` / `rebase` / `cherry-pick` / `branch -D` / `worktree remove` / `reset --hard`
- ❌ Never auto-run `unrealdevflow cleanup` — wait for explicit user confirmation after merge
- ❌ Never pick a merge strategy without asking the user
- ❌ Never modify files under `{plugins_root}/<plugin>/` (the main repo)
- ❌ Never run `Build.bat` / `RunUBT.bat` directly — use `unrealdevflow build`
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
unrealdevflow configure --hosts-root X --plugins-root Y --default-project Z
unrealdevflow create "desc" --id <id> --prompt "..." --yes
unrealdevflow create "desc" --id <id> --primary AesWorld,AesWorld_AI --yes
unrealdevflow build <id> [--background] [--primary-only]
unrealdevflow build-status <id>
unrealdevflow switch <id>           # user runs, not agent
unrealdevflow merge <id> --strategy <rebase|merge|squash|ff-only> [--plugin <name>|--all]
unrealdevflow cleanup <id>          # after user confirms
unrealdevflow delete <id>           # if verification failed
unrealdevflow list
unrealdevflow status
```

---

## Detailed docs

- `AGENTS.md` — full canonical workflow (v2 multi-plugin, errors, troubleshooting)
- `CLAUDE.md` — Claude Code entry point (same content as this file)
- `skill/SKILL.md` — opencode skill system equivalent
- `docs/plans/2026-06-09-001-feat-multi-plugin-support-implementation-plan.md` — v2 design
- `docs/insights/2026-06-09-001-ubt-strict-build-flags-reference.md` — UBT strict flags reference
