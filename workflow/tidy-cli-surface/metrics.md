---
schema_version: 1
protocol: 1.3.0
artifact: metrics
artifact_id: ar_01KZ40NS9F901YAWMQCA12H92X
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
created_at: 2026-08-03T14:33:05.072030Z
producer: collect-metrics
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts: []
---

# 度量快照

## 汇总

| 指标 | 值 |
| --- | --- |
| 输入 token | 771 |
| 输出 token | 305418 |
| 缓存写 token | 2405794 |
| 缓存读 token | 238514535 |
| 用户输入轮数 | 9 |
| 助手消息数 | 366 |
| 工具调用数 | 403 |
| 用户纠偏次数 | not_recorded |
| 墙钟跨度 | 4:57:54.929000 |
| 模型 | claude-opus-5 |
| 提交数 | 3 |
| 其中 fix 提交 | 2 |
| 代码增删行 | +277 / -58 |

## 质量

| 信号 | 值 |
| --- | --- |
| 评审 code-review | approved（有返工） |
| 验收 | passed（有返工） |

## 环节

| 环节 | 记录 | 起止 | 输入轮数 | 输出 token | 工具调用 |
| --- | --- | --- | --- | --- | --- |
| design | ar_01KZ3K7ZXZF9KGDQ7DDP6Q2WSB | 09:35–10:38 | 2 | 32368 | 18 |
| plan | ar_01KZ3MVK2Z8BJ2RWJBS7G1338P | 10:38–11:06 | 2 | 48234 | 23 |
| implementation | ar_01KZ40KNHCMHJFZHTY17CGTA2T | 11:06–14:31 | 5 | 221339 | 355 |
| review | ar_01KZ40MSSATP1D6K1K3CH4WW1A | 14:32–14:32 | 0 | 1835 | 2 |
| validation | ar_01KZ40N1BGZ6GJRGHMTZM585QH | 14:32–14:32 | 0 | 496 | 1 |
| manual-test | ar_01KZ40N9BPKNWQWGJANPPB2VXC | 14:32–14:32 | 0 | 459 | 1 |
| current | — | 14:32–14:33 | 0 | 687 | 3 |

## 口径

- 会话目录：`C:\Users\YUMEI\.claude\projects\F--AiProject-UnrealDevFlow`，命中会话：8977f818-93ab-42d5-b090-3b7f9e3654c4。
- 归因：attempt 创建到交付的时间窗内，目录与分支 `refactor/tidy-cli-surface` 都对上的消息才计入。
- 同一条助手消息按内容块占多行，按消息去重后才累计 usage。
- 环节边界第一次采集时钉进账本，之后记录被修订也不再移动。
- 带图片等富内容的用户输入不计入输入轮数。
- 采集器版本：1.2.0。数字由 collect-metrics 生成，不手工维护。
