---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01M1QPYQQK9SMGH5B482XV80TW
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T02:47:07Z
producer: aes-review
review_type: design
verdict: approved
blocking_findings: []
reviewers:
  - "Codex native subagent runtime_design_critique"
supersedes: ar_01M1QM97JZX2KDWQ1ZKFVH5E12
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts:
    - artifact_id: ar_01M1QPYQJSMRQ3VT6JK5E65CVM
      digest: sha256:253b56514c88f3db86af66b9047e6b75db8e7944244adae89ace39db12e22cf0
      locator: design/run-native-tools-design-v3.md
---

# 原生工具优先设计审查

## 结论

approved。独立子代理只读审查，主会话修订文档并保存本记录。两项阻断修正后复核通过，最后新增的六项案例映射也已单独核对。
该结论只批准设计质量，不代表用户接受全部细节，也不代表 CLI 或 Udf.EditorExit 已实现。

审查范围为当前 work-item、v3 设计、原生工具调查及两个 v3 JSON 示例。v2 审查保留为历史记录，不作为当前设计的批准依据。

## 已修正的问题

| 问题 | 依据与后果 | 已核对的修正 |
| --- | --- | --- |
| Automation 配置没有过滤参数 | UE.Automation.cs:167 缺少 RunTest 会抛异常 | 添加 runTest 到原生 -RunTest 的映射，给出示例；零匹配不通过 |
| strict 退出只导出原始码，未明确判定 | Gauntlet 的 RequestedExit 可归一化为成功 | 自然终止、非强杀、原始码 0、无原生致命失败才通过；缺码 unknown，非零 failed |
| 已有 Editor 路线不够具体 | 可能误开第二个实例或把启动地图当当前地图 | 只读 plan 路线给 PID、目标、人工 actions；未知地图为 null |
| 原生实时输出可能污染 JSON | 当前全局格式约定只输出一份 JSON 文档 | JSON 模式原生日志去文件或 stderr |

## 已核实的事实

v3 将启动和测试执行交给 Editor CLI、Commandlet、UAT/Gauntlet，删除 v2 的通用监督器、完整快照和状态准备 DSL。
普通启动只报告 started；测试失败与 unknown 返回非零；check/plan 不启动 UE。
退出测试先验证原生 EditorBootTest，只有已确认的缺口才进入最小测试节点扩展。
两次运行仅作观察对照，不自动证明修复因果。原始关卡、Git Bash 转换、窗口大小及既有 Editor 案例均有对应设计措施。

依赖头部使用 AES 规范化 artifact digest；下表为独立文件的原始 SHA-256，两者算法输入不同。

| 文件 | 原始 SHA-256 |
| --- | --- |
| run-native-tools-design-v3.md | `306b8efe34ca5a270f762472adfa9910c56cad1972799de3d32bb517eea62212` |
| editor-exit-native-v3.json | `1c96dc0b206a09a6b598a519386900d7287ed20de024a0216da7644a2c9cd961` |
| perflab-game-native-v3.json | `dcbb1ff8d5fb36742d5f86457f9afca735a4d0bdab20e44f7bd2113cab0fff42` |

两个 JSON 已解析并检查目标、backend、地图、触发命令和窗口参数与正文一致。候选配置尚未由 UDF 执行。
本轮再次查询远端 dev，与任务 HEAD 都是 `2a516b50dfb8b3d58e5a7962b7c0705c84ba78ce`。

## 未覆盖的范围

没有执行 UE、UAT 或 Cargo 测试，没有修改生产代码。
原生程序集加载、Cmd 与 Editor 外壳差异、原始进程状态获取及三种 Windows shell 的实收参数，仍须在实施阶段按设计验证。
这些是明确保留的实施验证项，不构成要求恢复 v2 框架的理由。
