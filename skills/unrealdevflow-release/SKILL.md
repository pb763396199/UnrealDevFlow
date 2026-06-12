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

## Required Assets

Every GitHub Release must contain:

```text
unrealdevflow.exe
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

5. Generate release notes:

```powershell
pwsh scripts/generate-release-notes.ps1 -Version <version> -OutputPath dist/RELEASE_NOTES.md
```

6. Package assets:

```powershell
pwsh scripts/package-release.ps1 -Version <version>
```

7. Inspect `dist/SHA256SUMS.txt` and verify all required assets exist.
8. Commit release preparation changes if any tracked files changed.
9. Tag and push:

```powershell
git tag v<version>
git push origin v<version>
```

10. Wait for the GitHub Release workflow to finish.
11. Check the draft release assets.
12. Smoke-test the installer from the draft/latest release URL when available:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pb763396199/UnrealDevFlow/releases/latest/download/unrealdevflow-installer.ps1 | iex"
```

13. Only after assets and installer are correct, publish the draft release.

## Release Notes Shape

Use the template produced by `scripts/generate-release-notes.ps1`. Do not replace it with a raw commit dump.

Required sections:

- 一句话总结
- 安装 / 升级
- 新增
- 修复
- 破坏性变更
- AI Agent 变化
- 校验
- 变更列表

## If Something Fails

- Preflight failure: fix code or tests first, then rerun the full preflight.
- Packaging failure: fix packaging script or missing files, then rerun package.
- Release workflow failure: inspect GitHub Actions logs, fix, retag only if the failed tag never produced a valid release.
- Installer failure: do not publish. Fix installer, cut a new release candidate or replace draft assets before publishing.

