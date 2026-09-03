---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1B7ZNFFN4YRFFT2ZMC958VA
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-08-31T06:28:00Z
producer: aes-research
result: complete
topic: 交付与外部数据
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts: []
---

# 固定目录交付与外部数据调查

## 问题与停止条件

确认覆盖交付与受保护数据能否分离，以及失败后能恢复到什么程度。日期 2026-08-31，基线 07b7c4b。

## 看过的地方

| 来源 | 事实 |
| --- | --- |
| src/commands/package.rs:160、183 | manifest 当前仅有路径和字节数 |
| src/commands/package.rs:1056 | clean 校验受管根后按 cleanup_targets 删除目录 |
| src/junction/mod.rs、validator.rs | 已有 Windows Junction 操作与检查 |
| reference/reference.md | 固定 stage 路径有旧 exe 与外部数据链接，失败不能冒充新交付 |

## 查到的

当前受管根校验不等于“任意外部交付目录可以安全清空”。
文件存在和文件时间均不足以证明属于本次执行。外部目录的用户文件也不能通过一次扫描就归工具所有。

## 推断及设计输入

准备、Cook、Stage、Archive 在受管工作目录完成，验证后才更新配置中的固定交付目录。
manifest 应记录本次执行、相对路径、文件大小、校验值与链接类型，链接目标单独记录。
首次接管已有交付目录需要显式 adopt-existing，先展示将接管的生成文件和保留文件，
拒绝目录根、源工程目录、数据目录及重解析越界。

整目录 rename 不适合包含受保护用户链接的交付目录。采用文件级交付清单：
先写同卷临时文件和恢复日志，保存待替换旧文件，再发布生成文件；只清理上一代 manifest 声明的旧生成文件。
未记账的用户文件和保护链接不动。新包生成文件与保护路径冲突时阻塞。
跨卷复制先完整复制校验到目标卷，再进入替换过程。

交付目录需独占租约，运行中的游戏占用文件时延后交付。
多文件替换不承诺整体原子可见，工具在未完成时标记 delivering，禁止工具启动半交付的包。
崩溃后按日志恢复或继续；只有验证完新生成文件和链接关系后标记 succeeded。
外部启动器不受工具控制，文档必须提醒交付期间不能启动该目录中的游戏。

## 未知与风险

Windows ACL、杀毒软件、文件占用和断电会影响恢复，需真实 NTFS 故障注入。
保护是“不删除、不改写外部数据”；它不等于数据内容在 Cook 期间绝对不变。
现有包采用 loose 或 pak 必须检查 manifest 和实际布局，不能从 exe 存在猜测格式。
