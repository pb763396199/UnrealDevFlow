---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1XHET6S2W0P8TVE8WKSH4D0
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-07T17:05:00+08:00
producer: aes-research
result: complete
topic: package-storage-lifecycle
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts: []
---

# 项目打包存储和清理调查

## 问题和停止条件

本次调查回答三个问题：UDF package 还会把数据写到哪里，哪些目录能证明归属，现有命令为什么漏报。
停止条件是列全源码声明的路径、执行记录指向的现存路径和本机上可识别的旧路径，并把观察和推断分开。

调查时间是 2026-09-07。安装版是 `udf 0.5.0 (git 2a516b50d, built 2026-09-03T08:47:33Z)`。
代码基线是 `284525b422739bd6b2689bf1e7aca9c08fc1835a`。

## 查过的地方

| 来源 | 查法 | 用途 |
| --- | --- | --- |
| `src/commands/package.rs` | 查临时目录、缓存键、执行记录、交付和清理代码 | 确认谁创建、谁保留、谁删除 |
| `src/package_profile.rs` | 查 profile 的归属和 revision | 确认 workspace 与 task 怎样分开 |
| `src/package_storage.rs` | 查空间估算和磁盘保护 | 确认预检统计范围 |
| `tests/package_lifecycle.rs` | 统计已有生命周期测试 | 确认哪些行为还没有回归保护 |
| `C:\Users\YUMEI\.unrealdevflow\executions\package` | 解析 49 份 JSON 记录和 `latest` | 把路径对回 execution、workspace 和 task |
| 本机目录 | PowerShell 逐目录统计普通文件；另查 Junction 和进程命令行 | 得到当前字节数并排除正在使用项 |

可复算命令：

```powershell
Get-ChildItem -LiteralPath "C:\Users\YUMEI\AppData\Local\Temp\UDF" -Recurse -File -Force |
  Measure-Object Length -Sum
Get-ChildItem -LiteralPath "C:\Users\YUMEI\.unrealdevflow\package\cook-cache" -Recurse -File -Force |
  Measure-Object Length -Sum
udf package clean --dry-run --format json
Get-CimInstance Win32_Process |
  Where-Object CommandLine -Match "Temp\\UDF|package\\cook-cache"
```

## UDF 明确拥有的现存数据

| 位置 | 字节数 | 里面是什么 | 当前命令能否完整报告 |
| --- | ---: | --- | --- |
| `%TEMP%\UDF` | 332,136,061,561 | 3 份项目 staging，3 份插件 staging | 不能。旧记录缺 `targetKind`，clean 会忽略 |
| `~\.unrealdevflow\package\cook-cache` | 679,088,238,377 | 8 份 iterate 项目副本和 Cook 结果 | 不能。它们没有进入 `cleanupTargets` |
| `UGA\DEV_1\Saved\UnrealDevFlow` | 10,903,253,265 | 一份已交付事务的旧包备份和日志 | 不能。对应旧记录缺 `targetKind` |
| `UGA\DEV\Saved\UnrealDevFlow` | 95,338,153 | 10 次 package execution 的日志 | 能报告新格式记录 |
| 两个 task Host 的 `Saved\UnrealDevFlow` | 4,226,948 | 任务打包日志 | 能报告新格式记录 |
| `~\.unrealdevflow\executions\package` | 406,946 | 执行 JSON、latest 和一份 engine 日志 | 记录本身应保留，受管日志可清理 |
| `%TEMP%\UnrealDevFlow\package-stage` | 331,608 | 0.4 版以前的插件 staging | 不能。新 cleaner 没有扫描旧根目录 |

上表合计 1,022,227,856,858 bytes。`udf package clean --dry-run` 只报告 99,565,101 bytes，
当前盘点漏掉约 99.99%。本机 C 盘当时剩余 348,025,163,776 bytes。

`C:\Package` 另有 44,313,955,076 bytes。这里是带 `.udf-manifest.json` 的最终包，属于用户交付物，
不能计入自动清理候选。

## 不能证明由 UDF 独占的数据

| 位置 | 字节数 | 判断 |
| --- | ---: | --- |
| `UGA\DEV\Saved\Cooked` | 23,210,000,000 左右 | 时间与旧版原地打包吻合，但 UE Editor、UAT 或人手命令都可能写入 |
| `UGA\DEV\Saved\StagedBuilds` | 11,426,000,000 左右 | 与上项同一时间生成，现有 execution 没有所有权字段 |
| 三个项目的 `Intermediate` | 75,608,000,000 左右 | 编译、Editor 和 package 共用，不能交给 package clean |
| `Saved\Logs\UnrealDevFlow` | 84,858,130 | 属于 run 子系统，不应混进 package 清理 |
| `%TEMP%\udf-*` | 29,584 | 仓库测试或评审留下的目录，不属于安装版 package 数据 |

这些目录可以出现在诊断报告里，但自动清理必须保持排除。旧版原地打包产生的 `Cooked` 和
`StagedBuilds` 只能作为人工候选，不能通过当前记录证明所有权。

## 已确认的原因

### Profile revision 会制造新副本

`cook_cache_root()` 在 `src/commands/package.rs:39-54` 把 `task_uid`、项目、引擎、配置、容器、
包名和 `profile.revision` 一起算进目录名。改输出名称、修改说明或任何会增加 revision 的配置都会换目录。
旧目录没有索引淘汰，也没有容量、数量或时间限制。

当前 8 份缓存中，`c20e7e7f47adb0d3d0db3788778b708d` 对应当前
`workspace/neon-dev` revision 7，占 98,586,863,880 bytes。其余 7 份合计
580,501,374,497 bytes，分别来自旧 revision、失败执行或中断执行。三份目录缺
`.udf-cook-cache.json`，不能复用。

### Iterate 缓存没有清理入口

`src/commands/package.rs:1702-1708` 在 iterate 模式下只把日志放进 `cleanupTargets`。
`src/commands/package.rs:1890-1891` 明确保留项目副本。`package clean` 在
`src/commands/package.rs:2356-2468` 只遍历 execution 的 `cleanupTargets`，所以不会看到 cook-cache。

### 任务插件变化没有参与复用判断

`src/commands/package.rs:1593-1613` 先对源项目计算摘要并决定能否复用。
任务插件 overlay 到 `src/commands/package.rs:1629-1660` 才收集。复用判断没有读取任务 worktree
中主插件的内容。由此推断，任务插件改了而源项目摘要没变时，旧 staging 仍可能被判为可复用。
这项推断需要一条回归测试确认。

### 已交付备份会一直保留

`publish_directory()` 在 `src/commands/package.rs:542-644` 把被覆盖文件复制到
`delivery-backup`，交付完成后把事务标成 `delivered`，没有删除备份。
`package recover` 只接受 `delivering`，见 `src/commands/package.rs:2486-2492`。
所以 `delivered` 后的备份不能用于现有恢复命令，却仍可能保留一整份旧包。

本机 `UGA\DEV_1\Saved\UnrealDevFlow\package-project-20260901T080425Z-938` 中的
`delivery-backup` 占 10.90 GB，事务状态是 `delivered`，记录的最终输出已不存在。
删除前应由人决定是否把它当作仅存旧包保留。

### 空间预检低估下一次占用

`src/package_storage.rs:12-14` 把无历史输出时的估算定为 1 GiB，
`src/package_storage.rs:115-134` 只统计传入的当前 staging 和日志。
现场一份完整缓存占 90.47 GB 到 122.40 GB。当前估算无法阻止连续创建 100 GB 级副本，
也看不到别的 cache key 已占的空间。

### 清理安全和记账还有四个缺口

| 缺口 | 代码证据 | 后果 |
| --- | --- | --- |
| 旧记录缺 `targetKind` | `src/commands/package.rs:2401-2414` | cleaner 保守跳过，旧目录长期留存 |
| workspace 或 task 范围可直接删除 | `src/commands/package.rs:2374-2446` | 范围删除没有 `--yes` 门槛 |
| 范围删除只更新第一条记录 | `src/commands/package.rs:2447-2450` | 文件删了，其他 execution 仍显示旧状态 |
| 受管路径按目录名判断 | `src/commands/package.rs:156-173` | 任意路径只要含 `UnrealDevFlow` 或 `.udf-logs` 就可能通过 |

插件包从 task 取源码时没有把 `requested_task` 写进 `PackageMetadata`，见
`src/commands/package.rs:2058-2060` 和 `2114-2118`。项目包会在 `1795` 和 `1922`
写 `taskRef`。两类包的任务过滤行为不一致。

## 仍不知道的事

- UE 的 iterate Cook 是否需要长期保留 `Saved\StagedBuilds`。源码已经证明 `Archive` 会在下一次复用前删除，
  所以 `Archive` 不属于复用所需数据。`StagedBuilds` 需要真实 UE 包回归后再决定。
- `UGA\DEV\Saved\Cooked` 和 `Saved\StagedBuilds` 是否全部由 2026-08-31 的旧 UDF 产生。
  时间吻合，但现有记录不足以排除其他 UAT 或 Editor 操作。
- 10.90 GB 的 `delivery-backup` 是否仍有业务保留价值。代码认为事务已交付，最终输出目前不存在。

## 给设计阶段的输入

设计需要同时解决五件事：完整盘点、稳定的复用目录、活动执行保护、旧记录处理、最终包保护。
只增加一个删除命令会继续漏掉 task overlay、空间估算和事务备份问题。
