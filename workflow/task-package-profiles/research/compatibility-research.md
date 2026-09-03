---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1B7ZSSZ97KJQHC3X8NKATM9
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-08-31T06:28:00Z
producer: aes-research
result: complete
topic: 兼容与验证范围
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts: []
---

# 兼容范围与验证调查

## 问题与停止条件

确认新增配置能否保留既有入口，以及哪些覆盖不能凭单元测试宣称。日期 2026-08-31，基线 07b7c4b。

## 看过的地方

| 来源 | 事实 |
| --- | --- |
| src/cli.rs:276、src/main.rs:316 | 8 个 package 二级动作与固定分发 |
| tests/package_commands.rs | 6 个命令参数测试，Shipping 只覆盖底层生成器 |
| tests/package_lifecycle.rs | 12 个集成测试，覆盖查询不更新 latest 等行为 |
| tests/cli_taxonomy.rs | 命令词义和 help 约束 |
| README.md:256、skills/unrealdevflow/SKILL.md | README 已有 package，技能未覆盖配方与恢复 |
| src/execution.rs、src/source_context.rs | 可复用的计划和来源契约 |

## 查到的

旧 --task 表示任务 Host；直接改成完整主项目会改变既有行为。
check/plan 不创建执行记录是已有契约，需要保留。
已有参数测试通过不能证明 CLI 暴露 Shipping，更不能证明实际 Cook 和交付成功。

## 推断及设计输入

增加 package config init/show/update/import；原 project/check/plan/run 入口保留。
有保存配置就统一解析；无配置时查询只报告 legacy 默认计划与初始化建议，不写配置。
旧执行命令可保持原 Host 行为并明显提示 legacy；不静默切到完整项目，不默认禁用任何插件。
新配置初始化显式选择完整项目或 Host，并冻结默认项。

只承诺本轮设计覆盖当前 Windows/UE 5.5 场景。Linux、移动端、控制台、Server、DLC、签名与商店发布
保留扩展边界并在未适配时明确拒绝，不以自由字符串参数冒充已支持。

验收至少包括：CLI 至 UAT Shipping 传递、配置冻结/并发/重建同名 task、
MCP 排除及依赖冲突、loose/pak/iostore、受保护链接交付、
失败退出码、中断/进程复用、source 漂移、旧制品识别、
34/37 条日志去重、Full Cook 与 Shader 警告分级、旧 JSON 查询兼容。
真实 UE 编译、Cook、运行时 smoke 是另外一层验收，不能用本轮文档校验替代。

## 影响面

定位 PackageAction、commands::package、project_package_commands 的复算命令：
rg -l 'PackageAction|commands::package|project_package_commands' src tests
当前返回 7 个文件。另需检查生命周期、共享执行与来源、task 清理切换、文档和技能入口。
设计不删除现有命令，本轮不变更生产文件或测试。

## 未知与风险

共享主项目中其它 task 在开发，未来实现应继续使用本任务 worktree。
原子配置写入、执行进程与文件交付的详细模块划分留给 aes-plan，设计先固定可观察行为。
