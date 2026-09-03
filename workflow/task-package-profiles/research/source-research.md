---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1B7ZHG8VNX6YR9J5P36EYMQ
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-08-31T06:28:00Z
producer: aes-research
result: complete
topic: 项目来源与插件排除
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts: []
---

# 项目来源与插件排除调查

## 问题与停止条件

判断完整项目、任务插件及用户指定插件排除能否不改共享开发项目。查询日期 2026-08-31，基线 07b7c4b，UE 5.5.4。

## 看过的地方

| 来源 | 可核对事实 |
| --- | --- |
| src/commands/package.rs:645 | task 分支直接把 Host 当作 project_root |
| src/host/mod.rs:19 | TaskContext 保存 default_project、engine_path 与插件根信息 |
| src/source_context.rs:10 | SourceContext 区分 project、host、主插件和依赖插件 |
| src/commands/package.rs:497、539、564 | 已有插件依赖闭包、复制及 HostProject 生成逻辑 |
| reference/reference.md | 用户要求真实场景、任务代码、禁用 MCP 和保留外部数据 |

## 查到的

现有 --task 语义是打包编译 Host，不能假定它含完整项目地图、配置和游戏模块。
workspace 路径下的 Plugins Junction 是可变输入。只查 UBT mutex 不会阻止另一个会话 switch。
用户明确要求禁用无法打包的 MCP 插件，不应将其改写成默认一律启用或拒绝插件排除。

## 推断及设计输入

保存两种独立信息：完整项目来源和插件来源。新配置默认建议 context.default_project，
主插件取任务 worktree；依赖按解析后的实际路径保存。保留显式选择 task Host 的能力。

在任务专用的打包工作目录复制项目描述、Source、Config、Build 和需要编译的插件源码。
副本维持原项目及 Target 名称，在副本 .uproject 修改用户排除的插件，开发项目不改。
当前 copy_tree 不排除 Binaries，但复制到文件不代表它适用于本次目标。预编译插件和 ThirdParty 文件需按目标与 receipt 验证。

任务源码不能继续通过共享 DEV/Plugins Junction 间接引用。复制前后对源码输入清单比较，
变化则拒绝使用该份准备结果。描述文件内 AdditionalPluginDirectories 和绝对路径也要解析并报告。
禁止依赖静默指回另一个任务或主检出上的可变链接。

Content 与外部数据路径要按配置逐一声明。不能把只读访问约定宣称为操作系统强制隔离。
大型共享数据不全量复制时，记录真实路径和可用的版本标识，声明无法保证外部写入者不变更。

排除插件先检查保留插件的强依赖和目标模块。若 A 强依赖被禁用 B，阻塞并列出依赖链，
不自动级联关闭 A。历史用户授权的 MCP 例子只作为配方示例，不变成产品硬编码黑名单。

## 未知与风险

复制后的工程仍可能含绝对 Build.cs 路径、二进制插件绑定路径或外部资源写入行为。
这些是具体工程的准备检查与实测条件，不能承诺所有 UE 项目可无条件搬迁。
若副本模式不兼容，返回具体阻塞；不得自动退回原地修改共享开发项目。
