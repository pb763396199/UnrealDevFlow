---
schema_version: 1
artifact: metrics
artifact_id: ar_01KZ5G5ARQ43EWQA156ZXS4YYJ
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
created_at: 2026-08-04T04:22:57.559846Z
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
| 输入 token | 777 |
| 输出 token | 307428 |
| 缓存写 token | 2407338 |
| 缓存读 token | 239781712 |
| 用户输入轮数 | 9 |
| 助手消息数 | 369 |
| 工具调用数 | 406 |
| 用户纠偏次数 | not_recorded |
| 墙钟跨度 | 4:58:36.098000 |
| 模型 | claude-opus-5 |
| 提交数 | 0 |
| 其中 fix 提交 | 0 |
| 代码增删行 | +0 / -0 |

## 质量

还没有评审或验收记录。

## 环节

| 环节 | 记录 | 起止 | 输入轮数 | 输出 token | 工具调用 |
| --- | --- | --- | --- | --- | --- |
| design | ar_01KZ3K7ZXZF9KGDQ7DDP6Q2WSB | 09:34–10:38 | 2 | 32824 | 20 |
| plan | ar_01KZ3MVK2Z8BJ2RWJBS7G1338P | 10:38–11:06 | 2 | 48234 | 23 |
| manual-test | ar_01KZ5G5351KXET1YXPYJ23ESP9 | 11:06–14:32 | 5 | 224129 | 359 |
| delivery | ar_01KZ5G537HG5Z0YWFEAQQWXT0Z | 14:32–14:33 | 0 | 2241 | 4 |

## 口径

- 会话目录：`C:\Users\YUMEI\.claude\projects\F--AiProject-UnrealDevFlow`，命中会话：8977f818-93ab-42d5-b090-3b7f9e3654c4。
- 归因：只按目录和时间窗过滤。路线没有可用的分支名，**未按分支过滤**。
- 同一条助手消息按内容块占多行，按消息去重后才累计 usage。
- 环节边界第一次采集时钉进账本，之后记录被修订也不再移动。
- 带图片等富内容的用户输入不计入输入轮数。
- 采集器版本：1.3.0。数字由 collect-metrics 生成，不手工维护。
