# UnrealDevFlow

UE 插件多任务并行开发工具。基于 Git Worktree + NTFS Junction，实现多个开发任务隔离编译、快速切换。

## 核心特性

- **任务隔离**：每个任务独立 worktree + Host 项目，互不干扰
- **快速切换**：Junction 切换，秒级切换开发环境
- **安全合并**：4 种合并策略（rebase/merge/squash/ff-only），延迟清理机制
- **编译隔离**：独立日志、智能 Mutex、后台编译支持
- **状态追踪**：任务状态、编译状态、Junction 状态一目了然

## 安装

### 从源码编译

```powershell
# 1. 安装 Rust（如果还没有）
winget install Rustlang.Rustup

# 2. 克隆仓库
git clone <repo-url>
cd UnrealDevFlow

# 3. 编译
cargo build --release

# 4. 添加到 PATH（可选）
$env:Path += ";F:\AiProject\UnrealDevFlow\target\release"
```

### 预编译二进制

从 [Releases](https://github.com/your-org/unrealdevflow/releases) 下载 `unrealdevflow.exe`，放到任意 PATH 目录。

## 快速开始

### 1. 配置

```powershell
unrealdevflow configure `
    --hosts-root "F:\ShanghaiP4\neon\Hosts" `
    --plugin-path "F:\ShanghaiP4\neon\Plugins\AesWorld" `
    --default-project "F:\ShanghaiP4\neon\UGA\DEV"
```

### 2. 创建任务

```powershell
unrealdevflow create "修复 EarthPrefabActor 保存后 Component 丢失" `
    --id prefab-save-bug `
    --prompt "用户原始需求描述" `
    --yes
```

### 3. 编译

```powershell
# 前台编译
unrealdevflow build prefab-save-bug

# 后台编译
unrealdevflow build prefab-save-bug --background

# 查看编译状态
unrealdevflow build-status prefab-save-bug
```

### 4. 切换验收

```powershell
unrealdevflow switch prefab-save-bug
# 重启 UE Editor
```

### 5. 合并

```powershell
# 合并（保留 worktree 和分支供检查）
unrealdevflow merge prefab-save-bug --strategy rebase

# 检查 merge 结果
git log --oneline -5

# 确认无误后清理
unrealdevflow cleanup prefab-save-bug
```

或者一步到位（跳过检查）：
```powershell
unrealdevflow merge prefab-save-bug --strategy rebase --cleanup
```

## 命令参考

| 命令 | 说明 |
|---|---|
| `configure` | 首次配置（Hosts 路径、插件路径、项目路径） |
| `create` | 创建任务（worktree + Host） |
| `build` | 编译任务 |
| `build-status` | 查看编译状态 |
| `switch` | 切换 Junction 到指定任务 |
| `list` | 列出所有任务 |
| `status` | 查看当前 Junction 状态 |
| `merge` | 合并任务到主仓库 |
| `cleanup` | 清理 worktree 和分支 |
| `delete` | 删除任务（不合并） |

### 全局参数

| 参数 | 说明 |
|---|---|
| `--format <json\|human>` | 输出格式 |
| `-v, --verbose` | 详细日志 |
| `-h, --help` | 帮助信息 |
| `-V, --version` | 版本信息 |

## 合并策略

| 策略 | 说明 | 适用场景 |
|---|---|---|
| `rebase` | 把任务提交 replay 到 dev 之后（线性历史） | **默认推荐**，保持历史整洁 |
| `merge` | 创建 merge commit，保留任务历史 | 需要保留任务来源 |
| `squash` | 压缩所有任务提交成一个 | 任务提交很零散 |
| `ff-only` | 只在能快进时合并 | 严格线性工作流 |

## 工作流示例

```
用户说："调查 EarthPrefabActor 保存后 Component 丢失的问题"

1. 创建任务
   unrealdevflow create "EarthPrefabActor 保存后 Component 丢失问题调查" \
       --id prefab-save-bug \
       --prompt "调查 EarthPrefabActor 在关卡中保存后重新打开时 Component 丢失的问题" \
       --yes

2. 在 worktree 中调查代码
   F:\ShanghaiP4\neon\Hosts\T-prefab-save-bug_Host\Plugins\AesWorld\Source\...

3. 编译
   unrealdevflow build prefab-save-bug

4. 切换验收
   unrealdevflow switch prefab-save-bug
   # 重启 UE Editor

5. 验收通过后合并
   unrealdevflow merge prefab-save-bug --strategy rebase
   git log --oneline -5  # 检查
   unrealdevflow cleanup prefab-save-bug  # 确认无误后清理
```

## 常见问题

### Q: 切换时提示"目录被占用"怎么办？

A: 关闭占用目录的程序（Rider、VSCode、文件资源管理器），然后重试：
```powershell
unrealdevflow switch <task-id> --force
```

### Q: 合并后想回退怎么办？

A: 使用 `git reflog` 找到合并前的 commit，然后 `git reset --hard <commit>`。

### Q: 如何查看任务状态？

A:
```powershell
unrealdevflow list          # 列出所有任务
unrealdevflow status        # 查看当前 Junction 状态
unrealdevflow build-status <id>  # 查看编译状态
```

### Q: 日志文件在哪里？

A: 每个任务的编译日志在：
```
<HostsRoot>\T-<task-id>_Host\Logs\Build_<timestamp>.log
```

## 架构说明

```
主仓库 (Plugins/AesWorld)     ← 用户日常开发，永远不动
    ↓ git worktree
任务 Host (Hosts/T-xxx_Host)  ← 隔离的工作空间，独立编译
    ↓ NTFS Junction
UE 项目 (DEV/Plugins/AesWorld) ← 验收时切换指向
```

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request！
