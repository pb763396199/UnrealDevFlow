---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01M1QM97JZX2KDWQ1ZKFVH5E12
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T02:00:05Z
producer: aes-review
review_type: design
verdict: approved
blocking_findings: []
reviewers:
  - "Codex native subagent runtime_design_critique"
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts:
    - artifact_id: ar_01M1QM97EAQG8PYJ6ZQESV3P8S
      digest: sha256:a891c400562635bb4f47a1d3a82305ed974a3d50359f7632c193340a47bc5ea4
      locator: design/run-verification-design-v2.md
---

# 运行与退出验证设计审查

## 结论

独立子代理只读审查，主会话负责修订和保存本记录。最终设计层结论为 approved。
本审查不表示用户已接受方案，不表示 CLI 已实现，也不证明 AesWorld 主项目退出问题已通过回归。

审查对象为 [v2 设计](../design/run-verification-design-v2.md)，依据当前 `dev=2a516b5` 和两组真实日志。
JSON 示例与正文同步审查，其 SHA-256 为：

| 文件 | SHA-256 |
| --- | --- |
| earthmodeler-exit-smoke.json | `7F26127875212B0D8F9303B28607734DBF86163C0B584DD808325A9B79F30403` |
| business-pass-exit-crash.json | `D46B608549D82FCAC8577E252C5930D0755692C264DBF9D0D0207639D59CD12C` |

## 已修正的问题

| 问题 | 后果 | 当前修正 |
| --- | --- | --- |
| 报告先于进程退出就通过 | 漏掉 OnPreExit 崩溃 | 业务与退出分开，收完终态证据再判定 |
| 事后靠 PID 找退出码 | 进程消失或 PID 复用被判成功 | 监督器持有句柄，丢失观察证据为 inconclusive |
| 异步命令紧跟 Quit | 未执行到目标路径就退出 | 有限退出策略与完成条件，能力不足明确拒绝 |
| mtime/公共 Crash 目录串证据 | 旧文件造成假通过或假失败 | execution 隔离与进程关联 |
| 相同名字即可前后比较 | 不同版本环境或错误基线变成修复证明 | 明确允许变化项、前提和目标故障签名 |
| check 自动启动发现进程 | 违反现有只读语义 | 显式 discover --refresh，check 使用已有事实 |
| JSON 成功与 CI 成功混用 | 测试失败但 CI 绿灯 | start/wait/compare 非零表示未达期望，status 保留查询语义 |
| Cmd/NullRHI/Commandlet 混用 | 绕过真正的 Editor 退出路径 | 外壳、模式与 RHI 分开 |

## 新版复核时修正的两项不一致

- 主文关于 smoke 的业务结果与 JSON 中 dispatch-only 不一致。现在 workload.kind=startup-smoke，触发命令保留，actionResult=not_applicable，退出通过只代表声明的覆盖。
- 持久 phase 与签名中的 shutdown 没有映射。现在后台写 starting/pending，故障另用 lifecycleStage 分类；缺失时序为 unknown，不用 finished 匹配退出崩溃。

复核也确认 C 阶段先实现已有执行记录的比较，E 阶段补历史导入与重复运行，实施依赖不成环。

## 已核实事实与剩余范围

主会话重新计算 baseline/after 两份日志摘要，与调查记录一致。baseline 有 Critical error 与 access violation，after 没有这些致命标记但保留 74 行 Error。
实际安装版帮助已确认 v0.5.0 的 configure/check/plan 约定与 package run 移除。
两个 JSON 示例通过语法和业务成功/退出失败关系检查，这些检查不属于实现测试。

UE 5.5.4 的退出/Automation 队列解析、跨 shell 参数、状态准备、模块加载身份和证据完整性，仍需按设计执行实现探针与真机验收。
历史记录缺少主项目修复后运行、二进制摘要和完整 Details 预置状态，设计已保留这些覆盖缺口。
