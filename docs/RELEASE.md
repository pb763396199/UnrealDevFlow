# UnrealDevFlow 发布规范

本文是 UnrealDevFlow 的发布源规范。发布不是打一个 tag，而是固定执行：

```text
preflight 通过 -> release notes 生成 -> tag 触发 CI -> 产物齐全 -> installer 可用 -> draft release 检查 -> publish
```

## 发布入口

普通用户安装命令固定为：

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pb763396199/UnrealDevFlow/releases/latest/download/unrealdevflow-installer.ps1 | iex"
```

这个入口必须满足：

- 不要求用户安装 Rust。
- 不要求用户 clone 仓库。
- 自动下载 GitHub Release zip。
- 安装到 `%USERPROFILE%\.unrealdevflow\bin`。
- 写入 User PATH，并刷新当前 PowerShell 会话 PATH。
- 安装全局 AI skill。
- 验证 `unrealdevflow --version` 和 `Get-Command unrealdevflow`。

## 发布前检查

每次发布前必须运行：

```powershell
pwsh scripts/release-preflight.ps1 -Strict
```

这个脚本必须通过：

- git 工作区干净。
- `Cargo.toml` 版本合法。
- 发布支持文件齐全。
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --all-targets --all-features --locked`
- `cargo build --release --locked`
- `target/release/unrealdevflow.exe --version`

任一失败都禁止发布。

## 准备版本

1. 更新 `Cargo.toml` 的 `version`。
2. 运行 preflight。
3. 生成 release notes：

```powershell
pwsh scripts/generate-release-notes.ps1 -Version 0.1.0 -OutputPath dist/RELEASE_NOTES.md
```

4. 本地打包 smoke test：

```powershell
pwsh scripts/package-release.ps1 -Version 0.1.0
```

5. 检查 `dist/` 中必须存在：

```text
unrealdevflow.exe
unrealdevflow-x86_64-pc-windows-msvc.zip
unrealdevflow-installer.ps1
SHA256SUMS.txt
RELEASE_NOTES.md
```

## 打 tag 发布

```powershell
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会创建 draft release 并上传资产。发布者必须检查：

- Release 是 draft。
- 资产齐全。
- `SHA256SUMS.txt` 包含 exe、zip、installer、release notes。
- release notes 有安装命令、变更、校验项。
- 下载安装命令可以在干净 PowerShell 中执行。

确认后再手动 publish draft release。

## Release Notes 模板

每次 release notes 必须包含：

- 一句话总结。
- 安装 / 升级命令。
- 新增。
- 修复。
- 破坏性变更。
- AI Agent 变化。
- 校验。
- 变更列表。

可以用 GitHub 自动 release notes 补充 PR/commit 信息，但不能替代这份产品说明。

## 禁止事项

- 禁止跳过 `release-preflight.ps1`。
- 禁止 clippy warning 存在时发布。
- 禁止只上传 exe 而不上传 installer/zip/checksum。
- 禁止发布非 draft release 后再补资产。
- 禁止让用户通过 clone + cargo build 作为默认安装方式。
- 禁止让 AI 凭记忆手写发布流程。

