---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01M1K296MG12RFN2Y3V9GAQK2J
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-03T06:30:00Z
producer: aes-validate
result: passed
supersedes: ar_01M1BGHDGGQFC8DFGW80YYC0T9
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts: []
  subject:
    kind: change_set
    digest: sha256:4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: a817dd2473ebe5fa18ceb0168be9692bf905fcea
    revision: 2d76b395291f8b79b8ba447409a5b6d4dd402c26
    tree: bb896285024e07a65b6c43696026c79e104573f6
    content_digest: sha256:e402fec3b80a318be4aeab9bb0ec3ee423eed114772f8d290b379b9b07dd09ac
    branch_or_pr: feature/task-package-profiles
    workflow_excluded: true
---

# 人工验收清单

当前共有 11 条，预计 30 至 60 分钟。请在真实 UE5.5 Windows Win64 Game 工程上按顺序执行；不要把此前用户手工生成的 `C:\Package\Windows` 包当作本次工具结果。

## 设计阶段复核

以下 6 条是原任务在设计阶段留下的人工核对项，本次施工没有替人勾选：

<!-- AC-001 -->
- [x] 核对调查中的日志路径、错误计数和三项用户更正，确认事实与推断分开。

<!-- AC-002 -->
- [x] 检查 Wayfinder 的每张调查票均有证据和结论，地图没有遗漏问题。

<!-- AC-003 -->
- [x] 阅读设计中的配置生命周期和命令示例，确认固定配置、更新原因及来源绑定完整。

<!-- AC-004 -->
- [x] 逐项核对正常执行、失败、中断和来源变化场景，确认保护与恢复边界清楚。

<!-- AC-005 -->
- [x] 核对文件影响范围、兼容表和验证矩阵，确认未实测能力没有写成已通过。

<!-- AC-006 -->
- [x] 检查本任务代码变更、施工记录和记录校验结果，确认本次进入施工的范围有迹可查。

<!-- AC-007 -->
- [x] 为一个实际 task 执行 `udf package configure --task <task-ref> --configuration Shipping --container loose --output <固定目录> --disable-plugin ModelContextProtocol --disable-plugin AllToolsets --reason "验证 Shipping"`，确认只改 task 的 `.udf-package.toml`，主项目和 Host `.uproject` 不变。

<!-- AC-008 -->
- [x] 执行 `udf package plan project --task <task-ref>`，确认显示 Shipping、loose、固定输出目录，并确认命令没有 `-skipcookingeditorcontent`、没有 `-pak`，也没有自由注入的 UAT 参数。

<!-- AC-009 -->
- [x] 执行 `udf package project --task <task-ref>`，等待 UAT 完整 Cook 结束，确认退出码为 0，最终包在配置的固定目录，并确认被禁用的 MCP 插件未参与此次项目副本打包。

<!-- AC-010 -->
- [x] 在固定输出目录预先放入一个用户存档和一个外部 Junction，再次执行打包，确认用户存档未被覆盖，Junction 未被遍历、删除或改指向。

<!-- AC-011 -->
- [x] 在交付复制阶段人为中断一次进程，然后执行 `udf package recover <execution-id>`，确认旧文件恢复、新文件按清单撤销；如果发现目录已有外部修改，工具应拒绝恢复并列出冲突，不应强制覆盖。

## 记录方式

每条通过后将 `[ ]` 改为 `[x]`；失败改为 `[!]`，并在下一行写看到的现象、execution ID 和日志路径。未完成前不要把本任务标为验收通过。
