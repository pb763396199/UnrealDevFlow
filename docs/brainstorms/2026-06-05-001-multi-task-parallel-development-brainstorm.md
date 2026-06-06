---
title: UnrealDevFlow 多任务并行开发工具设计
type: brainstorm
module: Core
date: 2026-06-05
feature: multi-task-parallel-development
status: active
supersedes: null
review:
  - date: 2026-06-05
    reviewer: architecture-strategist
    verdict: go-with-conditions
    critical_fixes: [junction-model, worktree-locks, multi-project, editor-detection, state-persistence, ubt-cache]
---

# UnrealDevFlow — AesWorld 多任务并行开发工具

> 解决"开发中临时产生新想法，需要并行验证但不干扰当前开发"的痛点

---

## 0. 专家审查修正（2026-06-05）

### 已修复的致命问题

| 问题 | 修复方案 |
|---|---|
| Junction 不是热切换 | 明确 `switch` 只影响下次 Editor 启动，增加 Editor 进程检测与警告 |
| Worktree 锁竞争 | 编译时使用 `-NoMutex`，worktree 操作时使用 `-WaitMutex`，增加锁超时重试 |
| 多项目支持缺失 | `switch` 支持 `--project <path>` 指定 UE 项目路径，支持多项目同步切换 |
| UBT 中间缓存失效 | `switch` 时自动清理 UE 项目的 `Intermediate/Build/Win64/UnrealEditor/Development/AesWorld/` |
| 状态持久化缺失 | 增加 `~/.unrealdevflow/state.json` 记录每个项目的活跃任务 |
| 构建结果无验证 | `build` 完成后验证 DLL 是否存在且时间戳新于源码 |

### Worktree 保留决策

保留 Git Worktree 作为核心机制，修复以下问题：
- **锁竞争**：worktree 创建/删除时使用 `git worktree lock/unlock` 防止并发冲突
- **路径长度**：使用 `\\?\` 前缀 + `dunce` crate 规范化路径，确保不超过 Windows 限制
- **跨盘符限制**：文档明确 worktree 必须与主仓库同盘符
- **主仓库依赖**：文档明确 worktree 依赖主仓库 `.git`，主仓库不可移动

---

## 1. 问题陈述

### 1.1 用户场景

开发者在 UE 项目中基于某个分支开发 AesWorld 插件功能时，经常产生新的想法/TODO。这些想法需要：
- 独立验证（需要编译 + 在 UE Editor 中运行）
- 不干扰当前正在开发的功能（未完成、未验收）
- 能快速切换回主开发环境

### 1.2 核心矛盾

| 维度 | 冲突 |
|---|---|
| 代码隔离 | 一个分支只能干一件事，WIP 无法安全分叉 |
| 编译隔离 | UBT 编译覆盖 Binaries，多任务不能共享同一套 DLL |
| 验证隔离 | UE Editor 一次只能加载一套插件 DLL |
| 切换成本 | stash → 切分支 → 编译 → 验证，周期太长 |

### 1.3 约束条件

- 插件主仓库位于 `F:\ShanghaiP4\neon\Plugins\AesWorld\`（.git 在这里，永远不动）
- 另一个独立 clone 位于 UE 项目目录中
- Worktree 从主仓库创建
- `create` 不改变当前开发环境，只有显式 `switch` 才切换
- SmartGit 21.2.4 需要能正常识别和操作
- 清理必须通过 CLI 一条命令完成
- 用户不关心状态文件，CLI 自动管理

---

## 2. 架构设计

### 2.1 目录结构

```
F:\ShanghaiP4\neon\
├── Plugins\
│   └── AesWorld\                    ← 插件主仓库（.git 在这里，永远不动）
│       ├── .git/
│       ├── Source/                  ← 用户日常开发位置
│       ├── Binaries/Win64/
│       └── AesWorld.uplugin
│
├── Hosts\                           ← 任务 worktree + Host 包装（平铺）
│   ├── T-mat-blend_Host\
│   │   ├── T-mat-blend_Host.uproject
│   │   ├── .udf-meta.json           ← 任务元信息
│   │   └── Plugins\
│   │       └── AesWorld\            ← Git worktree (branch: task-mat-blend)
│   │           ├── Source/
│   │           └── Binaries/Win64/  ← 编译产物
│   │
│   ├── T-time-transition_Host\
│   │   ├── T-time-transition_Host.uproject
│   │   ├── .udf-meta.json
│   │   └── Plugins\
│   │       └── AesWorld\            ← Git worktree (branch: task-time-transition)
│   │
│   └── T-ai-nav_Host\
│       └── ...
│
└── UGA\DEV_5_7\
    └── Plugins\
        └── AesWorld\ → Junction → Hosts\T-mat-blend_Host\Plugins\AesWorld
```

### 2.2 核心设计

| 组件 | 说明 |
|---|---|
| **主仓库** | `neon\Plugins\AesWorld\`，用户日常开发，永远不动 |
| **任务 Host** | `Hosts/T-<id>_Host\`，每个包含 .uproject + worktree + .udf-meta.json |
| **Junction** | UE 项目的 `Plugins\AesWorld` 指向当前要验收的任务 Host |
| **Worktree** | 从主仓库创建，所有 worktree 共享同一个 `.git` |
| **.udf-meta.json** | 每个 Host 自带的元信息文件，CLI 扫描识别任务 |

### 2.3 命名规范

| 层级 | 命名规则 | 示例 |
|---|---|---|
| 任务 Host | `T-<语义ID>_Host` | `T-mat-blend_Host`, `T-time-transition_Host` |
| Git 分支 | `task-<语义ID>` | `task-mat-blend`, `task-time-transition` |
| .uproject | `<任务ID>_Host.uproject` | `T-mat-blend_Host.uproject` |

### 2.4 .udf-meta.json 格式

```json
{
  "id": "mat-blend",
  "name": "动态材质混合",
  "branch": "task-mat-blend",
  "created": "2026-06-05T10:00:00Z",
  "based_on": "a1b2c3d",
  "status": "active"
}
```

---

## 3. 工作流

### 3.1 配置 `unrealdevflow configure`

**前置条件**：首次使用

**交互**：
1. 询问 Hosts 根目录（默认 `F:\ShanghaiP4\neon\Hosts`）
2. 询问插件主仓库路径（默认 `F:\ShanghaiP4\neon\Plugins\AesWorld`）
3. 询问默认 UE 项目路径（如 `F:\ShanghaiP4\neon\UGA\DEV_5_7`）
4. 询问引擎路径（默认从 .uproject 推断，或手动指定如 `D:\Unreal Engine\UE_5.7`）
5. 保存配置到 `~/.unrealdevflow/config.toml`

**配置后可通过 `--project` 参数覆盖默认项目**。

### 3.2 创建任务 `unrealdevflow create "动态材质混合"`

**执行步骤**：
1. 从主仓库获取当前分支和最后一个 commit
2. 基于描述生成任务 ID 建议（如 "动态材质混合" → "mat-blend"），用户确认或修改
3. 创建 worktree
4. 创建 `T-mat-blend_Host.uproject`
5. 创建 `.udf-meta.json`
6. 输出：任务已创建，可用 `unrealdevflow build mat-blend` 编译，`unrealdevflow switch mat-blend` 验收

**不改变主仓库**，用户继续在 `neon\Plugins\AesWorld` 开发。

**新任务基于最后一个 commit，不包含当前 WIP**。WIP 保留在主仓库中。

### 3.3 编译 `unrealdevflow build <id> [--background]`

**执行步骤**：
1. 定位到 `Hosts\T-<id>_Host\T-<id>_Host.uproject`
2. 推断引擎路径（从 .uproject 的 EngineAssociation）
3. 调用 UBT 编译
4. 编译产物写入 `T-<id>_Host\Plugins\AesWorld\Binaries\Win64\`

### 3.4 切换验收 `unrealdevflow switch <id> [--project <path>]`

**重要**：Junction 切换只影响**下次 Editor 启动**，不会影响运行中的 Editor。

**执行步骤**：
1. 检测目标 UE 项目路径（`--project` 参数或配置中的默认项目）
2. 检测该项目的 Editor 是否正在运行（检查 `UnrealEditor.exe` 进程）
   - 如果运行中：警告用户"Editor 正在运行，切换将在下次启动时生效"，询问是否关闭
3. 验证任务 Host 存在且 .udf-meta.json 有效
4. 清理 UE 项目的 UBT 中间缓存：`{ProjectDir}/Intermediate/Build/Win64/UnrealEditor/Development/AesWorld/`
5. 删除 UE 项目 `Plugins\AesWorld` 的 Junction（如存在）
6. 创建新 Junction → `Hosts\T-<id>_Host\Plugins\AesWorld`
7. 更新状态记录（哪个项目的 Junction 指向哪个任务）
8. 输出：提示重启 Editor

**示例**：
```
unrealdevflow switch mat-blend                          # 切换默认项目
unrealdevflow switch mat-blend --project DEV_5_7        # 切换指定项目
unrealdevflow switch mat-blend --project DEV_2,DEV_5_7  # 多项目同步切换
```

### 3.5 合并 `unrealdevflow merge <id>`

**执行步骤**：
1. 在主仓库执行 `git merge task-<id>`
2. 冲突时提示解决
3. 合并成功后：
   - 删除 worktree
   - 删除 Host 目录
   - 删除 Git 分支
4. 如果当前 Junction 指向被删除的任务，自动切换回主仓库路径

### 3.6 删除 `unrealdevflow delete <id>`

**执行步骤**：
1. 删除 worktree
2. 删除 Host 目录
3. 删除 Git 分支（`-D` 强制删除）
4. 如当前 Junction 指向被删除的任务，自动切换回主仓库

### 3.7 列出任务 `unrealdevflow list`

**执行步骤**：
1. 扫描 `Hosts/` 目录下所有 `T-*_Host` 目录
2. 读取每个目录的 `.udf-meta.json`
3. 输出任务列表

---

## 4. 任务元信息与状态管理

### 4.1 .udf-meta.json（每个 Host 自带）

每个任务 Host 目录下自带一个 `.udf-meta.json` 文件，记录任务元信息。

### 4.2 全局状态 `~/.unrealdevflow/state.json`

记录每个 UE 项目的活跃任务和 Junction 状态：

```json
{
  "version": 1,
  "projects": {
    "DEV_5_7": {
      "path": "F:\\ShanghaiP4\\neon\\UGA\\DEV_5_7",
      "active_task": "mat-blend",
      "junction_path": "F:\\ShanghaiP4\\neon\\UGA\\DEV_5_7\\Plugins\\AesWorld",
      "junction_target": "F:\\ShanghaiP4\\neon\\Hosts\\T-mat-blend_Host\\Plugins\\AesWorld",
      "last_switch": "2026-06-05T10:00:00Z",
      "previous_task": "main"
    },
    "DEV_2": {
      "path": "F:\\ShanghaiP4\\neon\\UGA\\DEV_2",
      "active_task": "time-transition",
      "junction_path": "F:\\ShanghaiP4\\neon\\UGA\\DEV_2\\Plugins\\AesWorld",
      "junction_target": "F:\\ShanghaiP4\\neon\\Hosts\\T-time-transition_Host\\Plugins\\AesWorld",
      "last_switch": "2026-06-05T11:00:00Z",
      "previous_task": "mat-blend"
    }
  }
}
```

### 4.3 状态流转

```
created → active → merged/deleted
```

### 4.4 任务发现

CLI 通过扫描 `Hosts/` 目录下所有 `T-*_Host` 目录，读取 `.udf-meta.json` 来发现任务。无需全局任务列表文件。

### 4.5 回滚支持

`state.json` 中的 `previous_task` 字段支持快速回滚：
```
unrealdevflow switch --previous    # 切回上一个任务
```

---

## 5. CLI 命令参考

| 命令 | 参数 | 作用 |
|---|---|---|
| `unrealdevflow configure` | - | 首次配置（Hosts 路径、插件路径） |
| `unrealdevflow create` | `<描述>` | 创建新任务（worktree + Host） |
| `unrealdevflow list` | - | 列出所有任务 |
| `unrealdevflow switch` | `<id>` | 切换 Junction 到指定任务 |
| `unrealdevflow build` | `<id> [--background]` | 编译指定任务 |
| `unrealdevflow merge` | `<id>` | 合并到主仓库并清理 |
| `unrealdevflow delete` | `<id>` | 删除任务（不合并） |
| `unrealdevflow status` | - | 查看当前 Junction 指向 |

---

## 6. 关键设计决策

### 6.1 为什么保留 Git Worktree？

- 所有 worktree 共享同一个 `.git`，SmartGit 能看到所有分支
- 分支管理统一，不需要多个独立仓库
- 磁盘占用比独立 clone 小得多
- **修复措施**：
  - 编译时使用 `-NoMutex` 避免 UBT 互斥锁冲突
  - worktree 创建/删除时使用 `git worktree lock/unlock` 防止并发冲突
  - 使用 `\\?\` 前缀 + `dunce` crate 处理路径长度限制
  - 文档明确 worktree 必须与主仓库同盘符

### 6.2 Junction 不是热切换

- Junction 在文件打开时解析，Editor 启动后 DLL 已加载到内存
- `switch` 只影响**下次 Editor 启动**
- CLI 必须检测 Editor 进程并警告用户
- 切换时清理 UBT 中间缓存确保下次编译正确

### 6.3 为什么用 Junction 而不是直接改目录？

- UE Editor 的项目配置不需要修改
- 切换是原子操作（删旧建新），不会破坏文件
- Junction 对 IDE 和 Git 工具完全透明

### 6.4 为什么新任务基于 commit 而不是 WIP？

- 保证任务独立性，不继承其他任务的未完成代码
- 避免合并时的复杂冲突
- WIP 保留在主仓库，不受影响

### 6.5 为什么用 .udf-meta.json + state.json 双重状态？

- `.udf-meta.json`：每个 Host 自带，扫描目录即可发现任务（无单点故障）
- `state.json`：全局状态，记录每个项目的活跃任务和 Junction 指向（支持回滚）
- 两者互补：删除 Host 即删除任务，但需要 state.json 追踪运行时状态

### 6.6 多项目支持

- 每个 UE 项目有独立的 Junction
- `switch` 支持 `--project <path>` 指定项目，支持多项目同步切换
- `state.json` 按项目名隔离状态

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|---|---|---|
| Junction 被误删 | Editor 无法加载插件 | CLI 提供 `status` 检查，切换前验证目标存在 |
| Worktree 锁竞争 | 编译失败 | 编译时使用 `-NoMutex`，worktree 操作时使用 `git worktree lock` |
| Git 仓库损坏 | 代码丢失 | worktree 操作有 git 保护，主仓库不动 |
| 多个 Editor 实例 | 内存占用高 | 串行验收，一次只开一个 Editor |
| 合并冲突 | 需要手动解决 | 串行合并，CLI 提示冲突文件 |
| UBT 中间缓存过期 | 编译跳过或崩溃 | `switch` 时自动清理 `Intermediate/Build/Win64/UnrealEditor/Development/AesWorld/` |
| Editor 运行中切换 | 用户看不到变化 | 检测 `UnrealEditor.exe` 进程，警告并询问是否关闭 |
| Worktree 跨盘符 | 创建失败 | 文档明确限制，CLI 检测并报错 |
| 主仓库移动/删除 | 所有 worktree 失效 | 文档明确主仓库不可移动，CLI 启动时验证 |

---

## 8. 后续扩展

- **Agent 集成**：CLI 输出结构化 JSON，Agent 可解析任务状态
- **后台预编译**：`unrealdevflow build mat-blend --background` 后台编译，切换时直接验收
- **多插件支持**：未来可扩展到支持多个插件的并行开发
- **IDE 插件**：未来可在 Rider/VSCode 中集成任务切换面板

---

## 9. 工具实现

### 9.1 命名

**工具名**: `UnrealDevFlow`
**可执行文件**: `unrealdevflow.exe`

### 9.2 Rust 项目结构

```
unrealdevflow/
├── Cargo.toml
├── src/
│   ├── main.rs                    # CLI 入口
│   ├── cli.rs                     # clap 命令定义
│   ├── config.rs                  # 配置管理
│   ├── error.rs                   # 统一错误处理
│   ├── output.rs                  # 结构化输出
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── configure.rs
│   │   ├── create.rs
│   │   ├── switch.rs
│   │   ├── build.rs
│   │   ├── merge.rs
│   │   ├── delete.rs
│   │   ├── list.rs
│   │   └── status.rs
│   ├── junction/
│   │   ├── mod.rs                 # Junction 操作封装
│   │   └── validator.rs
│   ├── git/
│   │   ├── mod.rs
│   │   └── worktree.rs
│   └── host/
│       ├── mod.rs
│       └── uproject.rs
```

### 9.3 核心依赖

```toml
clap = { version = "4.5", features = ["derive"] }
junction = "2.0"
git2 = "0.19"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
thiserror = "2.0"
tracing = "0.1"
dialoguer = "0.11"
indicatif = "0.17"
dunce = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

### 9.4 Junction 管理策略

使用 `junction` crate（v2.0）：
- 底层通过 `DeviceIoControl` + `FSCTL_*` 正确操作 NTFS 重解析点
- 自动处理权限提升
- 安全删除，不会误删目标内容

### 9.5 .uproject 生成

内联 JSON 模板：
```json
{"FileVersion":3,"EngineAssociation":"5.7","Modules":[],"Plugins":[{"Name":"AesWorld","Enabled":true}]}
```

### 9.6 主仓库位置

插件主仓库永远位于 `F:\ShanghaiP4\neon\Plugins\AesWorld\`，用户日常开发位置不变。
只有验收时才创建 Junction 指向任务 worktree。
