---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01KZWJFVQ0JNC2EDH5NCF5FNN9
work_item_id: wi_01KZWHP5WGDDBVMW0WVRJTAJZE
created_at: 2026-08-13T03:30:00Z
producer: aes-research
result: complete
topic: UEB 编译与打包发布能力
supersedes: null
dependencies:
  work_item_contract_digest: sha256:57b10aa47e84304ef4ecf7f309eea2b72f99bc3f110ea3a68a92e08c803731b4
  artifacts: []
---

# UEB 编译与打包发布能力调查

## 结论

UnrealDevFlow 可以原生提供 UEB 等价发布命令，但当前实现只能复用环境解析、项目选择、任务 Host 和部分 UBT 调用。项目打包、插件发布包、引擎构建、Installed Build、流水线、产物清单和失败诊断都需要新增。现有 `build gate` 还会拒绝 UEB 依赖的 `RunUAT.bat`，并且两边默认互斥策略冲突。

“等价”应按可观察行为判断：同样的输入解析、UE 命令参数、阶段顺序、产物目录、JSON 证据、失败分类和副作用边界。只增加同名命令不算等价。

调查时间为 2026-08-13。UEB 工作区版本以当日 `F:\ShanghaiP4\neon\Plugins\UEB` 为准，UnrealDevFlow 设计分支基线为 `5138373f7a15d01c95a06022c153f73c8be7b3e6`。

## 看了哪些地方

| 范围 | 主要证据 | 用途 |
| --- | --- | --- |
| UEB 命令表面 | `UEB/src/cli.ts:110-340`、`UEB/README.md` | 枚举公开命令与全局参数 |
| 项目与引擎命令 | `UEB/src/commands/project.ts:10-80`、`UEB/src/commands/engine.ts:11-77` | 核对 RunUAT、BuildGraph、Build.bat 参数 |
| 插件发布包 | `UEB/src/commands/plugin.ts:87-120`、`UEB/src/lib/ubtMatrix.ts:36-61,214-248` | 核对 BuildPlugins 与依赖感知直接 UBT 路线 |
| 流水线与结果 | `UEB/src/commands/pipeline.ts:21-76`、`UEB/src/lib/steps.ts:34-62`、`UEB/src/result.ts` | 核对阶段顺序、失败短路和结果字段 |
| 诊断与清单 | `UEB/src/lib/diagnostics.ts:214-278`、`UEB/src/manifest.ts` | 核对失败证据与制品清单 |
| 配置与命令解析 | `UEB/src/config.ts:11-116`、`UEB/src/lib/resolve.ts` | 核对平台、配置、环境变量、配置文件和默认值 |
| 业务契约 | `UEB/tests/business-golden/uebCommandGolden.cases.ts:35-260` 及相关测试 | 核对用户命令到 UE 参数和 JSON 的稳定映射 |
| UnrealDevFlow 当前体系 | `src/cli.rs:134-221`、`src/commands/build.rs:39-209`、`src/build_policy.rs:210-395` | 核对已有命令、后端和门禁 |

## UEB 的全部构建与发布前命令

| UEB 命令 | 实际后端 | 主要输入 | 产物或证据 |
| --- | --- | --- | --- |
| `ueb build`、`ueb project build` | `RunUAT.bat BuildTarget`，Editor target | project、engine、platform、configuration、clean、mutex | 项目编译结果、命令列表、清单与摘要 |
| `ueb package`、`ueb project package` | `RunUAT.bat BuildCookRun` | project、platform、client config、archive dir、额外 package 参数 | Cook、Stage、Archive、Package 目录及清单 |
| `ueb plugin <seed...>` | 依赖闭包、持久 Host workspace、逐矩阵直接 UBT、组装插件包 | 插件名或目录、搜索根、输出、stage root、Win64/Linux | 每个 seed 的独立发布包、闭包证据、构建矩阵、清单 |
| `ueb plugin --all` | 同上 | 搜索根下全部源码插件 | 全部 seed 的独立发布包 |
| `ueb plugin build-deps` | 上述依赖感知路线的长命令入口 | whitelist、stage root 等配置 | 同上 |
| `ueb plugin build` | `RunUAT.bat BuildPlugins` | plugins root、package、whitelist、target platforms | Rocket-compatible 插件包 |
| `ueb engine build` | `GitDependencies.exe`、`GenerateProjectFiles.bat`、`Build.bat` | engine、editor target、额外 targets、platform、config、mutex | 源码引擎编译产物与命令证据 |
| `ueb engine installed` | `RunUAT.bat BuildGraph` 的 `Make Installed Build Win64` | engine、output dir、Windows/Linux 选择 | Installed Build 目录 |
| `ueb pipeline run` | 顺序调用配置的步骤，失败立即停止 | `engine.build`、`engine.installed`、`project.build`、`project.package`、`plugin.build` | 最后制品目录与完整命令序列 |

`inspect`、`doctor`、`explain`、`clean`、`clean diagnostics` 和 `switch` 不直接产出发布包，但它们构成发布构建的发现、预检、预览、维护和故障恢复入口。

## UEB 的共同产品行为

| 行为 | 源码事实 |
| --- | --- |
| 发现与覆盖顺序 | CLI flag、`UEB_*` 环境变量、JSON config、默认值合并；project 和 engine 还支持工作目录与 EngineAssociation 发现 |
| 短命令解析 | `build/package/plugin` 支持位置参数；platform 与 configuration 顺序不敏感；插件命令支持平台后缀 |
| 平台 | 项目支持 Windows、Linux、WindowsWithLinux；插件目标为 Win64、Linux、Win64+Linux |
| 配置 | Development、Shipping、Debug、Test；插件矩阵按 `.uplugin` 模块约束派生 Editor Development 和 Game Development/Shipping |
| 互斥 | `no-mutex`、`wait`、`fail-fast`；默认 `no-mutex`；能控制的 UAT 调用通过 `-ubtargs` 传入 |
| 预览与机器读取 | 全局 `--dry-run` 与 `--json`；`explain` 复用实际命令构造器，降低预览漂移 |
| 执行 | 结构化 argv，步骤按序执行，首个失败短路 |
| 制品清单 | 成功后记录制品相对路径、大小和可选 SHA-256；`--no-hash` 可关闭散列 |
| 失败诊断 | 写入 `.ueb/diagnostics/<时间>-<intent>/`，包含 report、summary、stdout、stderr 和日志索引 |
| 退出码 | 区分成功、构建失败、基础设施失败、用法错误和可重试情况 |
| 副作用边界 | 不做 SCM checkout/update、版本写入、上传、通知、Jenkins 编排或生产发布 |

## 插件发布能力的关键差异

UEB 保留两条插件路线。`plugin build` 是对 UE `BuildPlugins` 的薄封装。默认的 `plugin <seed...>` 是 UEB 自己实现的发布构建器，能力更强。

依赖感知路线会扫描 `.uplugin` 依赖，区分项目插件与引擎提供插件，为每个 seed 计算闭包和构建顺序，把需要的源码同步到持久 Host workspace，然后按 `.uplugin` 的平台、目标和配置限制生成 UBT 矩阵。每个 seed 最终只输出自己的发布包，依赖只作为编译输入。持久 workspace 与 manifest 支持后续运行复用缓存。

这部分不能用 UnrealDevFlow 现有 `build task --primary-only` 代替。后者只给任务 Host 的 Editor 构建追加 `-Module=`，没有 Game Development/Shipping 矩阵，也不组装可分发插件目录。

## UnrealDevFlow 当前能复用什么

| 当前能力 | 与 UEB 的关系 | 判断 |
| --- | --- | --- |
| workspace 的 project、engine、plugins root、Hosts 配置 | 覆盖 UEB 的部分发现输入 | 可复用，但需增加发布输出与配置层 |
| task Host 与主插件、依赖插件解析 | 可作为任务分支的发布构建输入 | 可复用，需定义从 task 或 workspace 取源码 |
| `build task` 与 `build project` | 都调用 `Build.bat` 编 Editor | 只能覆盖开发编译子集 |
| light、medium、heavy profile | 面向严格度和重建程度 | 可用于编译阶段，不能代替 Development/Shipping 等 UE configuration |
| background、status、日志路径 | 可扩展到长时间发布构建 | 需要把单次 UBT 状态扩成多阶段状态 |
| `build check` 的工程与 mutex 探测 | 可作为发布命令预检基础 | 可复用，需要识别 UAT 与多阶段锁使用 |
| 多 workspace 与明确 task ref | 比 UEB 当前目录发现更适合并行任务 | 应保留为 UnrealDevFlow 原生命令的身份模型 |

## 直接缺失的能力

| 缺失项 | UEB 已有行为 | UnrealDevFlow 原生等价命令需要补什么 |
| --- | --- | --- |
| 项目打包 | BuildCookRun 默认执行 build、cook、stage、archive、package、pak、compressed、prereqs | 新的 package 命令、参数模型、输出目录和阶段证据 |
| 插件发布包 | BuildPlugins 与依赖感知直接 UBT 两条路线 | 插件种子解析、闭包 staging、构建矩阵、包组装和 per-seed 输出 |
| 引擎源码构建 | GitDependencies、GenerateProjectFiles、Build.bat | 引擎命令族与下载缓存参数 |
| Installed Build | BuildGraph | output dir、平台开关和长任务证据 |
| 流水线 | 配置步骤顺序执行并失败短路 | 声明式步骤、预览和统一状态 |
| dry-run 与 explain | 输出将执行的结构化 argv 和解析来源 | 所有发布命令共用的计划模型 |
| JSON 结果协议 | 稳定结果信封、解析来源、命令、制品与诊断字段 | 统一 schema，不能沿用当前只有部分命令支持 JSON 的状态 |
| 制品清单 | 路径、大小、可选 SHA-256 | 发布目录遍历、散列开关和清单文件 |
| 失败诊断包 | stdout、stderr、UE 日志和人读摘要 | 多阶段日志收集、错误分类和安全清理 |
| 发布构建配置 | flag、env、config、default 合并 | workspace 配置扩展与命令级覆盖规则 |

## 语义冲突

| 冲突 | 当前证据 | 必须作出的改变 |
| --- | --- | --- |
| 原始 UAT 被门禁拦截 | `src/build_policy.rs:345-351` 把 `runuat.bat` 列为原始 UE 命令；只放行 `build task` 和 `build project` | 门禁改为放行声明过的 UnrealDevFlow 发布命令，仍拒绝用户直接绕过 |
| 默认 mutex 相反 | UEB 默认 `no-mutex`；UnrealDevFlow `build gate` 拒绝 `-NoMutex` 和 `--mutex no-mutex` | 统一一套安全策略。若保留 no-mutex，必须由预检证明没有共享中间产物冲突 |
| profile 与 configuration 不是一回事 | UnrealDevFlow profile 控制严格参数；UEB configuration 控制 Development、Shipping、Debug、Test | CLI 与数据模型必须同时保留两个维度，不能复用同一个参数名 |
| 当前状态只描述单次编译 | `build status` 记录 PID、起止时间、exit code、mutex 和一份 UBT log | 发布任务要记录阶段、当前步骤、每步命令、每步日志、制品和失败类型 |
| task Host 与发布输出来源未定义 | 当前 task build 总是编 Host Editor target；UEB 可从任意项目或插件根构建 | 原生命令必须明确支持 `task` 与 `workspace` 两种来源，默认值不能猜错项目 |

## 兼容判断

| 等级 | 能力 |
| --- | --- |
| 可直接复用 | workspace 环境解析、task ref、Host、项目与引擎路径、结构化进程启动的一部分、日志目录、后台 PID 和 UBT 预检 |
| 适配后兼容 | 项目 Editor 编译、严格 profile、mutex、task 与 workspace 输入、后台状态 |
| 需要新实现 | BuildCookRun、BuildPlugins、依赖感知插件发布、引擎构建、Installed Build、流水线、dry-run/explain、统一 JSON、manifest、diagnostics |
| 当前冲突 | RunUAT 门禁、`-NoMutex` 政策、单次编译状态模型、profile 与 UE configuration 的命名边界 |

## 建议的原生命令边界

UnrealDevFlow 应保留自己的 workspace 和 task 身份模型，同时提供与 UEB 可观察行为等价的发布命令。建议命令层至少覆盖：

1. `udf build project` 保留开发期 Editor 编译。
2. 新增项目 package，支持 workspace 或 task Host、platform、configuration 和 archive dir。
3. 新增插件 package，支持指定 task primary、显式 seed、全部 seed 和目标平台。
4. 新增 engine build 与 engine installed。
5. 新增 pipeline plan、run、status，并让 dry-run 与 JSON 成为所有发布命令的共同能力。

具体命令归组和兼容旧命令属于产品设计决定，应在这份事实调查之后单独设计。

## 验证证据

2026-08-13 在 UEB 仓库运行九个聚焦测试文件，覆盖业务 golden、项目 build/package、插件依赖构建、引擎 build/installed、pipeline、result 和 diagnostics。结果为 9 个测试文件通过，96 条测试通过，0 条失败。

## 未覆盖与未知

- 本轮没有运行真实 UE 引擎构建，源码参数与测试契约已经核对，真实耗时和真实制品内容未重新验证。
- 本轮没有把 UEB 自身的 `scripts/release.ts` 当成用户编译能力。它发布 UEB 可执行文件，不属于 UE 项目、插件或引擎发布构建命令。
- UEB 明确不负责签名、版本写入、上传、通知和生产发布。这些能力不应因“原生等价 UEB”自动进入 UnrealDevFlow 范围。
