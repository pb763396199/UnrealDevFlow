---
title: 多插件主次结构与依赖联动设计
type: brainstorm
status: active
date: 2026-06-09
---

# 多插件主次结构与依赖联动设计

> 起因：用户反馈"开发的插件要分主次，配置主插件的同时，可选配置依赖的多个插件"。
> 真实痛点：改完主插件的头文件后，依赖的插件 cpp 编译不过，但当前工具没发现。

---

## 1. 用户痛点真实根因

```
Step 1: 在 AesWorld 改了 SomeClass::SomeMethod 签名
Step 2: EarthPCG 的 cpp 引用了这个 API
Step 3: 当前工具只编 AesWorld → 通过 ✅
Step 4: 部署到 DEV 项目 → 链接时崩 ❌
```

根因：UBT 编译 .uproject 时只会编 .uproject 列出的插件。当前 Host .uproject 写死了
`"Plugins":[{"Name":"AesWorld","Enabled":true}]`，无法把依赖插件带进同一次编译。

---

## 2. 实地证据

- `F:\ShanghaiP4\neon\Plugins\` 下 **56 个插件，45 个是独立 Git 仓库**（非 monorepo）
- `EarthPCG.uplugin` 明确声明 `"Plugins": [{"Name": "AesWorld"}]`，证明插件之间有 uplugin 级依赖
- 同时存在源码级依赖：`Build.cs` 的 `PublicDependencyModuleNames` 引用其他插件模块
- `AesWorld_AI` 的 uplugin 文件名也叫 `AesWorld.uplugin`（潜在命名冲突点，要按目录名而非 FriendlyName 区分）

---

## 3. 当前代码硬编码风险点（共 7 处）

| 文件 | 行号 | 硬编码内容 |
|---|---|---|
| `src/commands/create.rs` | 61 | `host_dir/Plugins/AesWorld` |
| `src/commands/switch.rs` | 44, 53 | Junction 路径 + plugin 名 |
| `src/commands/merge.rs` | 125 | worktree 路径 |
| `src/commands/cleanup.rs` | 50 | worktree 路径 |
| `src/commands/delete.rs` | 149 | worktree 路径 |
| `src/commands/build.rs` | 142 | DLL `UnrealEditor-AesWorld.dll` |
| `src/host/uproject.rs` | 5 | `"Plugins":[{"Name":"AesWorld","Enabled":true}]` |
| `src/config.rs` | 12 | `plugin_path: PathBuf` 单值 |

---

## 4. 专家组分析

### Git Worktree 视角
- Git worktree 按 repo 操作，每个 repo 一个独立 worktree
- 同一 Host 目录可放多个 worktree，分别指向不同 repo
- 路径结构应该改为：`Hosts/T-xxx_Host/Plugins/<plugin_name>/`

### UE 编译视角
- UBT 编译 .uproject 时按 `"Plugins":[]` 数组决定要编哪些
- 只要 Host .uproject 把主插件 + 依赖插件全部 `Enabled:true`，UBT 就会编全部
- 引擎自带插件不在 `Plugins/` 子目录而是在 `<Engine>/Engine/Plugins/`，自动可见

### Junction 视角
- 当前 switch 只切一个 Junction
- 多插件需要遍历，每个插件一个 Junction
- 依赖插件可以直接 Junction 链回主仓库（最简单），跟着 dev 走

### Merge 视角
- 多个独立 repo 就是多次 merge
- 主插件之间没有顺序耦合，但失败处理要清晰
- 依赖插件只读，不参与 merge

### UX 视角
- 命令复杂度爆炸：原来 1 task 1 plugin，现在 1 task N plugin
- 解决：依赖通过 `.uplugin` 智能扫描自动发现，用户只在冲突/缺失时介入

---

## 5. 决策汇总（用户裁决结果）

| 决策点 | 选择 |
|---|---|
| 依赖分类 | B: 彻底区分（依赖只读） |
| 插件池发现 | uplugin 智能扫描 + 引擎/项目分类 + 用户覆写权 |
| 主插件上限 | 不限 |
| merge 策略 | 逆序逐个 + 独立确认 |
| 旧任务迁移 | 自动迁移 |
| 交付节奏 | 渐进 6 Phase（最终合并为一个 commit） |
| 依赖接入 | Junction 直链主仓库 |
| 依赖只读 | 只检查不锁定 |

落地方案详见同日 plan：
[[2026-06-09-001-feat-multi-plugin-support-implementation-plan]]
