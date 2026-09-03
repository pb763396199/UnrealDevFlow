---
schema_version: 1
protocol: 1.3.0
artifact: implementation
artifact_id: ar_01M1K2962MV7BJDAWW3KPG1TAP
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-03T06:30:00Z
producer: aes-execute
result: complete
supersedes:
  - ar_01M1JP3MJDQ7BPJR4MCTTCS5EW
  - ar_01M1JP3MJDG785GAW51M12CEV7
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1GCKQ5K74MTZJ9FFZK74M54
      digest: sha256:174e055c18b8c7c4af7126964a7b983ad9df997c24c51b8a0891d68b3739089b
      locator: design/package-design-v5.md
    - artifact_id: ar_01M1GD7JRRHR8QXDXHPXFTJ2Q8
      digest: sha256:25c4d285e83cf0d34053512bcad767ed1b3766b4431d6eb7c794e87e1d5eee0d
      locator: plan.md
  subject:
    kind: change_set
    digest: sha256:4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: a817dd2473ebe5fa18ceb0168be9692bf905fcea
    revision: 71085c0b785e41317d03c43fa4d9747c58b8c125
    tree: 08907cbb20d394efbb6ec06b13d3fcca9454c266
    content_digest: sha256:169f8e6829e36f04a2286d4b419f80c9abdd76033d5d043a05d1f8eb599b7055
    branch_or_pr: feature/task-package-profiles
    workflow_excluded: true
---

# 实现回执

## 本轮补缺口

- 修复 `task create` 不必要地拒绝主仓未提交修改的问题：现在以当前 `dev` 的已提交 HEAD 创建 Host，脏文件保留在主仓且不会被带入任务。
- 新增主仓脏状态端到端回归测试，覆盖 `output/` 与 workflow 文件等未跟踪内容。

- 新增 `src/task_routes.rs`，在任务切换完成前持久化任务、Host 和主项目路径；清理完整成功后删除对应路由。
- `cleanup_for_missing_task` 在当前配置和 state 之外读取持久化路由，覆盖 Host、state 和 workspace 项目路径同时变化的恢复场景。
- `cleanup_for_task` 纳入任务冻结的 `context.default_project`，避免配置路径漂移后漏清旧项目 Junction。
- metadata 中的 dependency Junction 路径拒绝空值、绝对路径、根路径和 `..`，异常项计入失败结果，阻止误报完整清理。
- 新增配置项目无 state、任务 context 路径漂移、持久化路由恢复和 Host 外路径保护回归测试。

## 本轮验证

`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 和
`cargo test --all --no-fail-fast` 全部通过；单元测试 67 个，集成测试全部通过。最终代码提交为
`71085c0b785e41317d03c43fa4d9747c58b8c125`，已快进落地到 `dev`。

## 做了什么

- 新增 `package configure`，按 task 或 workspace 保存固定打包配置，并要求更新已有配置时提供原因。
- 将 Shipping、loose、Pak、IoStore、固定输出目录和重复 `disable-plugin` 接入项目打包计划。
- 对配置项目建立受管临时副本，只修改副本 `.uproject`，不改开发项目、Host 描述或 DEV Junction。
- 新增 `package recover`，交付中断时依据事务日志和文件摘要回退，拒绝猜测性删除。
- 将 Cook 配置明确分为日常开发的 `iterate` 与完整发布的 `full`；新 profile 默认 `iterate`，旧 profile 保持原有 `full` 语义。
- 在 `package configure --help` 与 `package project --help` 中说明两种模式、缓存复用条件，以及 iterate 不保证 Pak 不重建。
- 增加稳定 `--name`，并让项目、插件、Installed Build 都能指定 `--output` 根目录。
- 预检断链和插件索引做快速路径，避免真实大工程在明显错误上长时间复制或扫描。
- 增加配置摘要、输出目录锁、Junction 保护、执行 running/failed 记录和中文使用文档。
- 对齐本机成功 UAT 日志中的 `target`、`unrealexe`、`installed`、`skipbuildeditor` 和 `nocompile*` 参数。
- 失败的 Installed Build 现在登记输出目录，`package clean` 可清掉 UAT 已创建的空输出根目录。
- staging 跳过 IDE 与 Agent 元数据目录，避免把无关目录复制进项目临时副本。
- 新增 `src/task_junctions.rs`，在 `cleanup/delete` 中统一清理项目侧 Junction、悬空 Junction 和 Host 内项目依赖 Junction；缺失 Host 时按 workspace 推导路径恢复清理。
- 删除前按目标路径再次过滤候选，只处理位于当前任务 Host 下的链接；清理失败会进入结果状态，不能伪报完整成功。

## 逐步结果

| 步骤 | 结果 | 证据 |
| --- | --- | --- |
| A 配置和 CLI 测试 | 完成 | `tests/package_profile.rs`、`tests/package_commands.rs`、`tests/cli_taxonomy.rs` |
| B 配置模型和写入 | 完成 | 嵌套 TOML、revision、原因、候选文件和直接修改拒绝测试通过 |
| C 打包计划接入 | 完成 | `-clientconfig=Shipping`、loose 不含 `-pak`、IoStore 参数测试通过 |
| D 项目隔离和插件排除 | 完成 | 副本测试证明源 `.uproject` 不变；Junction 按链接处理 |
| E 执行记录与空间 | 完成 | `targetKind`、profile/source/lineage、空间决策和 cleanup 结果已接入；旧记录缺字段时按 unknown 处理 |
| F 事务交付和恢复 | 完成 | manifest 摘要交付、过期 UDF 文件回收、Junction/用户文件保护、逐插件日志和 recover 已接入 |
| G 文档和静态验证 | 完成 | 63 个单测、8/19/5/23/6/8/15/6/3/2 个集成测试组通过，fmt、Clippy 和 CLI smoke 通过 |

## 改了哪些代码

| 文件 | 修改 |
| --- | --- |
| `src/package_profile.rs` | profile 绑定、嵌套 TOML、revision、摘要校验、候选接纳和原子替换 |
| `src/project_packaging.rs` | 读取项目 Packaging、Cooker、地图和平台配置，并生成稳定摘要 |
| `src/package_storage.rs` | 输出/临时目录容量、卷剩余空间、兄弟版本和空间决策 |
| `src/commands/package_profile.rs`、`src/cli.rs`、`src/main.rs` | `configure` 与 `recover` CLI 接线 |
| `src/commands/package.rs` | profile 消费、项目副本、插件覆盖、执行状态、交付事务、锁和恢复 |
| `src/task_junctions.rs`、`src/commands/cleanup.rs`、`src/commands/delete.rs` | 统一任务 Junction 生命周期清理，覆盖悬空链接、缺失 Host、Host 依赖保护及失败状态 |
| `src/ue_commands.rs` | 配置枚举和容器到 UAT 参数的映射 |
| `tests/package_profile.rs`、`tests/package_commands.rs` | 配置、参数和直接修改回归测试 |
| `README.md`、`skills/unrealdevflow/SKILL.md` | 使用入口、MCP 排除和固定配置说明 |

## 自审发现的问题

| 问题 | 怎么发现的 | 修了没有 |
| --- | --- | --- |
| 初版 profile 用扁平字段，和 v2 示例不一致 | 增加嵌套 TOML 回归测试时发现 | 已修复 |
| 固定输出目录不能作为 clean 的删除目标 | 对 profile 输出为外部目录的语义检查 | 已修复，profile 只清理日志和临时副本 |
| 真机 UAT、完整地图策略和 sha256 manifest 尚未覆盖 | 对照计划 F/G 和当前测试范围 | 列入人工验收或后续增强；当前 manifest 使用仓库已有 MD5 |
| DEV 的断链未在执行前快速报告 | 真实 `package run` 先耗时复制后失败 | 已修复，`package check` 与执行入口都在复制前阻塞 |
| 新 profile 默认会不会误走完整 Cook | 用户要求区分日常开发与完整发布 | 已修复，新 profile 默认 iterate，full 必须显式 configure；旧 profile 兼容为 full |
| 包目录默认仅按平台命名会冲突 | 真实 plan 检查和用户新需求 | 已修复，固定配置保存 name，默认名含项目、任务、平台和配置 |
| UAT 参数缺少成功日志中的项目目标和已安装引擎选项 | 对照 `UGA-backup-2026.08.31-05.45.18.log` 与本机 UE5.5 `ProjectParams.cs` | 已修复，并由 `tests/package_commands.rs` 锁定 |
| Engine 失败只保留日志目录，留下 `UE55-InstalledBuild-Win64` 空目录 | 实际 `package engine --output C:/Package --name UE55-InstalledBuild-Win64` 后检查清理目标 | 已修复，并增加失败清理回归测试 |
| 项目 staging 复制 IDE 与 Agent 元数据 | 临时副本中出现 `.claude`、`.codex`、`.vscode` 等目录 | 已修复，并由 staging 回归测试锁定 |
| cleanup/delete 删除 Host 后残留项目悬空 Junction | 会话 A 删除 Host/worktree 后 DEV 仍保留 Junction；源码同时存在 `Path::exists()` 漏判 | 已修复，统一清理入口使用文件系统元数据并先于 Host 删除执行 |

## 哪里没按计划走

- 没有新增 `src/execution.rs` 共享执行模型，先在现有 package runner 中落地最小可用的 running/failed 记录，避免并行维护第三套状态实现。
- 受现有依赖约束，配置摘要和交付清单使用仓库已有 MD5 能力；设计要求的 sha256 文件 manifest 仍未实现。
- 本轮没有启动真实项目 UAT/Cook，不能把 Rust 测试通过写成 Shipping 真机包通过。
- 交付清单使用仓库已有 MD5 摘要，尚未替换为设计中的 sha256 文件摘要。

## 跑过的检查

- `cargo fmt --all`：通过。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo test --all --no-fail-fast`：63 个单元测试及全部集成测试通过。
- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过。
- CLI smoke：普通 help 仅显示 project 及 advanced 分组；旧 `package plugin` 返回迁移提示；无参数 `package clean` 只盘点。
- 自动验收：AC-007 至 AC-010、AC-012 至 AC-015 通过；AC-001 至 AC-006、AC-011 仍需人工清单。
- 真实 `package plan/check/run project`：plan 生成 `C:\Package\UGA-Win64-Shipping-20260901`；check/run 明确报告 DEV 的 2 个断链。
- 真实 `package plugin AesWorld --output C:/Package`：UBT 到 1721/1727 后以退出码 6 失败，日志明确指向 `AesStreamingIoDispatchTests.cpp` 的源码错误。
- 真实 `package engine --output C:/Package --name UE55-InstalledBuild-Win64`：退出码 1，日志显示 UE5.5 安装版缺少 `Engine/Build/InstalledEngineBuild.xml`。
- 真实 `package recover` 无事务日志安全拒绝；`package clean` 对旧日志目录成功清理。
- `workflow_tool.py change-digest --repo .`：通过，变更集摘要为 `sha256:4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945`。
- `cargo test --test multi_plugin cleanup_missing_host_removes_dangling_project_junctions_before_branch_cleanup -- --exact`：通过。
- `cargo test --test multi_plugin delete_removes_dangling_project_junctions_before_host_and_worktree_cleanup -- --exact`：通过。
- 全局安装核对：`C:\Users\YUMEI\.unrealdevflow\bin\udf.exe`，版本 `udf 0.4.1 (git b1ac54964-dirty, built 2026-09-02T07:59:48Z)`。

## 剩余风险

真实工程仍需验证：在用户指定项目上执行日常 iterate 与完整发布 full 两条路径、禁用 MCP 插件、检查最终固定目录、故意制造交付中断后运行 recover，以及确认外部 Junction 和用户存档未被触碰。`C:\Package\Windows` 是用户手工包，本轮未修改。
