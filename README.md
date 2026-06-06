# UnrealDevFlow

Unreal Engine 插件多任务并行开发工作流工具。

## 概述

UnrealDevFlow 解决了一个核心痛点：**开发中临时产生新想法，需要并行验证但不干扰当前开发**。

### 核心能力

- **创建任务**：从主仓库创建 Git worktree + 独立 Host 编译环境
- **独立编译**：每个任务在独立的 Host 项目中编译，互不干扰
- **快速切换**：通过 NTFS Junction 切换 UE 项目加载的插件 DLL
- **合并清理**：验收通过后合并回主仓库，自动清理 worktree

## 安装

```powershell
cd F:\AiProject\UnrealDevFlow
cargo build --release
```

二进制位于 `target\release\unrealdevflow.exe`。

## 快速开始

### 1. 首次配置

```powershell
unrealdevflow configure
```

交互式引导配置：
- Hosts 根目录（默认 `F:\ShanghaiP4\neon\Hosts`）
- 插件主仓库路径（默认 `F:\ShanghaiP4\neon\Plugins\AesWorld`）
- 默认 UE 项目路径
- UE 引擎路径

### 2. 创建任务

```powershell
unrealdevflow create "动态材质混合"
```

自动建议任务 ID（如 `mat-blend`），确认后创建 worktree + Host。

### 3. 编译任务

```powershell
unrealdevflow build mat-blend
```

### 4. 切换验收

```powershell
unrealdevflow switch mat-blend
```

切换 Junction，重启 UE Editor 即可加载新任务的 DLL。

### 5. 合并任务

```powershell
unrealdevflow merge mat-blend
```

合并到主仓库，自动清理 worktree 和 Host。

## 命令参考

| 命令 | 说明 |
|---|---|
| `configure` | 首次配置 |
| `create <描述>` | 创建新任务 |
| `switch <id>` | 切换 Junction 到指定任务 |
| `build <id>` | 编译指定任务 |
| `list` | 列出所有任务 |
| `status` | 查看当前 Junction 状态 |
| `merge <id>` | 合并任务到主仓库 |
| `delete <id>` | 删除任务（不合并） |

### 全局选项

- `--format json|human`：输出格式（默认 human）
- `-v, --verbose`：详细日志

## 架构

```
F:\ShanghaiP4\neon\
├── Plugins\AesWorld\              ← 插件主仓库（.git 在这里，永远不动）
├── Hosts\
│   ├── T-mat-blend_Host\          ← 任务 worktree + Host
│   │   ├── T-mat-blend_Host.uproject
│   │   ├── .udf-meta.json
│   │   └── Plugins\AesWorld\      ← Git worktree
│   └── T-time-transition_Host\
│       └── ...
└── UGA\DEV_5_7\
    └── Plugins\AesWorld\ → Junction → Hosts\T-mat-blend_Host\Plugins\AesWorld
```

## 配置

配置文件位于 `~/.unrealdevflow/config.toml`：

```toml
hosts_root = "F:\\ShanghaiP4\\neon\\Hosts"
plugin_path = "F:\\ShanghaiP4\\neon\\Plugins\\AesWorld"
default_project = "F:\\ShanghaiP4\\neon\\UGA\\DEV_5_7"
engine_path = "D:\\Unreal Engine\\UE_5.7"
```

## 设计文档

详细设计文档位于 `docs/` 目录：

- `docs/brainstorms/` — 设计头脑风暴
- `docs/plans/` — 实现计划

## 许可证

MIT
