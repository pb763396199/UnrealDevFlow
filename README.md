# UnrealDevFlow

UE 插件多任务并行开发工具。基于 Git Worktree + NTFS Junction，实现多个开发任务隔离编译、快速切换。

## 核心特性

- **任务隔离**：每个任务独立 worktree + Host 项目，互不干扰
- **快速切换**：Junction 切换，秒级切换开发环境
- **安全合并**：4 种合并策略（rebase/merge/squash/ff-only），延迟清理机制
- **编译隔离**：独立日志、智能 Mutex、后台编译支持
- **状态追踪**：任务状态、编译状态、Junction 状态一目了然
- **v2 多插件**：单任务跨多插件协同开发（主插件可写 + 依赖插件只读 + 智能扫描依赖）

## 安装

普通用户只需要一条命令，不需要安装 Rust，不需要 clone 仓库：

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pb763396199/UnrealDevFlow/releases/latest/download/unrealdevflow-installer.ps1 | iex"
```

安装器会自动：

- 下载最新 GitHub Release 中的 Windows 预编译包
- 安装到 `%USERPROFILE%\.unrealdevflow\bin`
- 写入 User PATH，并刷新当前 PowerShell 会话 PATH
- 安装 Codex / Claude Code / opencode 可用的 AI skill
- 验证 `unrealdevflow --version`

开发者本地源码安装：

```powershell
pwsh scripts/install.ps1 -FromSource
```

## 快速开始（v2 多插件）

### 1. 配置（v2 plugins_root 取代 v1 plugin_path）

```powershell
unrealdevflow configure `
    --hosts-root      "F:\ShanghaiP4\neon\Hosts" `
    --plugins-root    "F:\ShanghaiP4\neon\Plugins" `
    --default-project "F:\ShanghaiP4\neon\UGA\DEV"

# 仍兼容 v1 旧字段
# unrealdevflow configure --plugin-path "F:\ShanghaiP4\neon\Plugins\AesWorld" ...
```

### 2. 创建任务（多主插件 + 智能依赖扫描）

```powershell
# 多主插件
unrealdevflow create "修复 EarthPrefabActor 保存后 Component 丢失" `
    --id prefab-save-bug `
    --primary AesWorld,AesWorld_AI `
    --prompt "原始需求描述" `
    --yes

# 引擎/项目同名依赖必须显式选边
unrealdevflow create "..." --id xxx `
    --primary AesWorld `
    --override-dep PCG=project `
    --override-dep GeometryProcessing=engine
```

工具会自动：解析主插件的 `.uplugin` → 提取依赖 → 在引擎/项目两边查找 → 引擎自带自动 enable，项目内创建 Junction。

### 3. 编译

```powershell
# 前台编译（编全 Host .uproject：主插件 + 项目依赖 + 引擎依赖）
unrealdevflow build prefab-save-bug

# 后台编译
unrealdevflow build prefab-save-bug --background

# 只编主插件模块（增量验证）
unrealdevflow build prefab-save-bug --primary-only

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
# 单个主插件
unrealdevflow merge prefab-save-bug --plugin AesWorld --strategy rebase

# 全部主插件按逆序逐个 merge
unrealdevflow merge prefab-save-bug --all --strategy rebase

# 检查 merge 结果
git log --oneline -5

# 确认无误后清理
unrealdevflow cleanup prefab-save-bug
```

## 命令参考

| 命令 | 说明 |
|---|---|
| `configure` | 首次配置（Hosts 路径、plugins_root、项目路径） |
| `create` | 创建任务（多主插件 + 智能依赖扫描） |
| `build` | 编译任务（全 Host .uproject，可 `--primary-only`） |
| `build-status` | 查看编译状态 |
| `switch` | 切换 Junction 到指定任务（多 Junction 遍历） |
| `list` | 列出所有任务 |
| `status` | 查看当前 Junction 状态 |
| `merge` | 合并任务到主仓库（`--plugin` / `--all` 选边） |
| `cleanup` | 清理 worktree 和分支 |
| `delete` | 删除任务（不合并） |

## 发布流程

发布流程已经标准化，详见 [docs/RELEASE.md](docs/RELEASE.md)。

发布前必须通过：

```powershell
pwsh scripts/release-preflight.ps1 -Strict
```

GitHub Release 由 `v*.*.*` tag 触发，并自动生成 draft release、Windows exe、zip、installer 和 checksum。

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

## v1 任务迁移

旧版单插件 `.udf-meta.json`（无 `schema_version` 字段）会被工具自动迁移为 v2 格式：
- 合成 `primary_plugins[0] = AesWorld`（v1 硬编码默认值）
- 原始文件备份为 `.udf-meta.json.v1.bak`
- 无需任何手动操作

## 工作流示例

```
用户说："调查 EarthPrefabActor 保存后 Component 丢失的问题，EarthPCG 也得跟着改"

1. 创建多插件任务
   unrealdevflow create "EarthPrefabActor 保存丢失 + EarthPCG 联动修复" `
       --id prefab-save-bug `
       --primary AesWorld `
       --override-dep EarthPCG=project `
       --prompt "..." `
       --yes

2. 工具智能扫描：
   [Engine] GeometryProcessing  ✓ 自动 enable
   [Engine] OpenCV              ✓ 自动 enable
   [Project] EarthPCG           ✓ 创建 Junction 链回主仓库
   [Project] EarthModeler       ✓ 创建 Junction 链回主仓库
   ...
   在两个 worktree 里修代码：
   F:\...\Hosts\T-..._Host\Plugins\AesWorld\Source\...
   (EarthPCG 通过 Junction 直接看到 AesWorld 改动)

3. 编译
   unrealdevflow build prefab-save-bug
   # UBT 看到 .uproject enable 了 AesWorld + EarthPCG
   # 两个插件的源码一起编译，跨插件 API 不一致立即暴露

4. 切换验收
   unrealdevflow switch prefab-save-bug
   # 重启 UE Editor

5. 验收通过后合并
   unrealdevflow merge prefab-save-bug --plugin AesWorld --strategy rebase
   git log --oneline -5
   # EarthPCG 的改动用户在主仓库自己 commit
   unrealdevflow cleanup prefab-save-bug
```

## 常见问题

### Q: 切换时提示"目录被占用"怎么办？

A: 关闭占用目录的程序（Rider、VSCode、文件资源管理器），然后重试：
```powershell
unrealdevflow switch <task-id> --force
```

### Q: 合并后想回退怎么办？

A: 使用 `git reflog` 找到合并前的 commit，然后 `git reset --hard <commit>`。

### Q: 引擎和项目都有同名依赖插件，怎么办？

A: 用 `--override-dep <name>=engine` 强制使用引擎版，或 `=<name>=project` 强制使用项目版：
```powershell
unrealdevflow create "..." --primary AesWorld --override-dep PCG=project
```

### Q: 依赖插件的源码被改动了，会被工具发现吗？

A: 不会立刻拦截，但 build / switch / merge 前会 `git status --porcelain` 检查，dirty 时打印警告。

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
项目插件根 (Plugins/)        ← v2 配置 plugins_root
├─ AesWorld/ (独立 git)
├─ AesWorld_AI/ (独立 git)
├─ EarthPCG/ (独立 git)
└─ ...

主仓库 AesWorld (Plugins/AesWorld)     ← 用户日常开发，永远不动
    ↓ git worktree (per primary plugin)
任务 Host (Hosts/T-xxx_Host)            ← 隔离工作空间，独立编译
    ├─ Plugins/AesWorld/         (worktree, 可写)
    ├─ Plugins/AesWorld_AI/      (worktree, 可写)
    └─ Plugins/EarthPCG/         (Junction → 主仓库, 只读)
        ↓ NTFS Junction
UE 项目 (DEV/Plugins/AesWorld + DEV/Plugins/AesWorld_AI + ...)  ← 验收时切换指向
```

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request！
