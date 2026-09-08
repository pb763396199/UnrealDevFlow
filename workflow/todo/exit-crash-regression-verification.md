---
schema_version: 1
protocol: 1.3.0
record: todo
id: td_01M1QK547DGFJPA89ZBWXTAP8P
status: done
created_at: 2026-09-05
closed_at: 2026-09-08
outcome: "已由 ue-test-run-cli Work Item 吸收并交付"
importance: normal
source: "claude:dac1de11-e901-4324-86f6-856b03886093，工作区 F:\\ShanghaiP4\\neon\\Plugins\\AesWorld"
tags: [ue-test-run-cli, exit-verification, ci]
---

# UDF 需要一条"编辑器退出干不干净"的验证命令

## 背景

在 AesWorld 插件修一个编辑器退出阶段崩溃（`FEarthModelerModule::OnPreExit` 反注册 Details
定制后强制刷新面板，访问到已经清空的 Editor Subsystem 查找表，空指针调用触发
`EXCEPTION_ACCESS_VIOLATION`）的过程中，验证方式全程靠手拼命令行：

```powershell
UnrealEditor-Cmd.exe <uproject> -ExecCmds="<一批命令>, QUIT_EDITOR" -unattended -nopause -nosplash -nullrhi -log
```

跑完之后人工判断进程退出码是不是 0，再手动 grep 日志里有没有 `Critical error`/
`EXCEPTION_ACCESS_VIOLATION`，前后各跑一次做对照。完整过程和两次真机日志的原始证据见
AesWorld 仓库任务记录 `workflow/earthmodeler-exit-crash/debug.md`
（`http://10.100.10.55/neon/AesWorld.git`，分支 `fix/earthmodeler-exit-crash`）。

独立评审当时指出：这类"引擎退出阶段"的崩溃没法写 UE C++ Automation 测试覆盖（自动化框架
自己也在这个时间窗口被卸载），但命令行级别的黑盒验证（启动进程、等它退出、查退出码、grep
日志关键字）完全可以脚本化，进 CI 常跑，不需要每次真人上手敲命令行、自己判断退出码和日志。

## 这件事和 `ue-test-run-cli` 的关系

这条待办已并入 [Work Item d9j60tjs：让 UDF 正确选择并复用 UE 原生运行与测试工具](../ue-test-run-cli/work-item.md)。
该 Work Item 已完成，退出验证也纳入了统一的 `udf run` 设计、实现和验收，不再单独实现一套命令。

原需求关注两件事：一是正确选择工程、关卡和 UE 原生入口，二是进程退出后分别判断业务结果、UE
原始退出码和崩溃证据。相关设计将 clean-exit 作为可组合的退出断言，并规定自然终止、未被强杀、
原始退出码为 0 且没有原生致命失败时才算通过。

## 建议覆盖的能力

- 给定一个 workspace/task Host 或主项目 + 一批 `-ExecCmds` + 退出命令，跑起来、等它退出，
  返回结构化结果：进程退出码、有没有 Critical error/崩溃调用栈、日志文件路径。
- 支持"改动前后各跑一次，对比结果"这种回归判据（这次修复用的验证方式），不只是单次跑。
- 输出要能直接喂给 CI，不需要人读日志。

## 已完成情况

- [设计记录](../ue-test-run-cli/design/run-verification-design-v2.md) 将退出验证定义为各类运行场景可组合的能力，并规定缺少退出事实时返回 `unknown`。
- [实现记录](../ue-test-run-cli/implementation.md) 记录了 `Udf.EditorExit` 严格退出规则和原生结果保存。
- [验收记录](../ue-test-run-cli/validation.md) 中 AC-008、AC-009、AC-018 均为 `passed`，覆盖退出规则、原始退出码和严格退出报告。
- 同一记录中的 AC-022、AC-024 均为 `passed`，包含真实 Host 和独立会话的退出验证证据。
- [交付记录](../ue-test-run-cli/delivery.md) 标记 `outcome: delivered`，落地分支为 `dev`，提交为 `6097f4187a766476eae1231eaa8eab0e6499c4e1`。

## 不建议做的事

- 不建议这条待办自己去实现。`ue-test-run-cli` 已经吸收这类命令结构、退出判定和结果输出，重复设计
  会打架。这条待办保留为真实案例的来源记录。
