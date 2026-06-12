# 多 Workspace 并行与小白命令实施计划

## 目标

- 正确支持同一台机器上多个 UE 项目环境并行开发。
- A session 使用 A workspace，B session 使用 B workspace 时互不依赖当前全局配置。
- 任务创建后冻结自己的项目上下文，后续 build/switch/merge/cleanup/delete 不受其他 session 影响。
- 提供 `init/start/next/finish` 小白命令，隐藏复杂参数。
- 保持旧版 `configure/create/build/switch/...` 基本兼容。

## 数据模型

### Workspace

配置文件仍使用 `~/.unrealdevflow/config.toml`，新增：

```toml
last_used_workspace = "neon-dev"

[workspaces.neon-dev]
hosts_root = "F:\\ShanghaiP4\\neon\\Hosts"
plugins_root = "F:\\ShanghaiP4\\neon\\Plugins"
default_project = "F:\\ShanghaiP4\\neon\\UGA\\DEV"
engine_path = "D:\\Unreal Engine\\UE_5.5"
```

旧顶层配置继续兼容，并自动视为 `default` workspace。

### Task Context

`.udf-meta.json` 新增上下文快照：

```json
{
  "workspace": "neon-dev",
  "task_uid": "neon-dev/prefab-save-bug",
  "context": {
    "hosts_root": "...",
    "plugins_root": "...",
    "default_project": "...",
    "engine_path": "..."
  }
}
```

后续命令优先读 `context`，不再依赖当前顶层全局配置。

### Host 路径

新 workspace task：

```text
<hosts_root>/W-<workspace>/T-<task-id>_Host
```

旧 task 仍兼容：

```text
<hosts_root>/T-<task-id>_Host
```

## 命令设计

### 小白体验原则

第一屏只回答 3 个问题：

1. 这是哪个 UE 项目？
2. 这个 workspace 叫什么？
3. 现在下一步做什么？

AI/CLI 默认先给建议，不把配置表暴露给用户：

- workspace 名称从 `.uproject` 所在目录自动建议，例如 `DEV` -> `dev`，用户可用 `--workspace` 自定义。
- 用户输入的名称统一规范成 kebab-case，例如 `UGA DEV` -> `uga-dev`，后续命令固定使用这个安全名字。
- 只有一个 workspace 时，`start/create` 自动使用它；多个 workspace 时，必须显式 `--workspace`，避免并行 session 串项目。
- `next` 只给一条最应该执行的命令，不输出长说明书。
- `finish` 默认推荐 `rebase`，但仍必须由用户选择或通过 `--strategy` 明确传入。

### Workspace 专家命令

```powershell
unrealdevflow workspace add <name> --project <path> --plugins-root <path> --hosts-root <path> [--engine-path <path>] [--yes]
unrealdevflow workspace list
unrealdevflow workspace doctor <name>
unrealdevflow workspace remove <name>
```

### 小白命令

```powershell
unrealdevflow init [--workspace <name>] [--project <path>] [--yes]
unrealdevflow start "<需求>" [--workspace <name>] [--primary <Plugin>] [--yes]
unrealdevflow next [task-ref]
unrealdevflow finish [task-ref]
```

交互策略：

- `init` 自动建议 workspace 名称，`--yes` 时直接接受。
- `start` 自动建议 task id；若只有一个主插件候选或传入 `--primary`，直接创建。
- `next` 只输出当前最应该做的一步。
- `finish` 是 merge 向导，仍必须让用户选择策略；`--strategy` 可用于非交互。

用户可理解的最短路径：

```powershell
unrealdevflow init --project F:\ShanghaiP4\neon\UGA\DEV
unrealdevflow start "修复保存后组件丢失" --primary AesWorld
unrealdevflow next
```

后续 `next` 会根据状态提示 `build`、`switch`、验收或 `finish`，用户不需要记住完整流程。

## 并发与隔离规则

- `create/start` 用 workspace host root，不使用当前全局 project。
- `build` 用 task context 中的 engine path。
- `switch` 默认用 task context 中的 default project；允许 `--project` 临时覆盖。
- `state.json` 项目 key 改为 canonical path，不再使用目录名。
- 若 task id 在多个 workspace 中重复，短 id 命令必须报错，要求使用 `workspace/task`。

## 测试策略

- 单元/集成级：构造临时 workspace、插件 repo、uproject，验证：
  - 两个 workspace 可同名 task 并存。
  - `workspace/task` 能准确定位 host。
  - `list --workspace` 只列目标 workspace。
  - `next workspace/task` 不依赖当前全局配置。
- CLI smoke：使用 `UNREALDEVFLOW_CONFIG_DIR` 指向临时配置目录，不触碰真实 `~/.unrealdevflow`。
- 如需实机探测，只读扫描 `F:\ShanghaiP4\neon\UGA` 下项目目录作为 workspace 候选，不执行 switch，不写项目 Plugins。
- 结束后清理临时 config/state/hosts，确认真实项目未被改动。

## 完成标准

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --all-targets --all-features --locked`
- 自定义多 workspace smoke test 通过。
- 测试产生的 workspace 状态和 host 目录已清理。
- 按中文“修改内容 / 过程反思 / 后续注意”格式提交。
