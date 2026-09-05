---
schema_version: 1
protocol: 1.3.0
artifact: change-note
artifact_id: ar_01M1R037AQKA7EH2JC1880FZGM
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T05:19:26Z
producer: aes-execute
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts:
    - artifact_id: ar_01M1QW61K3PD4D6D4YDB1X8GC5
      digest: sha256:7f91ee5ee927c680f1ba116f5ea1518dde116db2816abee1b485706018a874b8
      locator: plan.md
    - artifact_id: ar_01M1QPYQJSMRQ3VT6JK5E65CVM
      digest: sha256:253b56514c88f3db86af66b9047e6b75db8e7944244adae89ace39db12e22cf0
      locator: design/run-native-tools-design-v3.md
    - artifact_id: ar_01M1QXNMGGPM9QVQR8WM2RFFYC
      digest: sha256:cdac13098ed3372e3c7d8d622e833766f8bcea8843c5961b56336f53b3d81fb7
      locator: research/native-probe-research.md
  subject:
    kind: change_set
    digest: sha256:fdb7027fe71981b5fcba9d7a45c06c933cb8524dda7a1c1894c189dc1cea4cb4
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 2a516b50dfb8b3d58e5a7962b7c0705c84ba78ce
    revision: 6097f4187a766476eae1231eaa8eab0e6499c4e1
    tree: e1760916841fb034c5b04c1406e99b1dec639dd1
    content_digest: sha256:8d547cc41dda5b6ba67dfed8d70b61ce82e5d2145311d05f6a64671b68481fa4
    branch_or_pr: feature/ue-test-run-cli
    workflow_excluded: true
---

# 修改说明

## 覆盖账本

| 改动 | 解决的问题 | 依据 |
| --- | --- | --- |
| `udf run` 七个叶子命令 | 新会话不再手填项目、地图和 UAT 参数 | v3 设计、施工计划 P3/P4 |
| `run_profile` 配置与原子保存 | 配置可跨会话复用，修改和并发更新可追溯 | 施工计划 P2 |
| Gauntlet 资源与严格退出报告 | 退出结果同时保留 UAT 结果和原始 UE 退出码 | P0 探针、施工计划 P1 |
| execution 记录、status、compare | 失败数据、缺终态和前后对照可被机器消费 | v3 结果契约 |
| shell/Host/回归测试与文档 | 证明真实参数边界，并让接手会话有短路径 | AC-013 至 AC-025 |

## 修改理由

1. UDF 只做作用域绑定、参数数组和证据关联。UE 原生 Editor、Commandlet、RunUAT/Gauntlet 继续负责测试发现、进程监督和业务断言。
2. 配置名指向一份完整 JSON。`task` 优先使用任务文件，缺失时才继承 workspace 文件；不合并字段，不根据最近会话猜目标。
3. `check` 和 `plan` 不启动 UE。`start` 才创建独立 execution 目录；失败、unknown 和 started 都保留可核对数据。
4. `Udf.EditorExit` 使用 `UdfSyncCmds` 让同步命令排在节点唯一的 `QUIT_EDITOR` 前，避免 UAT 普通 `ExecCmds` 的追加顺序改变退出语义。

## 代码范围

生产代码为 `src/cli.rs`、`src/main.rs`、`src/output.rs`、`src/run_profile.rs`、`src/commands/run.rs` 和模块注册；原生测试资源在 `resources/gauntlet/`；测试夹具、验收脚本和五份用户文档同步更新。

## 未解释或例外改动

没有扩大到环境快照、后台守护、通用断言 DSL、UE 控制接口或替代 Gauntlet 的监督器。真实 Host 使用计划创建的专用 `neon-dev/udf-run-acceptance`，旧的 `earthmodeler-exit-crash` Host 已不存在，因此没有伪造旧场景等价性。
