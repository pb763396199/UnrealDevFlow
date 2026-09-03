---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1DY0G9HF11ZFAJKST8W2VAK
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-09-01T07:30:00Z
producer: aes-research
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
topic: UE 原生 Package Project 路径、项目配置和 Cook 复用语义
---

# UE 原生打包路径取证

## 结论

当前 UDF 的 Shipping 实测成功，但它采用的是 UDF 自己拼接的 BuildCookRun 参数和新的临时项目副本。它没有在启动前读取并展示项目的有效打包设置，也没有持久化 Cook 状态。因此它能成功执行 UE 流程，却不能证明它复现了用户在 Editor 中点击 Package Project 时的配置选择，也不能复用上一次 UDF 执行的 Cook 结果。

## 官方语义

Epic 的 Packaging 文档把打包拆成 Build、Cook、Stage、Package。By the book 是默认的最终测试路径；On the fly 面向快速开发验证。官方 Cook 命令参考明确说明：不带 `-iterate` 时会删除之前的 Cook sandbox 并重新 Cook；`-iterate` 只处理过期项。项目的 `MapsToCook` 只在命令行没有另外提供地图列表时生效，`DirectoriesToAlwaysCook` 和 `DirectoriesToNeverCook` 属于项目设置。

证据：

- https://dev.epicgames.com/documentation/en-us/unreal-engine/packaging-your-project
- https://dev.epicgames.com/documentation/en-us/unreal-engine/cooking-content-in-unreal-engine
- https://dev.epicgames.com/documentation/en-us/unreal-engine/project-section-of-the-unreal-engine-project-settings

## DEV_1 项目配置

取证文件：`F:\ShanghaiP4\neon\UGA\DEV_1\Config\DefaultGame.ini`。

`ProjectPackagingSettings` 的有效项目值包括：`Build=IfProjectHasCode`、通用 `BuildConfiguration=PPBC_Development`、Windows 覆盖 `PPBC_Shipping`、`FullRebuild=False`、`IncludeDebugFiles=True`、`UsePakFile=True`、`bUseIoStore=False`、`bUseZenStore=False`、`bCompressed=True`、`PackageCompressionFormat=Oodle`、`PackageCompressionMethod=Kraken`、`IncludePrerequisites=True`、`bCookAll=False`、`bCookMapsOnly=False`、`bSkipEditorContent=False`，以及多个 `DirectoriesToAlwaysCook`（包括 `/AesWorld`、`/EarthArtAsset`）。`DefaultEngine.ini` 设置了 Game Default Map。

`Config` 本身是指向 DEV 配置目录的 Junction，工具必须记录规范化后的实际配置来源，不能只看路径字符串。

## 这次 UDF 执行的实际状态

执行记录：`package-project-20260901T061637Z-175`。

- UDF 固定配置是 Shipping、Pak、compressed、prerequisites、禁用 `ModelContextProtocol` 和 `AllToolsets`。
- UDF 生成 `BuildCookRun`，加入 `-cook -build -stage -archive -package`，没有加入 `-iterate`、`-allmaps` 或 `-skipcookingeditorcontent`。
- UDF 每次把项目输入复制到新的系统临时目录，不复制 `Saved` 和 `Intermediate`。
- DEV_1 在执行前没有 `Binaries/Win64/UGA-Win64-Shipping.exe`、`Saved/Cooked/Windows`、`Saved/StagedBuilds/Windows` 或 Shipping Intermediate，因此本次 Build 和 Cook 都是冷启动。
- Cook 日志先出现约 3,480 个 Shader Job，随后处理约 13,923 个包；最终 UAT 与交付都返回 0。

## 对原生流程的影响

UE 仍然从暂存副本里的 `DefaultGame.ini` 读取未被命令行覆盖的设置，所以当前工具没有完全忽略项目配置。问题在于 UDF 没有建立配置解析和覆盖关系，默认参数集合把 `-pak`、`-compressed`、`-prereqs`、`-build` 固定写死，也没有把项目设置快照纳入执行记录。`StagingDirectory=D:/Packages` 被 UDF 的 profile output 有意覆盖，这是用户指定最终目录的正常覆盖。`IncludeDebugFiles=True` 没有被覆盖，所以最终包带有约 664 MB 的 Shipping PDB。

## 事实、推断和未知

事实：本次执行成功，UAT ExitCode 为 0，最终包约 10.15 GB；本次执行前没有项目级 Shipping Cook/Build 状态；命令行没有 `-iterate`。

推断：本次是该 DEV_1 状态下的首次 UDF Shipping 冷启动，不能据此断言项目从未由 Editor 或其他机器打过包。

未知：仅靠项目 `DefaultGame.ini` 不能完整重建 UE 所有 Engine Default、Platform Default 和 Editor Launcher 内部状态。工具必须把项目明确配置和 UDF 覆盖分开报告，并让 UAT 继续作为最终有效值解释器，不应伪造一份“全引擎默认值”。

## 设计输入

1. 保存配置时冻结项目打包设置摘要；项目设置变化时要求显式更新固定配置。
2. 默认 argv 只表达 UE 原生 Package Project 必需的操作和已确认的项目有效值，不添加未经项目或用户要求的 Cook 策略。
3. 默认使用完整 By-the-book Cook；增量 Cook 只能在高级配置明确启用且持久化状态摘要匹配时使用。
4. 计划和执行记录必须同时显示项目设置、UDF 覆盖、最终 argv、Cook 模式和复用判定。
