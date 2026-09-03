---
schema_version: 1
protocol: 1.3.0
artifact: review
artifact_id: ar_01M1K2QHNR4DGW6X9BT3AXHKBQ
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-03T06:50:00Z
producer: aes-review
review_type: design
verdict: approved
supersedes: ar_01M1B98DD4X813908Y0PEVJK8P
blocking_findings: []
reviewers:
  - "Planck / executor / CLI易用性 / 01a05698-76fe-75e0-8b1c-e9af99d3108e"
  - "Fermat / critic / 架构精简 / 01a05693-7ef1-7062-8711-c41e0fb96902"
  - "Jason / executor / 安全兼容 / 01a05693-d63a-7951-974d-2047dae3491f"
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1GCKQ5K74MTZJ9FFZK74M54
      digest: sha256:174e055c18b8c7c4af7126964a7b983ad9df997c24c51b8a0891d68b3739089b
      locator: design/package-design-v5.md
---

# 项目打包设计第二轮专家评审

## 结论

最终版接口设计已完成修订。独立复核确认，三个原阻断点均已处理：
没有把作者自己的取舍写成三位专家的一致意见。评审未运行 UE，也不证明实现正确。

评审正文为 v1 的 proposed 版本，原摘要为 sha256:12c3f79831a1651cea5df34aacc9cf7f52a822463bcd0bec75b4a70f84e1a485。
归档时只把 v1 头部结果改为 superseded，正文和行号未变；依赖指向归档后的同一文档。
原 CLI architect 未完成返回，已停止，不计入完成专家数。由 Planck 完成 CLI 专项。

## 需要修正

| 编号 | 严重度 | v1 位置 | 问题与后果 | 最小修正 |
| --- | --- | --- | --- | --- |
| R1 | P1 | design.md:224、241 | 有交付崩溃恢复承诺，没有公开可用入口；可能留下无法操作的半交付目录 | 明确 recover 命令、输入、锁、幂等和冲突处理 |
| R2 | P2 | design.md:120、130 | config 有四个操作，正文另含 export/retry/adopt，完整公开面未盘点 | 给全部入口读写定义和删并表，复用 plan |
| R3 | P2 | design.md:90、123 | 禁用列表更新及清空语义不明；省略项可能被实现成意外恢复插件 | 固定增量标志语义及完整文件 [] 的清空行为 |
| R4 | P2 | design.md:84、92 | 身份证据与用户选项混列，中间输出目录增加选择负担 | 元数据工具生成，仅 output 是日常位置选项 |
| R5 | P2 | design.md:93、179 | 自由 UAT/UBT/Cooker 参数通道实现面过大，嵌套参数易绕过约束 | 首批不公开，按类型扩展实际需求 |
| R6 | P2 | design.md:65、109 | 身份和来源变化判定不够细，可能把正常状态/rebase判成配置失效 | 稳定身份绑定与可变来源证据分离 |
| R7 | P2 | design.md:232 | adopt-existing 范围未精确定义 | 只接管本次生成的同名普通文件，带旧摘要和逐项清单 |

R4 至 R7 是需在修订中明确的设计问题；未用它们制造必须等待用户重新选择的门槛。

## 专家意见与作者裁决

| 专家建议 | 处理 | 理由 |
| --- | --- | --- |
| Planck：合并 configure，保留四个高频选项 | 采用 | 让配置写入成为一项操作 |
| Planck：另加 configure --show | 不采用 | plan 已覆盖读取，不再加查询模式 |
| Planck：none 哨兵或 clear-disabled-plugins | 不采用 | 高级文件 disabled=[] 已明确清空；避免保留插件名哨兵及第二套重置参数 |
| Planck/Jason：新增 retry 并可选 revision | 不采用公开命令 | 新执行用 project；旧配方需显式 configure，减少历史重放语义 |
| Jason：新增 recover | 采用并收紧 | 固定回退未完成交付，不把只读诊断与实际恢复混在一起 |
| Jason：禁止 configure 带配置参数 | 不采用为硬规则 | configure 是持久写入，常用参数不违反固定配置；project 才禁止临时覆盖 |
| Jason：绑定整个 meta_digest、branch、based_on | 部分采用 | 保留观测证据，不当身份失效条件，避免正常任务状态和 rebase 被误拦 |
| Jason：补开发项目字节不变验收 | 保留并明确 | v1 验证表已有要求，不能声称完全缺失；增加外部变化不归因本工具的说明 |
| Fermat：history 不再是公开模型 | 采用 | current 单一正本，内部保留未执行修订的证据，避免丢掉配置更新原因 |
| Fermat：只保留两把最小锁 | 部分采用 | 两把应用锁描述日常对象，引擎 UAT/UBT 共享写入仍需协调 |
| Jason：仅 allowlist 自由参数 | 收窄 | 首批没有明确需求的自由参数通道不暴露 |
| 三位：保留插件排除、外部数据与副本 | 采用 | 这些是用户场景与安全约束，不能用简化接口为由删除 |

## 已核实的事实

- 当前 package project 只有 workspace/task 选择，配置实际写死为 Development。
- v1 示例有 8 个新增长参数，加上已有 task/format 共出现 10 个不同长参数。
- check/plan/status 已存在，且 check/plan 不创建执行记录。
- v1 已说明 .uproject 字节不改、强依赖冲突及未知副本风险，这些不重复算缺失。
- 原任务条件未变，本轮只更新设计记录；不改已经关闭的调查票答案。

## 未覆盖范围

- 未实测副本项目或 UAT，未验证 Windows 进程和交付恢复。
- 专家分工审查不等于三人逐行审查所有文件。
- 新版设计需另做独立复核，不能拿本份 changes_requested 作为通过证明。
