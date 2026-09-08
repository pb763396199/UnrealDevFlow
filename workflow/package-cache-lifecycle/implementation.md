---
schema_version: 1
protocol: 1.3.0
artifact: implementation
artifact_id: ar_01M201Z5S9Z68CST1SDEHDPFGR
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-08T15:33:00+08:00
producer: aes-execute
result: complete
supersedes: ar_01M1ZXMGKNRKQ5AH1RYJYP9FR3
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts:
    - artifact_id: ar_01M1Y6N5EPKA8HMNC9E6JGKQS4
      digest: sha256:6eae68193a7ce7c4595f85f1ae2d3658093e6e8829e966d0a212acdc06dd9907
      locator: plan.md
    - artifact_id: ar_01M201Z5M9J9Y4FRF0GV717QQR
      digest: sha256:ee01804fcb5ce81760cb6b4d86b63543ea6aee8399d0df22369f5587dbfda984
      locator: change-note.md
  subject:
    kind: change_set
    digest: sha256:60d56a644472396b2f4495673b45ebb362aa4069f4af3537fa2c9f0eae2e90c5
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 284525b422739bd6b2689bf1e7aca9c08fc1835a
    revision: b8bede7f7c4fbdfd3cdff714040a854fa7e68f5e
    tree: b5718aa1f6f34ec823fc4c9c1c6af9b468650a71
    content_digest: sha256:60d56a644472396b2f4495673b45ebb362aa4069f4af3537fa2c9f0eae2e90c5
    branch_or_pr: fix/package-cache-lifecycle
    workflow_excluded: true
---

# 实现记录

## 已实现

| 范围 | 结果 |
| --- | --- |
| 稳定缓存槽 | `CacheIdentity` 只使用 task/workspace、项目、引擎、平台、配置和容器；旧 revision 槽可在执行时按摘要迁移 |
| 活动保护 | Cook cache 使用排他目录锁和 PID/进程启动时间 lease；Windows inventory 额外读取 CIM 命令行，识别 UAT/UBT/编译子进程 |
| 统一盘点 | 扫描固定 UDF 根、execution cleanupTargets、日志、交付 backup、profile、record 和带 manifest 的输出；Junction 不递归，父子路径不重复计数 |
| 清理入口 | `package clean` 支持 `--cache`、`--stale`、`--legacy`、`--yes`；无 scope 只读，范围删除缺少 `--yes` 自动 dry-run，逐条更新 execution JSON |
| 复用与交付 | overlay/禁用插件摘要参与 iterate 复用判断；schema 2 状态保留执行和来源摘要；交付逐文件核对 digest 后标记 delivered、删除 Archive/backup，失败保留 recover journal |
| 空间预检 | global preflight 统计历史 Cook cache、当前 staging、源输入估算、磁盘保留线和 cache cap；输出历史占用和清理诊断 |
| 文档 | README、UnrealDevFlow Skill 和 v0.5.0 release note 同步新命令、保护边界和 UDF/AES 身份说明 |
| 插件 staging | 私有插件根只复制根文件和已有 Binaries，Source、Content、Config、Resources、Shaders、ThirdParty 改为 Junction；`Intermediate/Binaries/Saved` 留在 stage 内；交付成功后删除整个 stage |
| 失败现场的直接删除 | `package clean <execution-id> --yes --force` 只删除该条 execution 的受管失败 stage；活动 stage 继续拒绝；删除后 execution JSON 记录为 `cleaned` |

## 验证

- `cargo fmt --check`：通过。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo test --locked --all-targets`：通过，94 个单测以及全部集成测试通过。
- `cargo build --release --locked`：通过，产物 `target/release/udf.exe`。
- `target/release/udf.exe --version`：`udf 0.5.0 (git 07f99f873-dirty, built 2026-09-07T15:18:25Z)`。
- `plugin_stage_uses_read_only_junctions_and_private_generated_dirs`：通过。Source、Content、Config、Resources、Shaders、ThirdParty 是 Junction；Intermediate、Binaries、Saved 是私有目录；预编译 Binaries 会复制进私有目录。
- `successful_plugin_delivery_removes_private_stage`：通过。成功路径删除 stage，并从 cleanupTargets 移除该路径。
- `tests/skills/aes-workflow/test_package_cache_lifecycle.py` 两条新增 wrapper：通过。
- `force_cleanup_of_an_explicit_execution_removes_failed_diagnostic_stage`：通过。无 force 保留失败现场；加 `--yes --force` 后删除 stage 并更新 execution。
- 真实目录 dry-run 已验证：最终输出被列为 protected，当前缓存 `c20e7e7f47adb0d3d0db3788778b708d` 被识别并保留；活动 staging `C:\Users\YUMEI\AppData\Local\Temp\UDF\715231601b5f` 被识别为 active。

## 实际清理前提

用户已选择 B，允许删除旧 successful staging、旧 Archive/已交付 backup 和历史缓存，不建立恢复副本。旧版全局 `udf.exe` 已产生的最近 failed/running stage 仍遵守 24 小时保护期；新版本二进制在成功插件交付后不会再留下同类完整插件副本。

用户随后明确要求立即删除三份失败现场。运行中进程检查为零后，三条 execution 均由受管 force 入口删除，`%TEMP%\\UDF` 的 execution stage 降为 0 GiB，三条 JSON 记录均为 `cleaned`。

## 哪里没按计划走

实现保留了旧的 `package clean <execution-id>` 入口，并把新的 cache、stale、legacy 清理合并到同一份 inventory；没有新增独立的 `package cache clean` 命令。真实机器的第一轮 dry-run 发现旧 execution `artifacts` 会指向 UE 引擎目录，随后收紧为只认 cleanupTargets、outputs 和 logs；发现后立即修正并重新通过全量测试。

| 做了什么 | 计划里有吗 | 不做它验收标准能达成吗 |
| --- | --- | --- |
| 保留预编译 Binaries 的私有复制 | 有 | 不能。直接把 Binaries 也做成 Junction 可能让 UBT 写回源插件，或丢失预编译 DLL。 |
