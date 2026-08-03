---
schema_version: 1
artifact: implementation
artifact_id: ar_01KZ40KNHCMHJFZHTY17CGTA2T
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
attempt_id: at_01KZ3FM1W8WB4J9MNKW8AQBXGK
created_at: 2026-08-03T14:31:55.692320Z
producer: aes-execute
result: complete
supersedes: ar_01KZ400CM1DGHBT8KMG7RH4M04
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ3MVK2Z8BJ2RWJBS7G1338P
      digest: sha256:eb439c9ad0e6fa2e9bd1ccc93d6b0593499c77753b8363579dfe23175a029291
      locator: plan.md
  subject:
    kind: change_set
    digest: sha256:abc7dc42bdb017d8c6ab99c0eb05d2100c3d87c71d16c26b3a441797065871d6
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: bad22632e66ec81138d8b082b4ee6d29c95fc9a7
    revision: 46fe42a0e91e8fcbbcf18c7d92106a8f70e814a7
    tree: d2ef02c83135ef13a6c070f76d4ffa9238825e11
    content_digest: sha256:abc7dc42bdb017d8c6ab99c0eb05d2100c3d87c71d16c26b3a441797065871d6
    branch_or_pr: refactor/tidy-cli-surface
    workflow_excluded: true
---

## 做完了什么

`unrealdevflow` 改名为 `udf`，20 个平铺命令收成 `workspace` / `task` / `build` / `skill` 四个组，
22 个叶子命令全部支持 `--format json` 并返回真实数据。版本跳到 `0.2.0`。

13 个提交，基线 `bad2263`，落在 `refactor/tidy-cli-surface` 上。

| 步骤 | 提交 | 改了什么 |
| --- | --- | --- |
| S1 | `3c6d8cc` | `[[bin]] name` 改成 `udf`，测试与源码文案跟上 |
| S2 | `8968f7f` | 发布链路指向 `udf.exe`，安装器删掉遗留的 `unrealdevflow.exe` |
| S3 | `70f1706` | 统一 JSON 信封 `{command, ok, data, error, messages}` |
| S4 | `9e6cd34` | `workspace` 组成型，删 `configure` |
| S5 | `2baf4d6` | `task` 组成型，删 `start`，位置参数统一 `TASK_REF` |
| S6 | `ab7de0c` | `build` 组成型，受控命令识别改成两词匹配 |
| S7 | `843aa0d` | `skills` → `skill`，三个叶子补结构化输出 |
| S8 | `ef75a07` | 八份文档 229 处示例改写，版本 `0.2.0`，写出改名对照表 |
| S9 | `c85747a` | 端到端冒烟；修断链 junction 的三处根因；6 处源码文案 |
| S10 | `3257557` | 补齐最后两个叶子的结构化输出；三处收尾漏网 |
| S11 | `09ec1db` | 评审两条阻断的修复，顺带解掉一条建议 |
| S12 | `e0b715d` | 验收发现的文档遗漏：发布资产清单里的二进制名 |
| S12 | `46fe42a` | 验收发现的功能缺口：build status 的任务引用改成可省略 |

## 评审提的问题怎么修的（S11）

**`build gate` 丢了零配置豁免。** S6 把五个平铺的 build 命令收进 `Build` 组，
`Commands::BuildGate { .. }` 这个变体没了，挂在它上面的配置豁免也跟着消失，而整个
`Build` 组不在豁免名单里。它只做字符串判定、不读任何配置，但 hook 和 wrapper 正是在
还没跑过 `workspace init` 的机器上调它。豁免改成匹配到叶子
（`Commands::Build { action: BuildAction::Gate { .. } }`），组里其余四个仍然要配置。
实测：空配置目录下 `build gate` 返回 `blocked` 判定，`build check` 仍正确报未配置。

**`--dry-run` 和取消路径没有结构化输出。** 这些出口在构造结果对象之前就 `return Ok(())`，
靠 `flush_unemitted` 兜出一个只有 `messages` 的空信封。一个预览命令让机器读不到要预览
什么，等于没覆盖。改法是给每个出口都带一个结果对象出来：

- `delete` 加 `DeletePreview`，dry-run 把本来就算好的 `reports` 结构化输出；取消路径给
  `DeleteOutcome` 加 `cancelled` 位。
- `merge` 把 `merge_single_plugin` 的返回值从 `bool` 换成 `PluginMergeReport`，带上分支、
  仓库、worktree、待合并提交数、领先/落后数、未提交数。`MergeOutcome` 加 `dryRun` 和
  `plugins`。这样 `--all --dry-run` 的逆序预览机器也读得到。
- `cleanup` 加 `untouched()` 构造器，覆盖「无匹配分支」和两处用户取消；孤儿分支清理的
  收尾原本也没 emit，一并补上。
- `create` 与 `switch` 的取消路径复用各自的 Outcome，把布尔位置成未执行。

**顺带解掉一条建议：`task finish` 报的命令名是 `task merge`。** 按 S4/S7 定下的模式拆出
`merge::run_inner`，由 `finish` 自己 `emit("task finish", ...)`。

实测复核：`build gate` 零配置可用；`task delete --dry-run --format json` 有 `data.plugins`；
`task merge --dry-run --format json` 带出 11 个待合并提交；四条命令各自只吐一个 JSON 文档。

评审里另外两条建议没做：`workspace status` 的 `junctionValid` 三态化、装出去的 SKILL.md
升级缺口。前者是精度问题不是缺陷，后者发布说明已经写了动作。

## 验收又逼出两处（S12）

验收不是走过场，AC-003 和 AC-005 第一轮都没过。

**AC-003：S8 的八份文档漏了第九份。** `skills/unrealdevflow-release/SKILL.md` 的必备资产清单
还写着 `unrealdevflow.exe`，而 `package-release.ps1` 产出的是 `udf.exe`——照着这份清单核对
发布资产会判成缺件。`.gitignore` 的注释里也留着 `unrealdevflow skills install`。zip 和安装器
名按强约束不动，只改二进制这一项。

**AC-005：可选性统一没做完。** 设计第 120-122 行写明查询类命令（`build check`、`build status`、
`task next`）省略任务引用时按最近任务解析。三个里只有 `build status` 还是必填——而「`build-check`
可选而 `build-status` 必填，两个都是查任务状态」正是设计开头点名要消掉的那条不一致。S6 收
build 组时照搬了旧签名，我当时没查设计。现在走跟另外两个查询命令同一条 `latest_task_ref` 路径。

## 跑了什么，结果如何

三条发布门禁在末版 `46fe42a` 通过：`cargo fmt --check`、
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、
`cargo test` 退出码都是 0，**71 个用例全过**（基线 65，新增 6）。

`udf --help` 顶层只列出 5 条：`workspace`、`task`、`build`、`skill`、`help`。

**端到端冒烟在真实项目上跑完了**，环境完整还原，收尾清单 12/12 通过。

## 端到端冒烟（S9）实际跑了什么

固定参数：workspace `cli-smoke`、主项目 `F:\ShanghaiP4\neon\UGA\DEV`、
hosts_root `F:\ShanghaiP4\neon\Hosts`、plugins_root `F:\ShanghaiP4\neon\Plugins`、UE_5.5。

| 组 | 主插件 | 结果 |
| --- | --- | --- |
| T1 | `ArtCommon` | 11 步全过，含一次真实 UBT 工程文件重生成（11.79 秒） |
| T2 | `AesWorld`（44 模块，10GB worktree） | create 12 秒；worktree 13→14→13；当前分支 `new_vege_editor_clean` 全程未动 |
| T3 | `WdpCamera` | **真实编译成功**：13 个编译单元、36 秒、`-Module=WdpCamera` 生效、产出 DLL、日志落盘 |
| T4 | `WdpCamera` + `WdpEnvironment` | 两个 Junction 一次切完；不给 `--plugin` 正确报错；`--all --dry-run` 逆序预览 |
| T5 | 复用 T4 | 跨项目 switch 被拒且未动任何 Junction；`build gate` 三个方向全对 |
| T6 | `ArtCommon` | 空主插件守卫三态全对：cleanup 拒、delete 拒、`delete --force` 放行并警告 |

**收尾还原 12/12**：三个 junction 指回源库、两个实体目录移回、`DEV/Plugins` 仍是 8 项、
`W-cli-smoke` 消失、四个源仓库 worktree 与分支回到基线、`AesWorld` 仍是 13 个 worktree、
`workspace remove cli-smoke` 执行、`config.toml` 与 `state.json` 与 T0 备份逐字节一致、
Hosts 下原有三项未变。

## 冒烟测出的真 bug

**断链 junction 会挡住 `switch`，而且自愈逻辑一直是死代码。**

`task cleanup` 之后 `DEV/Plugins` 下留着指向已删 Host 的断链 junction。下一次 `switch`
直接失败在「文件已存在」（os error 183）。根因是三层判定都会跟随重解析点：

- `Path::exists` 对断链 junction 返回 `false`，所以 `switch` 里 `if junction_path.exists()`
  判为假，`handle_existing_path` 那段「发现断链 junction，移除」根本到不了。
- `junction` crate 的 `exists` 同样跟随目标，导致 `is_broken` **永远不可能返回 true**。
- `delete` 因此对断链 junction 报 `NotExists`，删不掉。

也就是说，这套自称能自愈的断链处理从来没有生效过。现在 `junction::exists` 在 crate 说「不是」
时再问一次文件系统本身（`symlink_metadata` 的 `is_symlink`），`delete` 在 crate 失败时退回
`remove_dir`（Windows 上对重解析点执行 `remove_dir` 只删链接不碰目标），`switch` 与 preflight
改用不跟随链接的判定。三个回归测试，其中一个专门确认删 junction 不会删掉目标里的文件。

修完之后 T4 复跑，断链 junction 被自动接管，两个 Junction 都正确切换。

## 与计划不一致的地方

**计划漏了一份文档。** `.github/copilot-instructions.md` 有约 20 条命令示例，计划的 S8 只列了
5 份，实际是 8 份。

**S3 提前做了 S4/S5 的活。** `list` 和 `status` 原来自己打原始 JSON，跟新加的安全网叠加会输出
**两个 JSON 文档**（实测确认）。所以在 S3 就把这两个迁移了，否则信封不自洽。

**组合命令需要拆内部函数，计划没预见。** `workspace init` 内部调 `workspace add`、
`workspace doctor`、`skill install`，三个都 emit 的话一条命令会吐四个 JSON 文档。
解法是 `add_inner` / `doctor_inner` / `install_inner` 加一层薄封装：组合命令调内部函数，
只有最外层 emit。这个模式在 S4 定下来，S7 沿用。

**T1 和 T2 的主插件需要显式消歧，计划假设可以直接用。** `plugins_root` 下 `ArtCommon` 有 3 处
同名声明、`AesWorld` 有 2 处（`AesWorld_AI` 目录里也有 `AesWorld.uplugin`），`create` 按
Task#031 的规则拒绝。两组都要加 `--override-dep <name>=<绝对路径>`。

**计划里「merge 是 no-op，dev HEAD 不变」这条安全属性是错的。** `task merge` 会先把本地 `dev`
快进到 `origin/dev`。T1 跑完后 `ArtCommon` 的 `dev` 从 `a15b4bf` 移到了 `b7aad90`——查 reflog
确认是 `merge origin/dev: Fast-forward`，`b7aad90` 是远端早就有的提交，没有本地独有提交、
工作区干净。无害，但收尾判据必须改成「不含本地独有提交」而不是「HEAD 与 T0 一致」。

**T2 的「14 个引擎依赖被 enable」这条检查写错了。** 引擎依赖不会写进 Host 的 `.uproject`，
UE 会从 `AesWorld.uplugin` 自己解析。真正该验的那一半——不给引擎依赖建 junction——确认成立
（`Host/Plugins` 下只有 `AesWorld`）。

**T2 的 merge 没跑成。** `AesWorld` 主仓库有未跟踪的 `?? workflow/`，工具正确拒绝在不干净的
工作区上 rebase。这是保护不是缺陷；我没有去动用户的工作区。

## 明确没有做的

**T8 项目内依赖 junction 仍是空白。** 四个可用主插件都没有声明对本地项目插件的依赖——
`AesWorld` 的 14 个依赖全是引擎插件。要覆盖只能引入 `AesArtAsset`（不在 `DEV/Plugins` 里），
违反「只用本来就在用的插件」这条约束，所以没做。

**历史记录文档保持原样。** `docs/brainstorms/`、`docs/plans/`、`docs/insights/`、`docs/reports/`
和旧版 release notes 里的旧命令名没有改写——那些记录的是当时的事实。AC-003 只把 Release notes
列为例外，实际还应包含这些历史目录。

## 还剩什么风险

**升级路径只验了一半。** 安装器删旧 exe 的行为用 `-FromSource` 实测过，但完整的
「下载 Release zip → 解压 → 安装」链路没跑过，因为还没有 `0.2.0` 的 Release。

**`build task --background` 的 JSON 输出没实跑。** 前台路径在 T3 真实编译里验过，后台分支
只有代码走查。

**`workspace init` 的 skill 安装路径没实跑。** 冒烟全程带 `--skip-skill-install`，避免改用户
四个 provider 的全局目录。`skill install` 装到临时目录的路径也没跑（T7 被跳过了）。

**装在 provider 目录里的 SKILL.md 仍是旧版。** 本次没有执行 `udf skill install --global`，
所以你机器上四个 provider 目录里的 skill 还教着旧命令。发布说明里点明了这一条，但升级动作
要用户自己做。
