---
title: UBT 严格编译参数源码考据
type: insight
status: active
date: 2026-06-09
---

# UBT 严格编译参数源码考据

> 沉淀于 Task#006 修复严格编译参数过程中，避免后续再次踩坑。
> 来源：`D:\Unreal Engine\UE_5.5\Engine\Source\Programs\UnrealBuildTool\Configuration\*.cs`

---

## 1. 真实存在的参数清单

| 参数 | 默认 | 源码位置 | 作用 |
|---|---|---|---|
| `-FailIfGeneratedCodeChanges` | false | `BuildConfiguration.cs:148` | UHT 生成的 `.generated.h` 过期则 fail（不让陈旧 generated 蒙混过关） |
| `-ForceHeaderGeneration` | false | `BuildConfiguration.cs:134` | 强制每次重新跑 UHT |
| `-NoUBTMakefiles` | makefile 启用 | `BuildConfiguration.cs:113` | 禁用 UBT 依赖图缓存，强制完整重扫描 |
| `-DisableUnity` | unity 启用 | `TargetDescriptor.cs:139` | 禁用 Unity Build，每个 cpp 独立编译（**慢，但 100% 暴露依赖错误**） |
| `-IWYU` | false | `TargetDescriptor.cs:151` | Include What You Use 模式 |
| `-Rebuild` | false | `TargetDescriptor.cs:133` | 先 Clean 再 Build（**完整重建，最稳但最慢**） |
| `-DisableAdaptiveUnity` | adaptive 启用 | `TargetRules.cs:1701` | 禁用启发式排除工作集文件 |
| `-NoPCH` | PCH 启用 | `TargetRules.cs:2017` | 禁用 PCH（极慢） |
| `-NoSharedPCH` | 共享PCH启用 | `TargetRules.cs:2248` | 禁用共享 PCH |
| `-NoPCHChain` | chain 启用 | `TargetRules.cs:2039` | 禁用 clang PCH 链 |
| `-WarningsAsErrors` | false | `TargetRules.cs:1820` | 所有警告变错误 |
| `-ShadowVariableErrors` | warning | `TargetRules.cs:1812` | shadow 变量警告变错误 |
| `-Module=<Name>` | - | `TargetDescriptor.cs` | 限定只编指定模块（用于增量构建特定插件） |
| `-StaticAnalyzer` | - | `TargetRules.cs:2080` | 启用静态分析器 |

---

## 2. 易错点警示（**千万不要踩**）

| 错误用法 | 真实含义 | 后果 |
|---|---|---|
| `-Clean` | **等价 `-Mode=Clean`**，只清理产物就退出 | DLL 根本不会产生，后续 DLL 时间戳检查必失败 |
| `-StrictIncludes` | **不存在的参数** | UBT 直接报错 |
| `-Precompile` | 构建为 precompiled 静态库（CIS/Installed Build 用） | 不是编译严格度，不该用于普通插件构建 |
| `-AllModules` | 编译所有模块（CIS 全量构建用） | 编插件插件目标不需要，会拖慢 |
| `-NoUBTMakefiles` 与 `-Clean` 混用 | 期望"清缓存+重编" | `-Clean` 会让进程直接退出，`-NoUBTMakefiles` 失效 |

---

## 3. 严格度档位组合

### Light（日常开发默认，推荐）
```
-FailIfGeneratedCodeChanges
-NoUBTMakefiles
-DisableAdaptiveUnity
```
- 编译时间：+10~20%
- 覆盖：90% 的"改头文件 cpp 没重编"问题

### Medium（PR 前验证）
```
Light + -WarningsAsErrors -ShadowVariableErrors
```
- 编译时间：+10~25%
- 覆盖：90% + 所有警告

### Heavy（接近 Install Build）
```
-Rebuild -DisableUnity -NoSharedPCH
-FailIfGeneratedCodeChanges -ForceHeaderGeneration
```
- 编译时间：**5-10 倍**（10-30 分钟）
- 覆盖：100% 不会遗漏，但日常太慢

---

## 4. UnrealDevFlow 当前默认（Task#006）

```rust
"-FailIfGeneratedCodeChanges"
"-NoUBTMakefiles"
"-DisableAdaptiveUnity"
```

Light 档位。命中 90% 痛点，编译时间增量可接受。
如果用户日后需要 Heavy 验证，应该加 `unrealdevflow build --rebuild` 标志，
内部追加 `-Rebuild -DisableUnity -NoSharedPCH`。

---

## 5. 验证参数存在性的标准流程

任何"听说有这个参数"的传闻都必须按下面验证：

```bash
# 在 UBT 三个核心文件搜 [CommandLine] 标注
rg 'CommandLine\("-XXX' D:\Unreal\ Engine\UE_5.5\Engine\Source\Programs\UnrealBuildTool\Configuration\*.cs
```

存在 → 看类型（bool？enum？value=...）和默认值
不存在 → 直接抛弃，**禁止凭直觉添加**

三个必查文件：
1. `BuildConfiguration.cs` - 全局构建配置
2. `TargetDescriptor.cs` - 目标描述（含 unity、IWYU 等）
3. `TargetRules.cs` - target rules（含 PCH、warning level 等大头）
