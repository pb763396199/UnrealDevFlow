---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01M1QWFBQX9PGFQCAKPTXGXCKN
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T04:18:59Z
producer: aes-review
review_type: plan
verdict: approved
blocking_findings: []
reviewers:
  - "Codex native subagent runtime_design_critique"
supersedes: ar_01M1QPYQQK9SMGH5B482XV80TW
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts:
    - artifact_id: ar_01M1QW61K3PD4D6D4YDB1X8GC5
      digest: sha256:7f91ee5ee927c680f1ba116f5ea1518dde116db2816abee1b485706018a874b8
      locator: plan.md
    - artifact_id: ar_01M1QPYQJSMRQ3VT6JK5E65CVM
      digest: sha256:253b56514c88f3db86af66b9047e6b75db8e7944244adae89ace39db12e22cf0
      locator: design/run-native-tools-design-v3.md
  subject:
    kind: change_set
    digest: sha256:4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 2a516b50dfb8b3d58e5a7962b7c0705c84ba78ce
    revision: 2a516b50dfb8b3d58e5a7962b7c0705c84ba78ce
    tree: b1c39934a486007b2d01438bceaef9a2e77ba66d
    content_digest: sha256:0cb859cb07508b5eb70c9a2b29a62d57ab010bfca35757735700a3b3824d94dc
    branch_or_pr: feature/ue-test-run-cli
    workflow_excluded: true
---

# 施工计划与验收标准审查

## 结论与范围

approved。独立子代理只读审查，主会话负责修订和保存记录。审查对象为已认可 v3、P0 至 P7 施工计划及 AC-013 至 AC-025 产品验收标准。
本次批准计划可以进入施工，不代表任何产品测试通过。下一步为 aes-execute 的 P0；不能因为存在这份评审就跳到产品验收或收口。

生产变更集为空，头部代码身份只证明本轮未改生产代码。实际审查版本由 plan 和 design 的 Artifact 摘要绑定。旧版审查保留，不修改它们来冒充覆盖新增验收标准。

## 已解决的问题

| 发现 | 对施工的影响 | 最终修正 |
| --- | --- | --- |
| P1 要求尚未实现的单 exe CLI 实跑 | 前置验收依赖后续 P4，形成循环 | P1 只验原生编译与加载；单 exe 验收移到 P6 |
| P2 要求 P3 才有的 configure/list/plan | 配置逻辑无法独立交付 | P2 模块单测；P3 跨进程查询；P4 补 start 复用 |
| P2 模块尚未登记到 main | 可能 0 个测试却误认为通过 | 提前登记模块，验实际发现和执行数量 |
| P6 实跑配置到 P7 才登记 | 实机验收没有输入 | P6 前置登记四份配置，P7 只复用已验证配置 |
| 三次 Game 运行没有轮间退出 | 上一实例阻止下一轮启动 | 只清理本次已核实 PID，等待退出；清理不作自然退出证据 |
| 直接 Cmd 探针没有有界等待 | 失败时可能遗留测试实例 | 120 秒后只清理本次实例，保存 timeout |
| 历史 Host 不存在 | 旧示例不能直接运行 | 创建明确的新验收 task，不声称还原旧场景 |

## 已核对的验收要求

产品标准有 13 条，每条均定义测试位置和证据；退出、CLI 返回与前后对照另有逐行判定表。
原始 UE 非零、Fatal、强杀与零匹配 Automation 都不能通过；缺失终态为 unknown；普通启动只报告 started。
真实 Host、三个 shell 和独立接手是完成门槛；单元测试、旧日志和空报告不能替代它们。
接手者可以是未继承历史的独立代理，不额外要求只能由人操作；最终人工确认沿用仓库流程。

UDF 源码基线及远端 dev 再次核对均为 2a516b5。旧 Host 目录已不存在，主项目 UGA.uproject 和指定 umap 已只读确认存在。
workspace doctor 的参数已用实际 help 核对为位置参数 neon-dev；其他拟新增 run 命令清楚标为施工目标。

## 未覆盖项

未执行 UE、UAT、Rust 或 C# 产品测试。本轮只校验文档结构、写作及需求映射。
外部 Automation 项目的实际编译引用、ScriptModules 加载、stat unit 后自然退出和实收参数，必须按 P0/P1/P6 获取新证据。
若真实项目被其他会话使用，对应实机项保持 not_run，不能切换其 Junction 或关闭其 Editor。
