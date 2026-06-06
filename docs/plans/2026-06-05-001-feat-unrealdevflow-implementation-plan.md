---
title: UnrealDevFlow 实现计划
type: plan
module: Core
date: 2026-06-05
feature: multi-task-parallel-development
status: draft
origin:
  - "[[2026-06-05-001-multi-task-parallel-development-brainstorm]]"
review:
  - date: 2026-06-05
    reviewer: architecture-strategist
    verdict: go-with-conditions
---

# UnrealDevFlow 实现计划

> 基于设计头脑风暴的落地实施方案
> 专家审查已识别 4 个致命问题和 5 个严重问题，均已在本计划中修复。

---

## 0. 专家审查修复清单

| 问题 | 状态 | 修复位置 |
|---|---|---|
| C1: Junction 不是热切换 | ✅ 已修复 | switch 命令增加 Editor 检测 + 明确提示 |
| C2: Worktree 锁竞争 | ✅ 已修复 | 编译用 `-NoMutex`，worktree 操作用 `git worktree lock` |
| C3: UBT 中间缓存失效 | ✅ 已修复 | switch 时自动清理 Intermediate 缓存 |
| C4: 多项目支持缺失 | ✅ 已修复 | switch 支持 `--project` 参数，state.json 按项目隔离 |
| M1: Editor 进程检测 | ✅ 已修复 | switch 前检测 UnrealEditor.exe |
| M2: 状态持久化缺失 | ✅ 已修复 | `~/.unrealdevflow/state.json` |
| M3: 构建结果无验证 | ✅ 已修复 | build 完成后验证 DLL 存在性和时间戳 |
| M4: 无回滚机制 | ✅ 已修复 | `switch --previous` + state.json previous_task |
| M5: 任务 ID 冲突 | ✅ 已修复 | create 时检查现有 .udf-meta.json |

---

## 1. 设计决策汇总

| 决策点 | 选择 |
|---|---|
| 工具名 | `UnrealDevFlow` (`unrealdevflow.exe`) |
| 语言 | Rust 1.85+ |
| 配置管理 | 交互式引导（`configure` 时） |
| 引擎路径 | 自动推断（`.uproject` 的 `EngineAssociation`） |
| 输出格式 | 默认结构化（Agent 友好） |
| 状态存储 | 每个 Host 自带 `.udf-meta.json` |
| 任务 ID | 自动建议 + 用户确认（语义化） |
| Junction 管理 | `junction` crate v2.0 |
| .uproject 生成 | 内联 JSON 模板 |

---

## 2. 状态存储结构

### 2.1 任务元信息（每个 Host 自带）

```
F:\ShanghaiP4\neon\Hosts\
├── T-mat-blend_Host\
│   ├── T-mat-blend_Host.uproject
│   ├── .udf-meta.json
│   └── Plugins\
│       └── AesWorld\            ← Git worktree
│
├── T-time-transition_Host\
│   ├── T-time-transition_Host.uproject
│   ├── .udf-meta.json
│   └── Plugins\
│       └── AesWorld\            ← Git worktree
│
└── ...
```

### 2.2 全局状态 `~/.unrealdevflow/state.json`

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
    }
  }
}
```

### 2.3 全局配置 `~/.unrealdevflow/config.toml`

```toml
hosts_root = "F:\\ShanghaiP4\\neon\\Hosts"
plugin_path = "F:\\ShanghaiP4\\neon\\Plugins\\AesWorld"
default_project = "F:\\ShanghaiP4\\neon\\UGA\\DEV_5_7"
engine_path = "D:\\Unreal Engine\\UE_5.7"
```

---

## 3. CLI 命令设计

### 3.1 `unrealdevflow configure`

```
功能: 首次配置
交互:
  1. 询问 Hosts 根目录
  2. 询问插件主仓库路径
  3. 保存到 ~/.unrealdevflow/config.toml
```

### 3.2 `unrealdevflow create <描述>`

```
功能: 创建新任务
交互:
  1. 基于描述自动生成任务 ID 建议
  2. 用户确认或修改 ID
  3. 确认基于当前 commit 创建
执行:
  1. 获取主仓库当前 commit
  2. 创建 worktree
  3. 创建 Host 目录和 .uproject
  4. 创建 .udf-meta.json
```

### 3.3 `unrealdevflow switch <id> [--project <path>]`

```
功能: 切换 Junction 到指定任务（只影响下次 Editor 启动）
执行:
  1. 确定目标 UE 项目路径（--project 参数或 config.toml 默认值）
  2. 检测 UnrealEditor.exe 是否正在运行该项目
     - 如果运行：警告用户，询问是否关闭
  3. 验证任务存在
  4. 清理 UBT 中间缓存：{ProjectDir}/Intermediate/Build/Win64/UnrealEditor/Development/AesWorld/
  5. 删除旧 Junction
  6. 创建新 Junction
  7. 更新 state.json（记录 active_task、previous_task）
  8. 输出：提示重启 Editor
```

### 3.4 `unrealdevflow build <id> [--background] [--no-mutex]`

```
功能: 编译指定任务
执行:
  1. 定位 Host.uproject
  2. 推断引擎路径
  3. 调用 Build.bat（默认 -NoMutex，因 worktree 共享 .git）
  4. 编译完成后验证：
     - DLL 是否存在于 T-<id>_Host\Plugins\AesWorld\Binaries\Win64\
     - DLL 时间戳是否新于源码文件
  5. 更新 .udf-meta.json 的 last_built 字段
  6. 输出编译结果
```

### 3.5 `unrealdevflow list`

```
功能: 列出所有任务
执行:
  1. 扫描 Hosts/ 目录下所有 T-*_Host 目录
  2. 读取 .udf-meta.json
  3. 输出任务列表
```

### 3.6 `unrealdevflow status`

```
功能: 查看当前 Junction 状态
输出:
  插件: F:\ShanghaiP4\neon\Plugins\AesWorld
  当前 Junction: DEV_5_7\Plugins\AesWorld → Hosts\T-mat-blend_Host\Plugins\AesWorld
  验证: ✓ Junction 有效，目标存在
```

### 3.7 `unrealdevflow merge <id>`

```
功能: 合并任务到主仓库
执行:
  1. 在主仓库执行 git merge
  2. 处理冲突
  3. 删除 worktree + Host
  4. 如当前 Junction 指向被删除任务，自动切换回主仓库
```

### 3.8 `unrealdevflow delete <id>`

```
功能: 删除任务（不合并）
执行:
  同 merge，但不执行 git merge
```

### 3.9 `unrealdevflow switch --previous`

```
功能: 回滚到上一个任务
执行:
  1. 从 state.json 读取 previous_task
  2. 执行 switch 到 previous_task
```

---

## 4. Rust 项目结构

```
F:\AiProject\UnrealDevFlow\
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config.rs
│   ├── state.rs                   # 全局状态管理
│   ├── error.rs
│   ├── output.rs
│   ├── editor.rs                  # Editor 进程检测
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
│   │   ├── mod.rs
│   │   └── validator.rs
│   ├── git/
│   │   ├── mod.rs
│   │   └── worktree.rs            # worktree 管理（含 lock/unlock）
│   └── host/
│       ├── mod.rs
│       └── uproject.rs
└── tests/
    └── integration/
```

---

## 5. 核心依赖

```toml
[package]
name = "unrealdevflow"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
junction = "2.0"
git2 = "0.19"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
thiserror = "2.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dunce = "1.0"
tokio = { version = "1.0", features = ["full"] }
dialoguer = "0.11"
indicatif = "0.17"
chrono = { version = "0.4", features = ["serde"] }
dirs = "5.0"
sysinfo = "0.32"                  # Editor 进程检测
```

---

## 6. 实现阶段

### Phase 1: 骨架（1-2 天）
- [ ] Cargo.toml + 依赖
- [ ] CLI 框架（clap）
- [ ] 错误处理（thiserror）
- [ ] 结构化输出
- [ ] 配置管理（config.toml + dialoguer 引导）

### Phase 2: 核心模块（2-3 天）
- [ ] Junction 管理（junction crate 封装）
- [ ] Git worktree 操作
- [ ] Host 项目创建（.uproject 生成）
- [ ] .udf-meta.json 读写

### Phase 3: 命令实现（3-4 天）
- [ ] `configure` 命令
- [ ] `create` 命令
- [ ] `switch` 命令
- [ ] `list` + `status` 命令
- [ ] `build` 命令
- [ ] `merge` + `delete` 命令

### Phase 4: 测试与优化（2-3 天）
- [ ] 集成测试
- [ ] 边界情况处理
- [ ] 性能优化
- [ ] 文档

---

## 7. 关键实现细节

### 7.1 配置管理

```rust
fn load_config() -> Result<Config> {
    let config_path = dirs::home_dir().unwrap().join(".unrealdevflow/config.toml");
    if !config_path.exists() {
        return Err(Error::NotConfigured);
    }
    let content = fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
```

### 7.2 任务 ID 建议

```rust
fn suggest_task_id(description: &str) -> String {
    // 从描述中提取语义化 ID
    // "动态材质混合" → "mat-blend"
}
```

### 7.3 Junction 切换

```rust
fn switch_junction(config: &Config, task_id: &str) -> Result<()> {
    let junction_path = config.project_plugins_dir.join("AesWorld");
    let new_target = get_task_host_path(config, task_id)?;
    
    if !new_target.exists() {
        return Err(Error::TargetNotFound(new_target));
    }
    
    if junction::exists(&junction_path)? {
        junction::delete(&junction_path)?;
    }
    
    junction::create(&new_target, &junction_path)?;
    Ok(())
}
```

---

## 8. 风险与缓解

| 风险 | 缓解 |
|---|---|
| `junction` crate 兼容性 | 使用 v2.0（最新），有完整测试 |
| `git2` 需要 libgit2 编译 | 提供预编译二进制或静态链接 |
| 长路径问题 | 使用 `\\?\` 前缀 + `dunce` crate |
| 权限问题 | 自动尝试提升权限，失败时给出明确提示 |
| 并发编译 | UBT 的 `-WaitMutex`/`-NoMutex` 机制 |
