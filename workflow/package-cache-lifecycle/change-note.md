---
schema_version: 1
protocol: 1.3.0
artifact: change-note
artifact_id: ar_01M201Z5M9J9Y4FRF0GV717QQR
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-08T15:32:00+08:00
producer: aes-execute
result: complete
supersedes: ar_01M1ZXMGE78ZM6CS8EPCFZKW0H
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts:
    - artifact_id: ar_01M1Y6N5EPKA8HMNC9E6JGKQS4
      digest: sha256:6eae68193a7ce7c4595f85f1ae2d3658093e6e8829e966d0a212acdc06dd9907
      locator: plan.md
    - artifact_id: ar_01M1Y6N59V3BEHVPA4V0V2ZT57
      digest: sha256:e0d82755fc5a8d13b890078410c7572bc8f6a7839ed2bf1da644ab3cb800372b
      locator: design/plugin-stage-space-design.md
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

# 修改说明

## 覆盖账本

| 步骤 | 范围 | 结果 |
| --- | --- | --- |
| S1 | package lifecycle、CLI 和缓存/盘点单测 | 先锁定 inventory、稳定槽和租约的回归边界 |
| S2 | `src/package_cache.rs`、`src/commands/package.rs`、`src/package_storage.rs` | profile revision 不再制造新槽；Cook 槽支持 schema 2 状态和 PID/启动时间租约 |
| S3-S4 | `src/package_inventory.rs`、`package clean` 入口 | 统一扫描临时 staging、cook-cache、日志、事务备份、profile、record 和最终输出；范围删除改为 `--yes` 显式确认 |
| S5 | 项目/插件交付和复用 | overlay 摘要参与复用判断；交付摘要校验通过后删除 backup |
| S12-S13 | 插件 staging | Source、Content、Config、Resources、Shaders、ThirdParty 改为 Junction；Binaries 保留私有副本；成功交付后删除 stage |
| S16 | 失败 stage 的显式删除 | 新增指定 execution 的 `--yes --force`；活动任务仍受保护；清理进程不会误判自身为活动打包 |

## 修改理由

- 目录 key 只保留 task、项目、引擎、平台、配置和容器，避免 profile revision、包名和输出路径导致百 GB 复制。
- `.udf-cook-cache-lease.json` 同时保存 PID 与进程启动时间，清理不会把仍在运行的 Cook 当成 stale。
- inventory 只扫描固定 UDF 根和 execution 已记录路径，最终包必须有 manifest/执行记录才会列入，而且永远只列保护项。
- 无参数 clean 继续是只读盘点；范围清理缺少 `--yes` 时退回 dry-run，旧记录必须通过 legacy 证据链才可处理。
- 交付完成后先逐文件核对摘要，再删除 backup；复制中断仍保留 delivering journal 和 backup 供 recover。
- 插件包原先完整复制插件闭包，六次执行会复制六份 Content 和 Source。现在每次只建立私有插件根、生成目录和必要根文件，大目录留在源插件。
- 成功交付后的插件 stage 没有任何恢复用途。删 stage 前仍完成 manifest 与逐文件摘要校验；失败路径不进入删除分支。
- 失败现场默认保留 24 小时。用户明确要求删除时，只能指定一条 execution 并同时提供 `--yes --force`；最终输出、普通工程目录和活动 stage 仍不能成为目标。

## 当前阶段

S1-S5 已完成并通过对应单元/集成测试；S6 空间规则已接入 global preflight。后续继续补全测试、中文命令文档、release 构建和真实机器 dry-run，再按用户选择 B 执行删除。

## 例外和风险

真实机器的 package inventory 会读取约 1 TB 受管数据，dry-run 可能耗时；`C:\Package` manifest、当前 profile 槽和没有 UDF 证据的 UE 共用 `Intermediate/Saved` 仍被保护。用户选择 B 后旧 staging/缓存属于可再生数据，不建立恢复副本。预编译 DLL 可能位于插件 Binaries，因此它保留为私有副本，不能直接改成 Junction。
