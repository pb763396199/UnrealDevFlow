---
name: unrealdevflow-release
description: Standard UnrealDevFlow release workflow. Use when preparing, validating, packaging, tagging, or publishing a GitHub Release for this repository.
user-invocable: true
argument-hint: "<version>"
---

# UnrealDevFlow Release Skill

Use this skill when the user asks to release, package, publish, ship, tag, or prepare a version of UnrealDevFlow.

## Hard Rules

- Do not publish from memory. Follow this file and `docs/RELEASE.md`.
- Do not tag or push a release if `scripts/release-preflight.ps1 -Strict` fails.
- Do not skip `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- Do not create a GitHub Release without the required assets.
- Do not publish the draft release until the installer smoke test is checked.
- Do not publish generic release notes. Each release needs curated notes that
  explain the concrete user-facing changes in that exact version.
- Do not reuse the previous release's "新增/修复/AI Agent 变化" text unless
  the repeated text is still true and explicitly relevant to this version.

## Required Assets

Every GitHub Release must contain:

```text
udf.exe
unrealdevflow-x86_64-pc-windows-msvc.zip
unrealdevflow-installer.ps1
SHA256SUMS.txt
RELEASE_NOTES.md
```

## Workflow

1. Read `Cargo.toml` and determine the target version.
2. If the user did not specify a version, infer a patch/minor/major candidate and state it.
3. Update `Cargo.toml` version only when the user explicitly asked to prepare that release version.
4. Run:

```powershell
pwsh scripts/release-preflight.ps1 -Strict
```

5. Create curated release notes source:

```powershell
pwsh scripts/generate-release-notes.ps1 -Version <version> -OutputPath dist/RELEASE_NOTES.draft.md -AllowGeneratedDraft
```

Then write `docs/releases/v<version>.md` by reading:

- `git log <previous-tag>..HEAD --oneline`
- `git diff --stat <previous-tag>..HEAD`
- the key changed files behind those commits

The notes must say what changed, why users care, and which agent workflow
changed. Commit dumps alone are not acceptable.

6. Generate validated release notes:

```powershell
pwsh scripts/generate-release-notes.ps1 -Version <version> -OutputPath dist/RELEASE_NOTES.md
```

7. Package assets:

```powershell
pwsh scripts/package-release.ps1 -Version <version>
```

8. Inspect `dist/SHA256SUMS.txt` and verify all required assets exist.
9. Commit release preparation changes if any tracked files changed.
10. Tag and push:

```powershell
git tag v<version>
git push origin v<version>
```

11. Wait for the GitHub Release workflow to finish.
12. Check the draft release assets and release body.
13. Smoke-test the installer from the draft/latest release URL when available:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pb763396199/UnrealDevFlow/releases/latest/download/unrealdevflow-installer.ps1 | iex"
```

14. Only after assets, release body, and installer are correct, publish the draft release.

## Release Notes Shape

Use `docs/releases/v<version>.md` as the source of truth. The generator refuses
to create formal release notes when this curated source is missing.

Required sections:

- 一句话总结
- 安装 / 升级
- 重点变化
- 新增
- 修复
- 破坏性变更
- AI Agent 变化
- 校验
- 变更列表

Quality bar:

- The summary must name the main product value in this version.
- `新增` only lists features introduced in this version.
- `修复` only lists bugs fixed in this version; write `无。` when empty.
- `AI Agent 变化` only lists agent-facing workflow/skill/doc changes.
- `变更列表` may include commit subjects, but it cannot be the only useful
  part of the notes.
- Before publishing, compare the notes with the previous release and remove any
  repeated boilerplate.

## If Something Fails

- Preflight failure: fix code or tests first, then rerun the full preflight.
- Packaging failure: fix packaging script or missing files, then rerun package.
- Release workflow failure: inspect GitHub Actions logs, fix, retag only if the failed tag never produced a valid release.
- Installer failure: do not publish. Fix installer, cut a new release candidate or replace draft assets before publishing.
