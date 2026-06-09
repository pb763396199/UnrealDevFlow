---
title: v2 多插件支持实现计划
type: plan
module: Core
date: 2026-06-09
feature: multi-plugin-support
status: done
origin:
  - "[[2026-06-09-001-multi-plugin-support-brainstorm]]"
---

# v2 多插件支持实现计划

> 主插件（可写 worktree+branch+merge）+ 依赖插件（只读 Junction 直链主仓库 + dirty 检查）
> 依赖通过解析 `.uplugin` 智能扫描发现，区分 引擎自带 / 项目内 / 缺失 三类。

---

## 1. 决策汇总

| 决策点 | 选择 | 影响 |
|---|---|---|
| 依赖分类 | B: 彻底区分（依赖只读） | 简化数据模型，去掉中间态 |
| 插件池发现 | uplugin 智能扫描 + 引擎/项目分类 + 用户覆写 | 核心创新，零手动配置 |
| 主插件上限 | 不限 | 用户自律 |
| merge 策略 | 逆序逐个 + 独立确认 | 最稳妥 |
| 旧任务迁移 | 自动迁移（备份 `.udf-meta.json.v1.bak`） | 用户零感知 |
| 依赖接入 | Junction 直链主仓库 | 零额外磁盘占用 |
| 依赖只读机制 | 只检查 `git status --porcelain`，dirty 则警告，不锁定文件系统 | 灵活而清晰 |

---

## 2. 数据模型 v2

### `Config` 升级

```toml
# ~/.unrealdevflow/config.toml
hosts_root      = "F:\\ShanghaiP4\\neon\\Hosts"
default_project = "F:\\ShanghaiP4\\neon\\UGA\\DEV"
engine_path     = "D:\\Unreal Engine\\UE_5.5"
plugins_root    = "F:\\ShanghaiP4\\neon\\Plugins"   # v2 新增：项目插件根

# v1 兼容字段（自动作为默认主插件候选）
# plugin_path = "F:\\ShanghaiP4\\neon\\Plugins\\AesWorld"

[plugin_overrides]
# 强制把引擎同名插件用项目 fork 覆盖：
# "GeometryProcessing" = "F:\\MyForks\\GeometryProcessing"
```

### `TaskMeta` v2

```json
{
  "schema_version": 2,
  "id": "prefab-save-bug",
  "name": "...",
  "branch": "task-prefab-save-bug",
  "based_on": "abc...",
  "primary_plugins": [
    {
      "name": "AesWorld",
      "source_repo": "F:\\ShanghaiP4\\neon\\Plugins\\AesWorld",
      "worktree": "Plugins/AesWorld",
      "branch": "task-prefab-save-bug",
      "based_on": "abc..."
    }
  ],
  "dependency_plugins": [
    {
      "name": "EarthPCG",
      "source": "project",
      "source_path": "F:\\ShanghaiP4\\neon\\Plugins\\EarthPCG",
      "junction": "Plugins/EarthPCG"
    },
    {
      "name": "GeometryProcessing",
      "source": "engine",
      "source_path": "D:\\Unreal Engine\\UE_5.5\\Engine\\Plugins\\..."
    }
  ]
}
```

向后兼容：v1 没有 `schema_version`，读到后 `migration::migrate_in_place` 自动合成
`primary_plugins[0] = AesWorld`，并把原文件备份为 `.udf-meta.json.v1.bak`。

---

## 3. 智能依赖发现算法

```
解析每个主插件的 .uplugin
   ↓
提取 Plugins 数组（仅 Enabled=true）
   ↓
对每个依赖名查找：
   1. engine_plugins  = scan(<engine>/Engine/Plugins)
   2. project_plugins = scan(<plugins_root>)
   ↓
分类输出：
   engine  ✓ 自动 enable，不创建 Junction
   project ✓ 创建 Junction 链到主仓库
   conflict ❌ 同时存在 → 报错，让用户用 --override-dep 解决
   missing  ⚠ 都没有 → 报错，让用户用 --override-dep <name>=<path> 提供
   ↓
override 规则：
   --override-dep PCG=engine   → 强制使用引擎版
   --override-dep PCG=project  → 强制使用项目版
   --override-dep PCG=F:\path  → 自定义路径
```

---

## 4. CLI 接口契约

```bash
# configure（v2）
unrealdevflow configure \
    --hosts-root      F:\...\Hosts \
    --plugins-root    F:\...\Plugins \
    --default-project F:\...\UGA\DEV

# 兼容旧 v1：
unrealdevflow configure --plugin-path F:\...\AesWorld ...

# create（多主插件 + override）
unrealdevflow create "描述" --id task-xxx \
    --primary AesWorld,AesWorld_AI \
    --override-dep PCG=project \
    --yes

# build（默认全编，可只编主插件模块）
unrealdevflow build task-xxx
unrealdevflow build task-xxx --primary-only

# switch（多 Junction 自动遍历）
unrealdevflow switch task-xxx

# merge（必须 per-plugin 或 --all）
unrealdevflow merge task-xxx --plugin AesWorld   --strategy rebase
unrealdevflow merge task-xxx --all               --strategy rebase

# cleanup / delete（自动清理所有 worktree + Junction）
unrealdevflow cleanup task-xxx
unrealdevflow delete  task-xxx
```

---

## 5. 实施 Phase（最终合并为 Task#007 单 commit）

### Phase 1 - 数据模型 + 兼容层
- `src/config.rs`：加 `plugins_root`、`plugin_overrides`、`legacy_primary_plugin()`、`effective_plugins_root()`
- `src/host/mod.rs`：`TaskMeta` 加 `schema_version`/`primary_plugins`/`dependency_plugins` + 新增 `create_host_with_plugins`
- `src/host/uproject.rs`：`generate(version, &[name])` 接收插件名列表
- 新增 `src/plugin/mod.rs` + `scanner.rs` + `uplugin.rs`
- 新增 `src/migration.rs`：v1→v2 自动迁移
- `src/state.rs`：`ProjectState.junctions: Vec<JunctionState>`

### Phase 2 - Create 命令
- `src/cli.rs`：加 `--primary`、`--override-dep`、`parse_dep_override`
- `src/commands/create.rs`：多 worktree 创建 + 智能扫描 + Junction 创建
- `src/commands/configure.rs`：支持 `--plugins-root` 参数

### Phase 3 - Build 命令
- `src/commands/build.rs`：DLL 检查改为遍历 primary_plugins，依赖 dirty 检查，`--primary-only` 用 `-Module=`

### Phase 4 - Switch 命令
- `src/commands/switch.rs`：遍历主+依赖创建 Junction，`GlobalState.ProjectState.junctions`

### Phase 5 - Merge / Cleanup / Delete
- `src/commands/merge.rs`：`--plugin` / `--all`，逆序逐个独立确认
- `src/commands/cleanup.rs`：遍历所有 worktree + Junction
- `src/commands/delete.rs`：同上 + 每个 primary 独立 dirty/unmerged 评估

### Phase 6 - 文档
- 本 plan + brainstorm + insight 三份知识文件
- `AGENTS.md`：多插件工作流章节
- `README.md`：v2 示例

---

## 6. 风险评估

| 风险 | 严重度 | 缓解措施 |
|---|---|---|
| 多 worktree 路径名冲突（AesWorld_AI 的 .uplugin 也叫 AesWorld.uplugin） | 中 | 用插件目录名做唯一标识，不用 uplugin 内 FriendlyName |
| 依赖 Junction 跟着主仓库 dev 变化导致编不过 | 高 | build/switch 前对依赖跑 `git status`，dirty 则警告 |
| 多 merge 部分成功导致状态不一致 | 中 | 每个 plugin 独立 git 仓库，独立确认，失败不回滚，--force 才继续 |
| 引擎插件路径在不同电脑不一致 | 低 | 从 `config.engine_path/Engine/Plugins` 派生 |
| 旧任务自动迁移失败 | 低 | 迁移前自动备份 `.udf-meta.json.v1.bak` |

---

## 7. 改造规模

- 新增文件：`src/plugin/mod.rs`、`src/plugin/scanner.rs`、`src/plugin/uplugin.rs`、`src/migration.rs`
- 修改文件：`Cargo.toml`（无需改）、`cli.rs`、`config.rs`、`main.rs`、`host/*`、`state.rs`、所有 `commands/*`
- 新增代码：~550 行
- 修改代码：~400 行
- 复杂度：中高

---

## 8. 验收清单

- [x] `unrealdevflow merge` 不带 `--strategy` 直接报错
- [x] `unrealdevflow create --help` 显示 `--primary` 和 `--override-dep`
- [x] `unrealdevflow build --help` 显示 `--primary-only`
- [x] `unrealdevflow merge --help` 显示 `--plugin` 和 `--all`
- [x] `cargo build --release` 零 error
- [x] v1 `.udf-meta.json` 自动迁移（schema_version=2 + 备份 .v1.bak）
