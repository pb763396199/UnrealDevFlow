# Task#021 测试报告

## 结论

- 被测提交：`4078970 Task#021 支持多 workspace 并行与小白命令`
- 结论：通过
- 报告日期：2026-06-12
- 测试范围：格式检查、严格 Clippy、Rust 测试、隔离多 workspace CLI smoke
- 真实项目影响：未执行 `switch`，未写入真实 UE 项目目录；CLI smoke 使用临时 fake UE 项目和 `UNREALDEVFLOW_CONFIG_DIR` 隔离配置

## 验证命令

| 项目 | 命令 | 结果 |
|---|---|---|
| 格式检查 | `cargo fmt --all -- --check` | 通过，退出码 0 |
| 严格静态检查 | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | 通过，退出码 0 |
| Rust 测试 | `cargo test --workspace --all-targets --all-features --locked` | 通过，退出码 0 |
| 二进制构建 | `cargo build --workspace --all-targets --all-features --locked` | 通过，退出码 0 |
| 多 workspace smoke | 临时配置目录 + fake UE project/plugin repo + CLI 命令链 | 通过，退出码 0 |

## 命令输出摘要

### `cargo fmt`

```text
cargo fmt --all -- --check
exit code: 0
```

### `cargo clippy`

```text
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.30s
exit code: 0
```

### `cargo test`

```text
cargo test --workspace --all-targets --all-features --locked

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

Finished `test` profile [unoptimized + debuginfo] target(s) in 1.35s
Running unittests src\main.rs (target\debug\deps\unrealdevflow-91d673bced9ab612.exe)
exit code: 0
```

说明：当前仓库没有 Rust 单元测试用例，所以 `cargo test` 的有效意义是测试构建和测试二进制启动通过；多 workspace 行为用下面的 CLI smoke 覆盖。

## 多 Workspace Smoke

### 测试目的

验证同一台机器上两个 workspace 可以并行创建同名任务，且不会因为 Host 目录、主项目或全局配置混淆：

- workspace `wa` 创建 `wa/shared-task`
- workspace `wb` 创建 `wb/shared-task`
- `next wa/shared-task` 指向 ProjectA
- `next wb/shared-task` 指向 ProjectB
- `next shared-task` 必须失败并提示多个 workspace 歧义
- `list --workspace wa` 不能出现 `wb/shared-task`
- `list --workspace wb` 不能出现 `wa/shared-task`
- `.udf-meta.json` 中的 `context.default_project` 必须分别冻结 ProjectA / ProjectB

### 测试环境

```text
UNREALDEVFLOW_CONFIG_DIR = C:\Users\pb763\AppData\Local\Temp\udf-report-smoke-80b891cc22cc400691af981bc3b04f7c\config
Fake Engine            = <temp>\UE_5.5\Engine\Build\BatchFiles\Build.bat
Fake ProjectA          = <temp>\ProjectA\ProjectA.uproject
Fake ProjectB          = <temp>\ProjectB\ProjectB.uproject
Fake PluginA repo      = <temp>\PluginsA\PluginA
Fake PluginB repo      = <temp>\PluginsB\PluginB
```

### 执行的关键命令

```powershell
unrealdevflow workspace add wa --project <ProjectA> --hosts-root <HostsA> --plugins-root <PluginsA> --engine-path <UE_5.5> --yes
unrealdevflow workspace add wb --project <ProjectB> --hosts-root <HostsB> --plugins-root <PluginsB> --engine-path <UE_5.5> --yes
unrealdevflow start "same task in wa" --workspace wa --id shared-task --primary PluginA --yes
unrealdevflow start "same task in wb" --workspace wb --id shared-task --primary PluginB --yes
unrealdevflow next wa/shared-task
unrealdevflow next wb/shared-task
unrealdevflow next shared-task
unrealdevflow list --workspace wa
unrealdevflow list --workspace wb
```

### Smoke 输出证据

```text
SMOKE_OK
config_dir=C:\Users\pb763\AppData\Local\Temp\udf-report-smoke-80b891cc22cc400691af981bc3b04f7c\config
wa_task=wa/shared-task project=C:\Users\pb763\AppData\Local\Temp\udf-report-smoke-80b891cc22cc400691af981bc3b04f7c\ProjectA branch=task/wa/shared-task
wb_task=wb/shared-task project=C:\Users\pb763\AppData\Local\Temp\udf-report-smoke-80b891cc22cc400691af981bc3b04f7c\ProjectB branch=task/wb/shared-task
ambiguous_exit=1
SMOKE_CLEANED=C:\Users\pb763\AppData\Local\Temp\udf-report-smoke-80b891cc22cc400691af981bc3b04f7c exists_after_cleanup=False
```

### Smoke 判定

通过。

证据点：

- `wa/shared-task` 和 `wb/shared-task` 都创建成功。
- 两个任务 branch 分别为 `task/wa/shared-task` 与 `task/wb/shared-task`。
- 两个任务的 `context.default_project` 分别冻结为 ProjectA 与 ProjectB。
- 短引用 `shared-task` 返回 `ambiguous_exit=1`，符合“多个 workspace 不允许隐式猜测”的设计。
- 临时目录清理后 `exists_after_cleanup=False`。

## 未覆盖项

- 未在真实 UE 项目上执行 `switch`，因为该命令会修改项目 `Plugins` Junction，不适合在本次验证中触碰真实项目。
- 未执行真实 UE 编译，因为本次改造重点是 workspace 隔离和 CLI 流程；真实编译应在后续以具体 UE 项目任务进行。
- 当前仓库缺少 Rust 单元/集成测试，后续可以把本次 smoke 固化为自动化集成测试脚本。
