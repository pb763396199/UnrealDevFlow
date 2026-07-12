# UnrealDevFlow 三主插件真实环境测试报告

日期：2026-07-11

## 测试范围

- 主项目：`F:\ShanghaiP4\neon\UGA\DEV_2`
- Hosts：`F:\ShanghaiP4\neon\Hosts`
- 主仓库：`AesWorld`、`WdpCamera`、`WdpAPI`
- 测试分支：三个独立仓库均使用 `feature/test`
- 基线：执行 `git fetch origin dev` 后，各仓库分别从 `origin/dev` 创建 worktree
- 禁止项：全程未 push

## 基线提交

| 主仓库 | `origin/dev` |
| --- | --- |
| AesWorld | `1cff75309abf39e361c865b74ded14f46540295f` |
| WdpCamera | `313d6d890b89bc7039888eb91f6d338323f54f1b` |
| WdpAPI | `18131c244f5a54be0e02f870de0497affc361027` |

## 发现并修复的问题

1. `switch` 原先逐插件修改，后续插件冲突会留下未记录的半切换状态。现改为跨项目、跨插件全量预检；任何普通非空目录冲突都在零修改状态下终止。
2. `delete` 原先删除 Junction 后不更新 `state.json`。现同步移除任务 Junction 记录并清空活动任务状态。
3. 多主插件缺少真实回归测试。新增三主插件 create、switch、merge、delete 集成测试。
4. CLI 原先不能为多个仓库统一指定 `feature/test`，也不能在不切换用户主 checkout 的情况下从明确的 `origin/dev` 创建。新增 `--branch` 与 `--base-ref`。
5. 显式路径覆盖原先只参与依赖解析，不能消除主插件重名歧义。现允许自定义路径覆盖精确指定主仓库。
6. 多仓 create 中途失败会遗留先前已创建的分支。现回滚全部已创建 worktree、分支和 Host。
7. `WdpAPI` 是包含多个 `.uplugin` 和 Git submodule 的插件集合仓库。现递归发现描述符、模块和依赖，并在 worktree 创建后初始化 submodule。
8. 依赖原先只解析一层，遗漏 `PixelStreaming51Cloud` 等传递依赖。现递归展开项目插件依赖闭包。
9. kebab-case task id 直接进入 `.uproject` 文件名会让 UBT 生成非法 C# Rules 类。现对工程名做标识符安全化。
10. `build workspace/task-id` 的文件名计算原先错误使用完整 task ref。现使用元数据中的真实 task id。

## 自动化验证

- `cargo test`：40 个测试通过（9 unit + 15 create/source + 12 merge/delete + 4 multi-plugin）。
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`：通过。
- `cargo build --release`：通过。
- `git diff --check`：通过。

新增四个多主插件集成场景：

- 三个主仓创建三个 worktree，并统一使用显式分支。
- 主项目存在普通非空插件目录时，switch 必须零修改失败。
- 无冲突时一次创建三个 Junction，delete 一次清除三个 worktree、三个分支、Host、Junction 和状态记录。
- 多主 merge 未指定 `--plugin/--all` 时拒绝；`--all --dry-run` 按逆声明顺序覆盖三个主仓。

## DEV_2 真实测试结果

- 成功创建三个独立 worktree，提交与上表 `origin/dev` 一致。
- `merge` 未指定目标时正确拒绝；`--all --dry-run` 依次检查 `WdpAPI`、`WdpCamera`、`AesWorld`，未修改任何仓库。
- `DEV_2/Plugins/AesWorld` 与 `WdpCamera` 是普通目录，`WdpAPI` 是既有 Junction。switch 在预检第一个普通目录时零修改终止，并明确要求移动或备份冲突目录。
- 冲突测试后，AesWorld 与 WdpCamera 的目录类型、提交和未提交状态未变；WdpAPI Junction 仍指向原主仓库。
- UBT 首次暴露并验证修复了非法 Host 工程名、未初始化 submodule、遗漏传递依赖三类生成环境问题。
- 最后一次完整 UBT 重试受内网 DNS 间歇性故障阻断：`git.51vr.local` 在 submodule clone 过程中短暂无法解析。CLI 正确执行全量回滚，没有留下 Host、worktree 或 `feature/test` 分支。因此本次没有把“完整 UE 编译通过”列为已完成结论。

## 最终还原核验

- `F:\ShanghaiP4\neon\Hosts\W-udf-multi-dev2\T-multi-plugin-e2e_Host`：不存在。
- 三个主仓库中的 `feature/test`：均不存在。
- 三个主仓库的 worktree 列表：均无 `multi-plugin-e2e`。
- 临时 UnrealDevFlow 配置：已删除。
- DEV_2：AesWorld/WdpCamera 仍为原普通目录；WdpAPI 仍为原 Junction，目标仍是 `F:\ShanghaiP4\neon\Plugins\WdpAPI`。
- 用户原有状态未被清理：WdpAPI 的 `AesTilesEntity`、`GISCesium` 修改仍保留；未执行 push。

## 剩余风险

- 需要在内网 DNS 稳定时重跑一次真实 `build --primary-only`，才能对三主仓的完整 C++ 编译给出绿色结论。
- `delete` 在 Windows 上可能先收到 worktree 目录 Permission denied，但当前实现能识别 Git worktree 引用已移除，随后由 Host 清理删除残留；真实测试最终无残留。后续可继续优化日志，避免把可恢复路径显示得过于惊险。
