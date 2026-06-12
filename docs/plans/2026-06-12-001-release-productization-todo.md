# UnrealDevFlow 产品化发布 TODO

> 目标：先让 UnrealDevFlow 成为可以被普通用户一条命令安装和升级的软件，再继续设计 `init/start/next` 之类的新手工作流。

## 已定方案

- [x] 首选分发链路：GitHub Release + Windows 预编译 exe + PowerShell installer。
- [x] 不把 npm 作为第一入口；npm 以后只作为下载 GitHub Release exe 的 wrapper。
- [x] 不要求用户安装 Rust、clone 仓库、手动 `cargo build`。
- [x] installer 必须修复 PATH 当前会话不可见的问题。
- [x] installer 必须安装 AI skill，并验证 `unrealdevflow` 命令可用。
- [x] 发布流程必须标准化为脚本 + CI + AGENTS/skill，而不是靠人工记忆。
- [x] 发布前强制执行 `cargo fmt`、`cargo clippy -- -D warnings`、`cargo test`、release build。

## 执行清单

- [x] 新增本 TODO 文档，作为本轮产品化发布工作的落盘清单。
- [x] 新增 `.github/workflows/ci.yml`，在 PR/main push 上运行 Rust 质量门禁。
- [x] 新增 `.github/workflows/release.yml`，在 `v*.*.*` tag 上构建并发布 Release 资产。
- [x] 新增 `.github/release.yml`，规范 GitHub 自动 release notes 分类。
- [x] 改造 `scripts/install.ps1`，默认从 GitHub Release 下载 zip，不再默认源码编译。
- [x] 新增 `scripts/release-preflight.ps1`，统一发布前本地/CI 检查。
- [x] 新增 `scripts/package-release.ps1`，统一打包 exe、skill、installer、checksum。
- [x] 新增 `scripts/generate-release-notes.ps1`，统一 release notes 模板。
- [x] 新增 `docs/RELEASE.md`，写清人类发布流程。
- [x] 新增 `skills/unrealdevflow-release/SKILL.md`，写清 AI 发布流程。
- [x] 更新 `AGENTS.md`，要求发布任务必须走 release skill 和 preflight。
- [x] 更新 `README.md`，把安装入口改为一条 PowerShell 命令。
- [x] 修复 CLI 的 `skills install` 不能在未配置前运行的问题。
- [x] 运行 `cargo fmt`。
- [x] 运行 `scripts/release-preflight.ps1` 验证门禁。

## 后续阶段

- [ ] 发布第一个 GitHub Release 后，提交 Winget manifest。
- [ ] 视用户群体决定是否补 Scoop/Chocolatey。
- [ ] npm wrapper 仅在有 Node 用户需求后再做。
- [ ] 安装链路稳定后，再实现 `init/start/next/finish` 小白命令。
