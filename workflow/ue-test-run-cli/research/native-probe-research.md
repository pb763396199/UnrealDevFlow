---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1QXNMGGPM9QVQR8WM2RFFYC
work_item_id: wi_01M13Q10GG3EFA5ESAMMTXZWSA
created_at: 2026-09-05T04:37:30Z
producer: aes-research
result: complete
topic: "UE 5.5.4 原生 Gauntlet 启动探针"
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8b7494c6794d990e361b561d6a51aebdba648e15478dafeb1864d64cebf7f68c
  artifacts: []
---

# UE 5.5.4 原生启动探针

## 运行对象

2026-09-05 在新建的 `neon-dev/udf-run-acceptance` Host 上运行。UDF 创建结果给出的实际文件是：

```text
Host: F:\ShanghaiP4\neon\Hosts\W-neon-dev\T-udf-run-acceptance_Host
Project: F:\ShanghaiP4\neon\Hosts\W-neon-dev\T-udf-run-acceptance_Host\T_udf_run_acceptance_Host.uproject
Engine: C:\Program Files\Epic Games\UE_5.5
UDF: 0.5.0, git 2a516b50d
```

Host 由 `udf task create ... --workspace neon-dev --id udf-run-acceptance --type chore --primary AesWorld --yes --format json` 创建。
`udf build check` 返回 `ready`，随后 `udf build task --mutex wait --format json` 返回 `ok=true`、`exitCode=0`，UBT 日志为 `F:\ShanghaiP4\neon\Hosts\W-neon-dev\T-udf-run-acceptance_Host\Logs\UBT\Build_light_20260905_122641.log`。
日志的 496 个动作全部完成；AesWorld 聚合 DLL 缺失警告不影响模块 DLL 验证。

## 探针命令与结果

证据根为 `F:\AiProject\UnrealDevFlow\.aes-workflow\worktrees\ue-test-run-cli\target\run-evidence\p0-editor-boot-20260905`，每次调用使用独立 TempDir、LogDir 和 stdout 文件。

| 调用 | 结果 | 证据 |
| --- | --- | --- |
| PowerShell 裸传 `-test=UE.EditorBootTest` | failed，UAT 日志把测试名解析为 `UE`，退出码 1 | `runuat-editorboot.stdout.log`；没有启动 Editor |
| 参数变量传 `-test=EditorBootTest` | passed，UAT 退出码 0，用命名空间 `UE` 找到节点 | `runuat-editorboot-shortname.stdout.log` |
| 参数数组传 `-test=UE.EditorBootTest` | passed，UAT 退出码 0 | `runuat-editorboot-fullname-array.stdout.log` |

最后一次调用的参数数组为：

```text
RunUAT.bat RunUnreal
  -project=<HostProject> -build=editor -test=UE.EditorBootTest
  -platform=Win64 -configuration=Development -NullRHI -Unattended
  -NoP4 -NoSourceControl -TempDir=<独立目录> -LogDir=<独立目录>
```

UAT 运行约 83 秒。实际 Editor 命令行包含唯一 `-execcmds=QUIT_EDITOR`、`-gauntlet`、`-nullrhi`、`-unattended`、项目绝对路径和本次 `-abslog`。日志显示 `Engine is initialized`、`Total Editor Startup Time`、`Editor shut down`、`Object subsystem successfully closed` 和 `Exiting`。
UAT 报告为 `UE.EditorBootTest (Win64 Development Editor) result=Passed`，角色报告显示 `Exit was requested: UUnrealEdEngine::CloseEditor() (ExitOk, ExitCode=0)`，并保留了 `EditorOutput.log`。

## 解释边界

这次探针证明：UE 5.5.4 的已安装引擎可以加载并执行原生 `UE.EditorBootTest`，Host 插件模块可以正常装载，UAT 能把每次日志放入指定目录，Editor 能完成受控退出。

裸写参数的失败来自 PowerShell 到批处理入口的参数传递，不能把它概括为 UE 不支持带命名空间的测试名。UDF 运行层必须用 argv 数组传参；这也解释了设计中禁止让会话直接拼接 shell 字符串。

报告统计 `Log Errors: 74`、`Log Warnings: 67/68`，其中包含既有项目初始化与 Blueprint 诊断。`EditorBootTest` 通过不等于项目内容健康，也不等于业务命令通过。实际启动为 Untitled 编辑器世界，符合本探针“不要求地图”的约定；不能用它证明指定地图已加载。

UAT 角色报告的 `ExitCode=0` 是 Gauntlet 的角色结果，不能作为 strict 自然退出的唯一原始进程证据。P1 仍须验证 `AppInstance.ExitCode`、`WasKilled` 和 `HasExited` 的自定义节点读取及规则矩阵。

## 交给施工的决定

1. 原生入口固定为 `RunUAT.bat RunUnreal`，测试名可以保存完整 `UE.EditorBootTest`，但实现必须通过参数数组传给 UAT；短名只作为兼容探针结果，不作为绕过参数数组的理由。
2. 每次执行显式传自己的 TempDir、LogDir 和最终 `-abslog`，保留 UAT 报告和 Editor 日志；不读公共最新文件。
3. `EditorBootTest` 只覆盖启动和受控退出。指定地图、Automation 业务结果和 strict 原始码分别由 P4/P6 的配置和节点验收。
4. 自定义节点不能继承 `EditorBootTest` 后重复追加退出命令。它应在自己的配置中把同步命令排在唯一 `QUIT_EDITOR` 前面，并在节点报告中导出原始状态。

## 尚未通过探针证明

外部 `.Automation.csproj` 的引用闭包、`-ScriptDir` 编译记录、`Udf.EditorExit` 尚未创建；三种 shell、指定地图、原始退出码严格判定和 UDF 新 run 命令都尚未实现。
