---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01M1B9A2M0VRC0N808JQA3KRKR
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-08-31T06:58:00Z
producer: aes-brainstorm
result: superseded
supersedes: ar_01M1B7ZXGGS8PTJE5E2A7Y4RYR
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1B98DD4X813908Y0PEVJK8P
      digest: sha256:05f3ebeed549c0cc3c27c19bf90ca9cc80600491724f4beb5eca6701131f64ff
      locator: reviews/interface-review.md
    - artifact_id: ar_01M1B7ZVMZARF55F5GS4BRZWWT
      digest: sha256:01d18cb8fa1e556d20b738f62ee733063b681ec294edb2b0155d1d818177790d
      locator: research/verification-research.md
---

# 任务打包配置：精简接口设计 v2

## 本版结论

新增两个公开命令：configure 保存或更新配置，recover 回退未完成交付。
查看配置、预检、打包和状态沿用已有 plan/check/project/status。
用户常用的打包选项只有 configuration、container、output 和 disable-plugin。

task 首次需要打包时才保存配置，后续运行不改配置。用户或 AI 更新必须留下理由和差异。
MCP 等插件可显式禁用；固定目录中的外部数据与 Junction 必须保留。
本版为 proposed，等待设计接受；不表示生产代码或 UE 验收已完成。

本版取代 ../design.md。专家意见及采用理由见 ../reviews/interface-review.md。
事实依据沿用 ../research/ 和 ../reference/reference.md。

## 公开命令

下列命令是拟新增行为，当前 udf 版本尚未实现。

```powershell
# 首次配置，项目和引擎来自任务上下文
udf package configure --task neon-dev/runtime-panel-full-reflection --configuration development --container loose --output "<固定游戏目录>" --disable-plugin ModelContextProtocol --disable-plugin AllToolsets

# 查看、检查、执行都消费同一份保存配置
udf package plan project --task neon-dev/runtime-panel-full-reflection
udf package check project --task neon-dev/runtime-panel-full-reflection
udf package project --task neon-dev/runtime-panel-full-reflection --background
udf package status "<execution-id>"

# 只修改需要变化的项，其余不动
udf package configure --task neon-dev/runtime-panel-full-reflection --configuration shipping --reason "验证 Shipping"

# 高级修改使用严格类型的候选文件
udf package configure --task neon-dev/runtime-panel-full-reflection --file "<候选.toml>" --reason "调整地图或恢复插件"

# 崩溃发生在交付阶段时，先恢复旧交付
udf package recover "<execution-id>"
```

| 入口 | 读写语义 |
| --- | --- |
| configure | 唯一配置写入口；首次创建，后续修改；无变化返回 unchanged |
| plan project | 只读，展示保存值、来源、最终命令及候选交付位置 |
| check project | 只读，检查当前输入和执行条件，不保证未来 Cook 成功 |
| project | 使用保存配置创建新执行；不接受临时覆盖打包配置的选项 |
| status | 只读展示执行和恢复建议 |
| recover | 只按已记录交付事务回退，不 Cook、不升级配置、不猜待删文件 |

保留既有 package plugin/engine/run/clean。run 的现有 workspace 行为继续委托同一解析器，
不新增一套 task shorthand。全局 --format 和 --verbose 继续适用。
不增加 config show/import/export、retry、resume、history 子命令。

重新打包就是再次执行 project，使用当前保存配置和当前可识别源码，生成新 execution。
它不保证复现旧源码；不自动重放失败执行的参数。需要旧配置时以保存快照为候选，
经 configure 明确更新后再执行。recover 只恢复失败交付，不等同重新打包。

## 参数盘点和简化结果

| v1 中的接口或字段 | v2 处理 | 归纳依据 |
| --- | --- | --- |
| config init/update | 合并为 configure | 对同一份配置做幂等写入 |
| config show | 移除，plan 展示配置 | 避免两个查询入口解释同一份值 |
| config import、导出 | 不做独立命令；文件修改用 --file | 配置是普通 TOML，可以复制，无需格式迁移子系统 |
| 成功日志导入 | 延期公开导入接口 | 诊断仍对比日志，不把任意命令变成配置或脚本 |
| --configuration | 保留 | Shipping 是明确缺口 |
| --container | 保留 loose/pak/iostore | 三者是一个互斥选择 |
| --delivery | 改名 --output | 用户只需知道最终游戏目录 |
| --disable-plugin | 保留，可重复 | 支持用户要求的 MCP 排除 |
| --project | 移入文件 source.project | 常规从 TaskContext 推导，歧义时要求显式填写 |
| --expected-revision | 移除 CLI 选项 | 简单字段补丁在锁内合并；完整文件使用其原修订做比较 |
| --reason | 保留，仅更新时必填 | 配置变更可解释，首次创建工具记初始化原因 |
| --file | 一个高级入口 | 强类型候选文件，禁止无类型 --set 或自由 shell 字串 |
| --background | 保留，仅执行选项 | 不属于保存的打包内容配置 |
| platform、target_type | 首批由 Win64 Game 范围限定 | 不暴露尚未验证的平台或目标类型 |
| target_name | 自动解析唯一 Game Target，歧义时文件 build.target 指定 | 不能猜 UGA 或 UnrealGame |
| engine path/version、插件源码位置 | 从任务上下文解析并固定来源证据 | 不要求用户重复填写已知值 |
| Editor prerequisite、Game build | 内部策略 | 检查产物后决定构建，不让用户猜 skipbuildeditor |
| clean、full/iterative | 首批固定 full Cook，保留 UBT 正常增量编译 | 不增加跳阶段/清缓存选项，不混淆缓存类型 |
| maps、Editor content、compression、prereqs、debug files | 高级有类型字段或初始化时冻结项目值 | 不铺开每项 CLI 开关 |
| workspace/workdir/stage/archive | 工具管理执行中间目录 | output 之外不增加三组路径参数 |
| source/mount/保护链接/接管文件 | 高级 data 和 delivery 字段 | 安全规则不缩成一条万能 force |
| uat_args/ubt_args/cook_args | 首批不公开 | 新需求按有类型字段扩展，避免别名和嵌套参数绕过约束 |
| revision、binding、digest、manifest | 工具生成，不是用户选项 | 需要证据，不需要额外配置语言 |
| quality gate 选项 | 首批报告警告，不增加策略配置 | 保留真实 UAT 成败与 Shader 回退信息 |
| retry / resume | 不新增 | 新执行用 project；交付中断由 recover 明确回退 |
| adopt-existing | 不做宽泛布尔开关 | 用文件中的逐项相对路径及旧文件摘要表达接管 |

以 v1 实际出现的接口为基准：4 个明确 config 操作收敛为 1 个 configure，
另补齐 1 个必要 recover。v1 的导出、retry、接管只在正文隐含，不能把它们算成已实现命令。
v1 示例包含 8 个新增长参数；v2 定义 7 个：
configuration、container、output、disable-plugin、reason、file、background。
task/workspace/format/verbose 属于已有参数，不计入新增数量。

## 配置写入规则

| 输入 | 行为 |
| --- | --- |
| 首次 configure | 从任务和项目解析默认值，再应用明确选项，展示并保存结果 |
| 现有配置加常用参数 | 锁内读取当前版本，只补指定字段，其余保留 |
| 同时 --file 与常用参数 | 拒绝，避免两种修改来源的隐式优先级 |
| 修改值但缺 --reason | 拒绝写入，不产生有效修订 |
| 重复相同参数 | unchanged，不产生重复修订 |
| --disable-plugin A 多次调用 | 按集合添加 A，幂等；不会顺便启用其它已禁用插件 |
| 恢复插件或清空列表 | 候选文件的 plugins.disabled 是完整列表；[] 明确表示全部恢复 |
| --file 更新 | 复制当前配置作为候选，保留工具生成的 revision；与当前版本不同则冲突，不覆盖 |
| 直接改当前 TOML | 视为未接纳修改，project 拒绝消费；configure 验证候选和理由后接纳 |
| 未知字段、类型错误、跨任务候选 | 拒绝并定位字段，不忽略、不自动迁移 |

常用 CLI 补丁在锁内基于最新配置合并，不覆盖别人对未指定字段的修改。
同一字段的连续明确命令按执行顺序生效，审计包含实际旧值和新值。
完整候选文件必须带拷贝时的 revision，防止陈旧文件覆盖后来的配置。

当前配置仍在 Host/.udf-package.toml。工具在内部保留已接纳修订内容和变更记录，
包括“改了但还没运行”的修订；它们用于并发校验和追溯，不是第二个可编辑 current。
只有成功接纳后的记录能成为执行输入。历史保留不提供新的公开 history 命令。

示例仅展示用户可编辑字段，工具生成的 binding/revision/schema 字段必须原样保留：

```toml
[build]
configuration = "shipping"

[package]
container = "loose"
output = "F:/Example/Packages/Game"

[plugins]
disabled = [] # 明确恢复所有先前排除项
```

文件更新表示完整替换用户设置；上述节选不能直接作为完整 --file 输入。
完整候选从当前配置复制，保留未展示项。首次文件初始化可省略生成字段，由工具解析并创建。

高级文件的字段边界如下，不能任意添加 UAT 名称：

| 字段 | 类型与默认来源 |
| --- | --- |
| source.project | 项目文件绝对路径；默认 TaskContext.default_project 中唯一项目 |
| build.target | Game Target 名；唯一时自动解析，歧义时必填 |
| build.configuration | 已验证的配置枚举；首批以 Development/Shipping 验收 |
| package.container / output | 容器枚举与最终游戏目录绝对路径 |
| package.compression / prerequisites / debug_files | 布尔值；初始化冻结项目有效值 |
| plugins.disabled | 插件名称数组，文件更新时全量替换 |
| cook.maps | 包路径数组；省略时初始化冻结项目地图策略，[] 表示不额外指定地图，项目 AlwaysCook 目录仍生效 |
| cook.exclude_editor_content | 布尔值；初始化冻结项目有效值 |
| data.mounts | 由 source 绝对路径与项目内相对 mount 位置组成的数组；只允许读取映射 |
| data.protected_paths | 相对 output 的保护路径数组；不能解除工具发现的链接和外部数据保护 |
| delivery.adopt_files | 普通文件的相对 path 与旧内容 sha256 数组；默认 [] |

省略字段只在初始化时补默认值。完整更新文件缺少已保存字段时拒绝，防止误重置。
当前配置文件已有未接纳手工修改时，普通 CLI 补丁也拒绝；用 --file 显式接纳候选，不能覆盖掉手工修改。

## 配置与来源边界

身份绑定采用稳定子集：task_uid、created、canonical Host、主插件名称与规范化仓库/工作区身份。
branch、based_on 和完整元数据摘要作为观测证据保存，不把整个元数据变化当作实例更换。
正常任务状态更新、代码提交、可追踪的 rebase 不应作废打包配置。
主插件仓库换成另一个仓库、created 变化、Host 搬迁等情况阻塞并要求重新绑定。

配置中的完整 project_root 与 task 插件来源分别保存。
task 从何处取插件不能隐含改变 project_root。首次 configure 推荐 context.default_project，
用户通过高级 source.project 可明确选择 Host 或另一个允许的项目。

project/check/plan 使用同一解析器：
- 有保存配置：使用该配置；输入损坏或未接纳修改时拒绝，不退回 legacy。
- 无保存配置的旧 --task：保持旧 Host 路径与默认设置，显示 legacy 提醒；不接受新配置选项。
- workspace 与 task 不匹配：拒绝；选择唯一 workspace 的既有规则保留。
- source.project 明确变化：以有理由的配置修改接纳，检查新来源及保护路径，不能自动替换。

普通代码变更记录在执行快照，不要求改配方。
上游 Packaging 默认值变化只报告差异，已保存且仍合法的值继续有效，由用户或 AI 判断是否更新。
引擎/SDK/Target 兼容检查失败、路径丢失或任务实例不符才阻塞。
配置冻结打包意图，不冻结全部业务配置；无法正确解析的打包决策值报告 unresolved。

## 内部实现收敛

| 必须保留 | 如何控制复杂度 |
| --- | --- |
| 项目隔离 | 复用任务专用工作目录，按输入清单更新，避免每次全量复制源码 |
| 插件排除 | 只修改副本 .uproject，记录差异；开发描述文件不写 |
| 来源快照 | 复用 SourceContext，任务插件不通过共享 DEV Junction 间接取源 |
| 执行记录 | 复用 ExecutionPlan/Step；只增加必要字段，不并行维护第二套状态词 |
| 后台执行 | 一次执行的 runner，不建常驻调度服务 |
| 并发 | 任务 workdir 与 output 两把应用锁；UAT/UBT 的引擎级协调继续保留 |
| 历史 | current 一份，历史为内部不可变证据，不增加用户操作 |
| 交付 | 单一文件事务流程，不同时实现整目录交换和文件覆盖两套发布策略 |

禁止为了减少锁数量而删除 UAT/UBT 共享写入协调。两把应用锁不是引擎锁的替代品。
switch/delete/cleanup 在仍被执行引用时拒绝破坏输入；外部工具绕开锁的风险如实报告。

副本包含真实项目 .uproject、Source、Config、Build 及编译所需插件文件。
保留项目/Target 名称，处理预编译插件 receipt 和绝对路径，不能处理时阻塞。
不通过可写硬链接共享源码。不改原项目、Host 描述文件或 DEV Junction；
验证工具没有写入它们，并把外部变化与本工具修改分开，不能覆盖外部修改来“恢复原样”。

禁用插件检查当前目标的强依赖，被保留插件依赖时列链并阻塞，不自动级联排除。
Content 与数据引用按声明解析，Junction 不能被当作复制目录递归写入。
未版本化共享数据不能承诺不可变快照；发现变更时停止可信交付。严格复制数据模式留待有明确需求后扩展。

## 输出接管和恢复

output 指最终游戏根目录，内部 archive 的 Windows 包装层由工具解析。
工作目录和固定交付目录必须互不包含，也不得包含源工程或外部数据根。
多个 task 指向同一个 output 时共用该路径锁。

首次 output 已有内容：
1. 用户文件和链接默认受保护，工具不清空目录。
2. 与新生成文件不冲突的内容原样保留。
3. 冲突的普通文件需在候选配置 delivery.adopt_files 中逐项列相对路径和当前文件摘要。
4. 配置接纳时保存清单，交付前再核对；文件变化、链接或越界路径拒绝接管。
5. 只允许接管本次新包实际生成的同名普通文件，不扩大到整个目录。

准备新包时旧包不动。验证生成文件后才启动交付事务，保存旧生成文件备份及日志。
只替换本次生成文件、删除上代 manifest 中不再生成的文件，始终跳过受保护路径。

recover 的行为固定为回退本次未完成交付：
- 指定 execution ID，先获取原任务/输出锁并确认旧 runner 和游戏进程未占用。
- 在任何回退写入前校验完整恢复清单。现存内容必须匹配日志允许的中间状态。
- 按日志恢复旧文件、移除本次新增且哈希匹配的文件；外部修改、证据损坏或链接改变时拒绝并输出冲突清单。
- 不接受 --force，不重跑 Cook，不更新配方，不选择“最新执行”。
- 再次 recover 已恢复事务返回 unchanged；已成功发布的执行不允许通过此命令回滚。
- 首次交付没有上一代包时，只撤销本次已记账写入，保留预先存在的用户内容。
- 记录 delivery_rolled_back；原 UAT 退出码不被抹掉。之后 project 可创建新执行。

多文件交付不是对外原子可见。交付期间工具不启动游戏，外部启动器也必须遵守窗口。
中断后不保证旧目录立即可运行，直到 recover 或正常交付完成。
recover 不能覆盖运行后的新用户存档；没有可信恢复证据时保持 blocked。

## UAT、诊断与适用范围

沿用 v1 已查证行为：Shipping 映射 clientconfig；IoStore 使用 pak+iostore；
loose 不使用 skippak；Editor 内容策略只有显式保存为排除时才传 SkipCookingEditorContent。
按实际 Editor 产物决定是否构建，不能只看 DLL 存在。

首批验证 Windows/UE 5.5/Win64 Game。其它平台/目标未经适配就明确 unsupported，
不通过通用字符串允许绕过。高级字段采用版本化类型，不能包含任意 shell 命令或自由 UAT 参数。

错误报告必须保留总退出码、失败阶段、日志位置与 source/target/object。
Shader 回退单列警告，Base_Bridge001 类型错误与 NeverCook 分开。
status 可比较前一次同任务执行的配置、来源和错误集合，不增加 diff/history 命令。
check/plan 不创建执行、不改 latest；启动前保存 running，失联且不能确认终态记 unknown。
UAT 成功但交付失败时，整体执行仍为 failed，udf 返回非零；另存 UAT 退出码 0，避免掩盖交付失败。

## 必测场景与影响面

| 场景 | 需要证明 |
| --- | --- |
| configure 重复、局部修改、陈旧文件 | 幂等、未指定字段不变、CAS 冲突可见 |
| 禁用与恢复插件 | CLI 添加幂等，文件 [] 能清空，开发项目字节不被工具改动 |
| Shipping 与容器 | 配置至最终 UAT/receipt 一致 |
| 正常任务状态、rebase 与实例重建 | 不误报普通变化，不串新实例 |
| legacy 与未接纳配置 | 不改旧 Host 含义，坏配置不回退 |
| 共享 output/工作目录/引擎 | 冲突写入被阻止，不能并发交付 |
| 已有文件与保护链接 | 无整目录接管，无越界删除 |
| 交付每一步中断与再次 recover | 旧文件可恢复、外部新内容不被覆盖、恢复幂等 |
| 失败重新 project | 新 execution，不暗中重放旧参数或旧代码 |
| 历史日志分类 | 保持 11/4/17 与 11/5/18；成功 FULL COOK 不被误判 |

实际生产源码未变。沿用 v1 的 7 个直接调用文件和执行/来源/清理/文档影响面，
新增 configure/recover 的集成测试；不删除已发布命令。
参数计数可用 Select-String 检查本版“参数盘点”表，不能把自动生成字段计入人机输入。

尚未实测 UE 项目副本、完整 INI 解析和 Windows 交付故障注入。
设计评审通过只表示这些行为定义足够明确，不代替实现验证或用户接受。
