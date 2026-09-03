---
schema_version: "1"
protocol: "1.3.0"
artifact: "wayfinder"
artifact_id: "ar_01M1B7V45143BG88GY2S4W416J"
work_item_id: "wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ"
created_at: "2026-08-31T06:24:24Z"
producer: "aes-wayfinder"
state: "handed_off"
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: "ar_01M1B7ZVMZARF55F5GS4BRZWWT"
      digest: "sha256:01d18cb8fa1e556d20b738f62ee733063b681ec294edb2b0155d1d818177790d"
      locator: "research/verification-research.md"
navigation:
  destination: "确定任务打包配置、项目来源、插件排除、安全交付和执行诊断的设计，让后续实现可以按证据推进。"
  route: "调查已复核，按用户预授权交给 aes-brainstorm 写设计"
  domains:
    - id: "profile"
      title: "配置保存与更新"
      depends_on: []
      completion: "research/profile-research.md 及独立复核有证据，调查票已解决"
    - id: "source"
      title: "项目来源与插件排除"
      depends_on: []
      completion: "research/source-research.md 及独立复核有证据，调查票已解决"
    - id: "uat"
      title: "参数与引擎行为"
      depends_on: []
      completion: "research/uat-research.md 及独立复核有证据，调查票已解决"
    - id: "delivery"
      title: "交付与外部数据"
      depends_on: []
      completion: "research/delivery-research.md 及独立复核有证据，调查票已解决"
    - id: "execution"
      title: "状态与诊断"
      depends_on: []
      completion: "research/execution-research.md 及独立复核有证据，调查票已解决"
    - id: "compatibility"
      title: "兼容与验证范围"
      depends_on: []
      completion: "research/compatibility-research.md 及独立复核有证据，调查票已解决"
  nodes: []
  edges: []
  frontier: []
  tasks: []
  resume:
    session: "aw-1sntnf-u0x1p"
    position: "handoff"
    next: "aes-brainstorm 写 proposed 设计，不启动实施"
    claimed_tasks: []
  handoff:
    route: "aes-brainstorm"
    confirmation: "confirmed"
    summary: "用户本轮明确：如果没有需要讨论的就可以写设计。六张票已关闭，独立复核通过；授权仅覆盖进入设计，不代表设计已接受或允许实施。"
---

# WayFinder

## 目的地

确定任务打包配置、项目来源、插件排除、安全交付和执行诊断的设计，让后续实现可以按证据推进。

## 说明

用户已在本轮授权无待讨论问题时进入设计。独立复核见 [复核记录](research/verification-research.md)。
该授权不等于接受尚未阅读的设计。后续实际项目副本、配置解析和 Windows 恢复仍需实施阶段验证。

| 领域 | 完成条件 |
| --- | --- |
| 配置保存与更新 | [调查证据](research/profile-research.md)经独立复核，对应调查票有结论 |
| 项目来源与插件排除 | [调查证据](research/source-research.md)经独立复核，对应调查票有结论 |
| 参数与引擎行为 | [调查证据](research/uat-research.md)经独立复核，对应调查票有结论 |
| 交付与外部数据 | [调查证据](research/delivery-research.md)经独立复核，对应调查票有结论 |
| 状态与诊断 | [调查证据](research/execution-research.md)经独立复核，对应调查票有结论 |
| 兼容与验证范围 | [调查证据](research/compatibility-research.md)经独立复核，对应调查票有结论 |

## 已有决定

- [交付与外部数据](wayfinder-tickets/交付与外部数据.md)：受管生成后按交付清单替换文件
- [兼容与验证范围](wayfinder-tickets/兼容与验证范围.md)：保留 legacy 来源并显式初始化新配置
- [参数与引擎行为](wayfinder-tickets/参数与引擎行为.md)：冻结解析后的设置并用版本适配生成参数
- [状态与诊断](wayfinder-tickets/状态与诊断.md)：接入共享执行结构并保存运行状态
- [配置保存与更新](wayfinder-tickets/配置保存与更新.md)：Host 当前配置加独立修订历史
- [项目来源与插件排除](wayfinder-tickets/项目来源与插件排除.md)：独立项目副本应用插件排除

## 尚未明确

## 范围外

- 本轮不修改生产代码、不执行 UE 打包或发布版本。
- 本轮不修复业务材质或面板；不承诺所有平台、DLC 和商店分发。
- 实际项目搬迁、外部数据读写边界及恢复故障注入交给后续实施前验证。
