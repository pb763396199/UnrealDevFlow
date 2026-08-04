---
schema_version: 1
protocol: 1.3.0
artifact: metrics
artifact_id: ar_01KZ3FBCFNHZDBG085EDMM0B81
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
created_at: 2026-08-03T09:30:18.485261Z
producer: collect-metrics
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts: []
---

# 度量快照

## 汇总

| 指标 | 值 |
| --- | --- |
| 输入 token | 7233 |
| 输出 token | 213833 |
| 缓存写 token | 1222121 |
| 缓存读 token | 84724072 |
| 用户输入轮数 | 5 |
| 助手消息数 | 270 |
| 工具调用数 | 321 |
| 用户纠偏次数 | not_recorded |
| 墙钟跨度 | 2:13:34.102000 |
| 模型 | <synthetic>, claude-opus-5 |
| 提交数 | 12 |
| 其中 fix 提交 | 4 |
| 代码增删行 | +2971 / -296 |

## 质量

| 信号 | 值 |
| --- | --- |
| 评审 code-review | approved（有返工） |
| 验收 | passed（一次过） |

## 环节

| 环节 | 记录 | 起止 | 输入轮数 | 输出 token | 工具调用 |
| --- | --- | --- | --- | --- | --- |
| implementation | ar_01KZ3ET9D7QP28CP1F3ZT13R5E | 07:16–09:20 | 5 | 185004 | 291 |
| review | ar_01KZ3EW91M84KGH2GHJPBJM4RK | 09:21–09:22 | 0 | 3427 | 2 |
| validation | ar_01KZ3F3TNRPR5CTASREF550CFY | 09:22–09:26 | 0 | 12349 | 14 |
| manual-test | ar_01KZ3FA0CY84ESQAC8GFQ6WV3Y | 09:26–09:29 | 0 | 11280 | 8 |
| current | — | 09:29–09:30 | 0 | 1773 | 6 |

## 口径

- 会话目录：`C:\Users\YUMEI\.claude\projects\F--AiProject-UnrealDevFlow`，命中会话：8977f818-93ab-42d5-b090-3b7f9e3654c4。
- 归因：attempt 创建到交付的时间窗内，目录与分支 `feature/compare-devflow-parity` 都对上的消息才计入。
- 同一条助手消息按内容块占多行，按消息去重后才累计 usage。
- 环节边界第一次采集时钉进账本，之后记录被修订也不再移动。
- 带图片等富内容的用户输入不计入输入轮数。
- 采集器版本：1.2.0。数字由 collect-metrics 生成，不手工维护。
