---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01M201Z5Y9RNEH8DWME72PJD4S
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-08T15:35:00+08:00
producer: aes-validate
result: passed
supersedes: ar_01M1ZXVPNJM38DZYR1P1VMF72R
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts:
    - artifact_id: ar_01M201Z63CWYPKJ2DDRX7HAD0H
      digest: sha256:5f70b4d8b6b3b90d07e1756a67449cb34ae5842d48519d878a31babe8cb04c61
      locator: reviews/code-review.md
    - artifact_id: ar_01M201Z5S9Z68CST1SDEHDPFGR
      digest: sha256:08151e31596b3c06d9790e555f0b3870dfc43aefb88f9523e40cd5809c10775e
      locator: implementation.md
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
# 人工接手清单

这些步骤涉及真实机器目录和用户当前运行状态，自动测试不能代替人工确认。清理前只读，清理后再次只读复核。

## AC-001

<!-- AC-001 -->

- [X] 用 release 二进制运行 `udf --format json package clean --dry-run`，核对 `totalBytes`、`reclaimableBytes`、`protectedBytes`、`unknownBytes` 与调查记录；确认 `%TEMP%\UDF`、`%USERPROFILE%\.unrealdevflow\package`、日志、backup 和最终包都列出。

## AC-006

<!-- AC-006 -->

- [X] 用 `udf --format json package clean --dry-run --workspace neon-dev` 查看 workspace/task 归属；打开一条项目 execution 和一条插件 execution，确认 task execution 有 `taskRef`，旧记录显示 legacy/unknown，JSON 中没有猜测的 AES Work Item 绑定。

## AC-008

<!-- AC-008 -->

- [X] 确认没有 `udf.exe`、UAT、UBT、UnrealEditor-Cmd 或 cl.exe 命令行引用待删目录；运行 `package clean --legacy --yes` 再运行 `package clean --stale --yes`，最后核对当前 cache、profiles、execution JSON、`C:\Package` manifest 和 C 盘可用空间。

## 通过标准

人工确认三条都没有遗漏后，将 `[ ]` 改为 `[x]` 并记录命令输出路径；任一目录仍被使用或最终包摘要变化时标为 `[!]`，不要继续删除。
