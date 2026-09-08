---
schema_version: 1
protocol: 1.3.0
artifact: plan
artifact_id: ar_01M1Y6N5EPKA8HMNC9E6JGKQS4
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-07T23:09:20+08:00
producer: aes-plan
result: ready
supersedes: ar_01M1XJT7WQS5ZWSN4E5YYMJGP1
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts:
    - artifact_id: ar_01M1Y6N59V3BEHVPA4V0V2ZT57
      digest: sha256:e0d82755fc5a8d13b890078410c7572bc8f6a7839ed2bf1da644ab3cb800372b
      locator: design/plugin-stage-space-design.md
---

# 项目打包缓存清理和复用施工计划

## 开工条件

| 项目 | 值 |
| --- | --- |
| 仓库 | `https://github.com/pb763396199/UnrealDevFlow.git` |
| worktree | `F:\AiProject\UnrealDevFlow\.aes-workflow\worktrees\package-cache-lifecycle` |
| 分支 | `fix/package-cache-lifecycle` |
| 基线 | `284525b422739bd6b2689bf1e7aca9c08fc1835a` |
| Work Item | `ns4dy38b` |
| 设计 | `ar_01M1XHETDT2NJ5JXMTTYS20RFM`，`accepted` |
| 用户选择 | B，成功旧 staging、旧 Archive 和 10.90 GB delivered backup 均确认删除 |

执行阶段只能改这个 worktree。现有用户级 package 目录在最后两步前保持只读。

## 步骤依赖

```mermaid
flowchart LR
    S1[写失败测试] --> S2[缓存身份和租约]
    S2 --> S3[统一盘点]
    S3 --> S4[clean 命令和归属]
    S4 --> S5[打包复用和交付清理]
    S5 --> S6[空间规则]
    S6 --> S7[文档]
    S7 --> S8[代码验证]
    S8 --> S9[真机 dry-run]
    S9 --> S10[执行 B 清理]
    S10 --> S11[清理后验证]
```

## 施工步骤

| ID | 做什么 | 文件 | 验证与预期证据 | 退回办法 |
| --- | --- | --- | --- | --- |
| S1 | 先补现有缺口的失败测试。覆盖完整 inventory、旧根目录、稳定 cache key、overlay 变化、活动租约、范围确认、逐记录更新、plugin `taskRef`、最终包保护和 delivered backup | `tests/package_lifecycle.rs`、`tests/package_profile.rs`、`tests/cli_taxonomy.rs`、`tests/package_commands.rs`；新增 `src/package_cache.rs` 和 `src/package_inventory.rs` 的测试骨架 | 运行 `cargo test --test package_lifecycle -- --nocapture`、`cargo test --test package_profile -- --nocapture`、`cargo test --test cli_taxonomy -- --nocapture`。新增用例必须因旧行为失败；保存失败用例名和断言 | 只撤销新增测试和两个测试骨架，不动现有测试 |
| S2 | 实现稳定缓存槽、schema 2 状态、排他锁、PID 与进程开始时间租约、默认保留规则。目录 key 去掉 revision、包名、输出和修改理由。源项目、项目设置、禁用插件和 overlay 摘要只参与状态校验 | `src/package_cache.rs`、`src/main.rs`、`src/config.rs`、`src/package_profile.rs` | 运行 `cargo test package_cache::tests -- --nocapture`。只改 revision、name、output、reason 时 key 相同；改 task、project、engine、platform、configuration、container 时 key 不同；有效租约不可清理 | 删除新模块声明，恢复旧 `cook_cache_root()` 调用；不迁移用户目录 |
| S3 | 实现统一只读盘点。扫描 execution JSON、两个临时根、cook-cache、profiles、日志、事务备份、manifest 和最终输出。逐项输出归属、字节、状态、活动、保护理由、清理理由和可信程度。统计不穿过 Junction | `src/package_inventory.rs`、`src/package_storage.rs`、`src/commands/package.rs` | 运行 `cargo test package_inventory::tests -- --nocapture`、`cargo test package_storage::tests -- --nocapture` 和 `cargo test --test package_lifecycle clean_inventory -- --nocapture`。fixture 中所有受管目录各出现一次，最终包只出现在 protected，Junction 目标字节不计入 | 保留旧 `package clean` 路径，把新 inventory 模块从入口移除 |
| S4 | 收紧 `package clean`。加入 `--cache`、`--stale`、`--legacy`、`--yes`；多项删除缺 `--yes` 时退回 dry-run。受管路径改用固定根白名单。范围删除逐条更新 execution。plugin task 来源写 `taskRef` | `src/cli.rs`、`src/main.rs`、`src/commands/package.rs`、`tests/package_lifecycle.rs`、`tests/cli_taxonomy.rs` | 运行 `cargo test --test package_lifecycle -- --nocapture` 和 `cargo test --test cli_taxonomy -- --nocapture`。无参数零删除；task/workspace 多项删除缺 `--yes` 时零删除；旧记录证据不足时拒绝；所有已删记录都变成 cleaned；最终包无法成为 target | 保留 exact execution clean，移除新增范围开关；恢复前先跑原有 lifecycle 测试 |
| S5 | 修正项目打包复用和交付。task overlay 在复用判断前算摘要。摘要不匹配时清空同槽并完整重建。成功交付后核对目标摘要，再删除 `Archive` 和 backup；失败保持 `delivering`，继续支持 recover | `src/commands/package.rs`、`src/package_cache.rs`、`tests/package_commands.rs`、`tests/package_lifecycle.rs` | 运行 `cargo test commands::package::tests -- --nocapture`、`cargo test --test package_commands -- --nocapture`、`cargo test --test package_lifecycle delivery -- --nocapture`。overlay 改动必须令 `cookReused=false`；delivered 后 backup 和 Archive 不存在；注入复制失败后 backup 与 journal 仍存在，recover 通过 | 恢复旧交付流程。用户目录尚未清理，因此没有数据回退动作 |
| S6 | 修正空间预检。统计全部受管 package 数据；估算取同项目最近成功缓存、源输入 2.5 倍和 1 GiB 三者最大值；应用 2 槽、7 天、24 小时、14 天、30% 或 250 GiB、10% 或 50 GiB 的规则。跨 binding 只报告，不自动删除 | `src/package_storage.rs`、`src/package_cache.rs`、`src/config.rs`、`src/commands/package.rs` | 运行 `cargo test package_storage::tests -- --nocapture` 和 `cargo test package_cache::tests -- --nocapture`。fixture 中 120 GB 历史缓存不能再被估成 1 GiB；超过保护线时返回 blocked 和精确候选；当前槽、租约槽和别的 task 不进入自动删除 | 恢复旧 `assess()`，保留 inventory 只读输出 |
| S7 | 更新命令说明、默认规则、旧记录行为和最终包保护。文档明确区分 UDF task 与 AES Work Item | `README.md`、`skills/unrealdevflow/SKILL.md`、`docs/releases/v0.5.0.md` | 运行 `rg -n "package clean|cook-cache|stale|legacy|taskRef" README.md skills/unrealdevflow/SKILL.md docs/releases/v0.5.0.md`，确认三处命令和默认值一致；运行写作检查，错误为 0 | 只撤销文档步骤，不影响代码 |
| S8 | 验证全部代码并生成真实二进制 | 所有本任务代码文件 | 依次运行 `cargo fmt --check`、`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test --locked`、`cargo build --release --locked`、`git diff --check`。全部退出码为 0；`target\release\udf.exe --version` 输出本分支 git hash 和新构建时间 | 任一检查失败就留在本步骤修复，不进入真机目录 |
| S9 | 用新 release 二进制对真实目录做 dry-run。记录删除候选、保护项、活动租约、预计释放字节和 `C:\Package` manifest 摘要 | 不改仓库代码；读取用户级目录 | 运行 `target\release\udf.exe package clean --dry-run --format json`、带 `--workspace neon-dev`、`--workspace neon-dev1`、`--task neon-dev/earth-scalability-baseline-profile` 的 dry-run。全局受管字节应覆盖调查中的 1,022,227,856,858 bytes；`C:\Package` 44.31 GB 全部 protected；当前 cache `c20e7e7f47adb0d3d0db3788778b708d` protected；进程检查无 UAT、UBT 或 package 使用候选目录 | dry-run 不写文件。任一归属或字节不吻合就停止，不执行 S10 |
| S10 | 执行用户选择 B。删除 6 份旧 `%TEMP%\UDF` staging、7 份非当前 cook-cache、10.90 GB delivered backup、旧 package-stage、到期日志和空 package-test。保留当前缓存、profiles、records、最终包和无法证明归属的项目共用目录 | 用户级 UDF 目录和 execution 记录 | 先再次解析每个绝对目标，确认都落在固定受管根且没有有效租约。运行 `target\release\udf.exe package clean --legacy --yes --format json`，再运行 `target\release\udf.exe package clean --stale --yes --format json`。以当前快照预计释放约 923 GB；命令逐项返回 deleted 或 protected，不能出现 guessed | 这一步按用户选择 B 不保留旧包副本，删除后只能重新 Cook 或重新打包。出现路径不一致、活动租约或外部修改时跳过该项并返回 partial，不扩大删除范围 |
| S11 | 清理后复核磁盘、目录、记录和最终包。把真实结果写进 implementation 与 validation，代码和 workflow 记录分开提交 | `workflow/package-cache-lifecycle/implementation.md`、后续 `validation.md` | 再跑全局和三个 scope dry-run。旧 Temp staging 和 7 份旧 cache 必须消失；当前 cache、两份 profiles、execution JSON 和 `C:\Package` manifest 摘要保持；`Get-PSDrive C` 显示释放字节；再跑 `cargo test --locked` | 若代码验证失败，回退代码提交。已删除缓存按设计属于可再生数据，只能通过新 package 执行重建 |

## 测试名称

新增测试至少包含：

- `cache_key_ignores_profile_revision_name_output_and_reason`
- `cache_key_separates_task_project_engine_platform_configuration_and_container`
- `overlay_change_rebuilds_the_same_cache_slot`
- `active_cache_lease_is_never_a_cleanup_candidate`
- `dead_running_record_becomes_stale_after_grace_period`
- `clean_inventory_reports_temp_cache_logs_backups_and_final_outputs`
- `legacy_record_requires_three_matching_ownership_signals`
- `scoped_cleanup_requires_yes_and_updates_every_record`
- `plugin_task_package_persists_task_ref`
- `delivered_backup_is_removed_only_after_digest_verification`
- `failed_delivery_keeps_backup_and_remains_recoverable`
- `final_output_is_never_an_automatic_cleanup_target`
- `storage_preflight_uses_global_usage_and_historical_size`

## 插件 staging 收缩补充步骤

| ID | 做什么 | 文件 | 验证与预期证据 | 退回办法 |
| --- | --- | --- | --- | --- |
| S12 | 先补两个回归测试：私有 plugin root 的只读目录是 Junction，`Intermediate/Binaries/Saved` 是普通私有目录；成功交付后 stage 被删除，失败交付仍保留 stage | `src/commands/package.rs` 单元测试、`tests/skills/aes-workflow/test_package_cache_lifecycle.py` | `cargo test commands::package::tests::plugin_stage_uses_read_only_junctions_and_private_generated_dirs -- --nocapture` 与 `cargo test commands::package::tests::successful_plugin_delivery_removes_private_stage -- --nocapture` 先失败 | 只删新增测试，不动现有 stage 逻辑 |
| S13 | 将 `prepare_plugin_stage()` 改为私有根文件加子目录 Junction。跳过源 `Intermediate/Binaries/Saved/DerivedDataCache`，在 stage 内创建前三个私有目录；交付摘要校验后删除 stage，交付异常写 failed execution 并保留 stage | `src/commands/package.rs` | 单元测试通过；成功路径的 execution 不再包含已删除的 stage，失败路径保留 cleanup target | 恢复原完整复制函数；不删除现有用户目录 |
| S14 | 把两条新验收接到 AES wrapper，并跑 targeted、全量 Rust 测试、fmt、clippy 和 release build | `tests/skills/aes-workflow/test_package_cache_lifecycle.py`、`tests/package_lifecycle.rs` | 新 wrapper case 退出 0；`cargo test --locked --all-targets`、fmt、clippy、release build 全部退出 0 | 不通过就停在本步骤修复，不进入真实目录 |
| S15 | 更新插件 staging 生命周期说明，使用新 release 二进制做只读 dry-run，确认旧目录仍受保护、新代码不会把最终输出列为候选 | `README.md`、`skills/unrealdevflow/SKILL.md`、任务记录 | `target\\release\\udf.exe --format json package clean --dry-run`；最终输出和 current cache 为 protected | 文档改动可单独撤回，真实目录不删除 |

## 提交顺序

| 提交 | 内容 | 前置步骤 |
| --- | --- | --- |
| C1 | 测试、缓存身份、状态和租约 | S1、S2 |
| C2 | 统一 inventory、clean 安全和 execution 归属 | S3、S4 |
| C3 | 复用修正、交付清理和空间规则 | S5、S6 |
| C4 | README、Skill 和 release note | S7 |

每条施工提交只包含代码或文档，不提交 `workflow/`。任务收口时再单独提交 workflow 记录。

## 风险

| 风险 | 控制办法 |
| --- | --- |
| 新 cleaner 把最终包当作缓存 | final output 在类型层固定为 protected；测试所有参数组合 |
| 后台 Cook 被清理 | 排他锁、PID、进程开始时间和更新时间四项共同保护 |
| task 插件改动仍复用旧缓存 | overlay 摘要在复用判断前计算，并用真实 task worktree 测试 |
| profile 切换后误用不兼容数据 | 稳定槽只负责目录复用，状态摘要不匹配时完整重建 |
| 清理中途失败造成记录失真 | 每项删除后单独更新对应 execution，未完成项保持原状态 |
| S10 删除不可恢复数据 | 用户已选择 B；S9 必须证明范围，路径或保护项有一处不符就停止 |

## 完成证据

计划完成时要留下：新增测试的失败与通过记录、全量 Rust 检查输出、新 release 二进制版本、清理前后 dry-run、
C 盘释放字节、保留目录清单、`C:\Package` manifest 摘要，以及逐条 AC 验收结果。
