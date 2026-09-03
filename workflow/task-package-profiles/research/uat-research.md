---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1B7ZKEHST4M0DAW8DGX3P4H
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-08-31T06:28:00Z
producer: aes-research
result: complete
topic: 参数与引擎行为
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts: []
---

# BuildCookRun 参数调查

## 问题与停止条件

确认生成 Shipping、Pak、IoStore、loose 及 Cook 策略时哪些参数有一手依据。日期 2026-08-31，UE 5.5.4。

## 看过的地方

| 来源 | 事实 |
| --- | --- |
| src/ue_commands.rs:5、104、217 | 已有参数生成器，显式 package_args 会替代默认参数组 |
| src/commands/package.rs:666 | CLI 调用强制 Development、Win64、package_args=None |
| Engine/Source/Editor/TurnkeySupport/Private/TurnkeySupportModule.cpp:365、434 | Editor 根据 bSkipEditorContent、UsePakFile、bUseIoStore 拼参数 |
| Engine/Source/Programs/AutomationTool/AutomationUtils/ProjectParams.cs:711、778、798 | nocompileeditor 可影响 SkipBuildEditor；skippak 会将 Pak 设为 true |
| Engine/Source/Programs/AutomationTool/Scripts/CookCommand.Automation.cs:60 | SkipCookingEditorContent 转成 skipeditorcontent |
| Engine/Source/Editor/UnrealEd/Private/CookOnTheFlyServer.cpp:6458 | SkipEditorContent 拒绝保存 Engine/Editor 和 Engine/VREditor 资源 |

引擎路径为 C:/Program Files/Epic Games/UE_5.5。线上通用阶段说明：
https://dev.epicgames.com/documentation/unreal-engine/build-operations-cooking-packaging-deploying-and-running-projects-in-unreal-engine
版本差异以这里的 5.5.4 源码为准。

## 查到的

skippak 表示跳过构建并使用 Pak，不能用来表达 loose。
Editor 的排除 Editor 内容选项会真实改变 Cooker，不能无条件添加。
现有 nocompileeditor 已被 ProjectParams 作为 skipbuildeditor 别名识别，
不能把历史慢编译简单归因于“工具少了 skipbuildeditor”。

## 推断及设计输入

初始化时按 UE 配置层次解析并冻结 Packaging 设置，显示每项值的来源。
支持 Win64 的 Development、Shipping、Test、DebugGame 需按实际 Target/引擎能力预检，
不把底层 Configuration::Custom 当作任意配置都能成功的承诺。
其他平台必须通过对应适配验证，不能从枚举存在推断能在当前 Windows 引擎交叉构建。

容器三选一：pak 加 pak；iostore 按 UE 5.5 加 pak+iostore；loose 不生成两者，
副本配置同时关闭对应开关。compression、prereqs、debug files 独立保存。
map 使用显式列表或冻结的项目配置，不自动添加 allmaps。Cook full/iterative 要明确保存，
不得将清理 UBT Intermediate 等同清理 Cook/DDC，也不得自动容错忽略 Cook error。

UAT、UBT、Cooker 扩展参数按各自通道保存。大小写不敏感地检测与结构化配置冲突的同义参数，
禁止通过扩展参数覆盖 project、target、输出位置、插件排除及锁策略。
导入编辑器日志只生成候选配置，移除临时 EditorIOPort、自动更新 SDK、会话参数及敏感值。

## 未知与风险

UE 配置合并含平台层、数组增删及用户设置，Rust 侧不能只读取 DefaultGame.ini 就声称与 Editor 等价。
必须用 UE 5.5 fixtures 和真实项目对照验证。无法解析的设置报告 unresolved，不静默选默认。
