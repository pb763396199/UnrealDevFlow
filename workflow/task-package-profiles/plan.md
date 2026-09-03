---
schema_version: 1
protocol: 1.3.0
artifact: plan
artifact_id: ar_01M1GD7JRRHR8QXDXHPXFTJ2Q8
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-02T00:00:00Z
producer: aes-plan
result: ready
supersedes: ar_01M1E9DZ0Q59K7095AB2BMHTDA
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1GCKQ5K74MTZJ9FFZK74M54
      digest: sha256:174e055c18b8c7c4af7126964a7b983ad9df997c24c51b8a0891d68b3739089b
      locator: design/package-design-v5.md
    - artifact_id: ar_01M1GCKPYPHYGFPR60YHRY9K99
      digest: sha256:0f7a4ab94e8ed3c2e73693abb80d022fa648c867068a26713c741f7479e533a1
      locator: research/package-scope-research.md
---

# 项目包范围与生命周期施工计划 v2

## 施工目标

把项目包作为唯一默认打包入口，严格复用项目原生 Packaging 设置与任务固定 profile；把插件包和 Installed Build 降为显式高级工具链；在 UBT 运行前暴露空间与版本风险；用 manifest 做安全交付和回收，避免误打目标、重复全量 Cook、旧文件残留和临时目录失控。

## 不变的边界

- 普通任务只打 UE 项目包，不自动打插件包、引擎包或 Installed Build。
- 不修改 UE 项目、AesWorld、Engine 目录及其 authored 配置；只读项目设置并在 UDF profile 中保存用户明确的覆盖。
- 每个任务固定一个 profile；只有显式 configure 才能改变它。
- 最终包永不因 package clean 或新一轮打包而自动删除；比较包必须显式命名并说明原因。

## 分阶段实施

| 阶段 | 目标与改动 | 先行测试 | 完成证据 |
| --- | --- | --- | --- |
| 1. CLI 范围收敛 | 在 `src/cli.rs`、`src/main.rs` 建立 `package project` 普通路径与 `package advanced plugin/engine` 高级路径；`run` 保持 project-only 兼容；旧 `package plugin/engine` 只输出迁移提示、不得启动 UBT/BuildGraph；`check/plan` 默认只接受 project。 | `tests/cli_taxonomy.rs` 验证普通 help 不把 plugin/engine 当同级日常入口，advanced 可解析，旧别名安全失败。 | CLI 解析测试通过；高级目标不能从 project/run 隐式到达。 |
| 2. 空间与版本预检 | 在 `src/commands/package.rs` 抽取 UDF 制品目录、临时目录、持久 iterate cache、卷剩余空间、同任务兄弟版本/lineage 扫描；给 `plan/check/project` 统一返回 `ready|warning|blocked`、当前/预计字节和下一步。 | `tests/package_lifecycle.rs` 覆盖低空间阻断、近阈值警告、兄弟版本统计、无 UBT 调用。 | blocked 时没有启动 UAT；JSON/human 输出字段一致。 |
| 3. 执行记录扩展 | 扩展 `PackageResult` 与迁移逻辑，记录 `targetKind`、profile/source revision/digest、lineage、输出/临时字节、估算/剩余空间、space decision、cleanup policy/result；旧 JSON 迁移为 project 或 unknown，unknown 不授权删除。 | `tests/package_profile.rs`、`tests/package_lifecycle.rs` 验证旧 JSON 反序列化、字段完整、未知目标无法清理。 | 新旧 execution record round-trip，status 可读。 |
| 4. 原生配置与固定 profile 接口整理 | 保持既有 native settings snapshot、`iterate/full` 和 `--name` 语义；确保 project 只从 profile 生成 project UAT 命令，不把普通路径升级成 plugin/engine；比较包要求 configure `--name` + `--reason`，同名输出默认刷新。 | `tests/package_commands.rs`、`tests/package_profile.rs` 验证 Shipping/容器/压缩/地图/native flags、iterate 复用条件与命名。 | plan 中能看到 effective settings、来源、Cook 复用判定和 lineage。 |
| 5. Manifest 交付与自动清理 | 修改 `publish_directory` 使用 manifest reconciliation：只覆盖 UDF 拥有文件，删除上一 manifest 中已消失的 UDF 文件，保留用户文件/Junction/未拥有文件；full 成功后删除 execution stage/archive，保留最终包、manifest、必要日志和 record；iterate 保留一个按 task/profile/engine/platform/container 复用的 cache。 | `tests/package_lifecycle.rs` 覆盖旧 UDF 文件删除、用户文件保留、Junction 拒绝、full 清理、iterate cache 稳定。 | 交付事务可 recover；失败保留可恢复材料；成功不残留 stage/archive。 |
| 6. clean、帮助和文档 | `package clean` 支持显式 execution、task/workspace scope 与 `--dry-run`；无参数只报告可回收空间；不会删除 final output。同步 `README.md`、`skills/unrealdevflow/SKILL.md` 和帮助文案，明确日常 iterate / 发布 full、高级目标和比较包风险。 | CLI help/解析测试，clean dry-run 与 scope 测试。 | 文档命令表单一、逻辑顺序清晰，不再诱导 AI 打插件/引擎。 |
| 7. 集成验证与记录 | 执行 fmt、clippy、全量 cargo test、CLI help smoke；记录 changed files、风险和未执行的真实 UE UAT 验证。真实 UE 项目测试只在用户明确要求时进行。 | `cargo fmt --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test`。 | 验证记录通过，代码提交与 workflow 记录分离。 |

## 关键接口语义

```text
udf package project --task <workspace/task-id>
udf package project --workspace <workspace>
udf package plan project ...
udf package check project ...
udf package advanced plugin <Plugin> --workspace <workspace>
udf package advanced engine --workspace <workspace>
udf package run --workspace <workspace>          # project-only compatibility alias
udf package clean [<execution-id>] [--task <ref>|--workspace <name>] [--dry-run]
```

普通 project 路径不能通过参数选择 plugin/engine；高级路径必须明确写出 target。旧命令可以保留解析兼容，但只返回 needs-user-choice 与迁移命令，不能静默执行危险目标。

## 数据与安全约束

- 先解析 profile 和源指纹，再计算空间，最后才允许启动 UAT；`blocked` 永不进入 UBT。
- 输出目录必须是普通目录；输出内的 Junction 和非 UDF 文件不得被接管。
- manifest 是 UDF-owned 删除授权的唯一来源；旧记录没有可信 target/manifest 时只能报告，不能猜测删除。
- 比较包不复用另一个 lineage 的 final output；同任务版本列表、累计占用和预计新增量必须在执行前可见。
- 成功 cleanup 是执行结果的一部分；cleanup 失败必须记录 warning，不得伪报“已清理”。

## 验收标准映射

- AC-014：阶段 1、6；普通 task package 只生成 project，plugin/engine 仅显式 advanced。
- AC-015：阶段 2、5、6；空间预警、比较包风险、成功自动清理临时交付目录。
- 既有 native/profile AC：阶段 3、4、7；保持配置固定、原生 argv、Cook 复用和旧记录兼容。

## 回滚

每阶段独立提交；若集成验证失败，回退最近阶段代码提交，保留 workflow 记录和失败证据。不得删除用户最终包或用户文件；只清理 manifest 明确的 UDF 临时目录。
