---
schema_version: 1
artifact: validation
artifact_id: ar_01KZ40N1BGZ6GJRGHMTZM585QH
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
attempt_id: at_01KZ3FM1W8WB4J9MNKW8AQBXGK
created_at: 2026-08-03T14:32:40.560499Z
producer: aes-validate
outcome: passed
acceptance:
  - acceptance_id: "AC-001"
    outcome: "passed"
    method: "读 Cargo.toml 的 [[bin]] 名；在临时目录里真跑一遍安装器（-FromSource -NoPath -NoSkill），事先塞一个遗留 unrealdevflow.exe 进去，看安装后目录里剩什么；再 grep 源码与 README 确认配置目录、安装目录、两个环境变量名和安装器资产名没被改"
    evidence: "Cargo.toml:11-13 [[bin]] name = \\\"udf\\\"；临时目录安装前含 unrealdevflow.exe，安装器打印 'removed the superseded unrealdevflow.exe'，安装后目录只剩 udf.exe 与 skills/，udf --version 输出 'udf 0.2.0'；install.ps1:17 默认 InstallPath 仍是 $env:USERPROFILE\\.unrealdevflow\\bin；src/config.rs:34 CONFIG_DIR_ENV=UNREALDEVFLOW_CONFIG_DIR、src/build_policy.rs:26 ENGINE_ROOT_ENV=UNREALDEVFLOW_UE_ENGINE_ROOT 均未改；README.md:21 的一行安装 URL 仍指向 unrealdevflow-installer.ps1；用户真实的 ~/.unrealdevflow/bin 全程未动"
  - acceptance_id: "AC-002"
    outcome: "passed"
    method: "跑 udf --help，取 Commands 段"
    evidence: "Commands 段恰好五行：workspace / task / build / skill / help，没有任何平铺的动词命令"
  - acceptance_id: "AC-003"
    outcome: "passed"
    method: "全仓 grep 六个旧命令名与把 unrealdevflow 当可执行文件用的写法，排除 .git/target/workflow/dist/docs 与 Release notes"
    evidence: "第一轮命中三处真问题：skills/unrealdevflow-release/SKILL.md:29 的必备资产清单写着 unrealdevflow.exe（package-release.ps1:68/81/86 产出的是 udf.exe）、.gitignore:6 注释里的 `unrealdevflow skills install`；两处在 e0b715d 修掉。修后复跑，剩余命中只有 scripts/install.ps1:197/200（故意保留，就是用来删遗留 exe 的）与 .omx/ 下的历史会话状态（非本仓库产物）。docs/releases/v0.2.0.md 的改名对照表是验收明写的唯一例外"
  - acceptance_id: "AC-004"
    outcome: "passed"
    method: "--format 在 cli.rs:16 是 global 参数，22 个叶子都收得到；再枚举源码里全部 output::emit 的命令名，跟 22 个叶子对表；最后抽 7 条命令实跑 --format json，数 stdout 上的 JSON 文档个数并确认 data 非空"
    evidence: "emit 的命令名并集恰好 22 个：workspace init/add/list/doctor/remove/status（6）、task create/list/next/switch/merge/finish/cleanup/delete（8）、build task/project/check/gate/status（5）、skill install/list/remove（3）；实跑 workspace list、workspace status、task list、task next、build check、build gate、skill list 七条，每条 stdout 恰好 1 个 JSON 文档且 data 非空、command 是叶子名；写操作返回做了什么：task delete --dry-run 返回 data.dryRun=true 与 data.plugins[1]、task merge --dry-run 返回每插件的分支/仓库/worktree/待合并 11 个提交/领先落后数"
  - acceptance_id: "AC-005"
    outcome: "passed"
    method: "对九条接受任务引用的命令跑 --help，读 Usage 行里的参数名与方括号/尖括号；再对着设计第 120-122 行核对可选性分类"
    evidence: "九条 Usage 全部写 TASK_REF，grep 全仓不再有 TASK_ID；可选性：task next [TASK_REF]、task finish [TASK_REF]、build check [TASK_REF] 可省略，task switch/merge/cleanup/delete 与 build task 必填。第一轮 build status 是 <TASK_REF> 必填，与设计要求的查询类可省略不符，46fe42a 改成 [TASK_REF] 并走 latest_task_ref；复跑 udf build status --format json 省略参数时解析到 neon-dev1/19-aesworld-roadmap"
  - acceptance_id: "AC-006"
    outcome: "passed"
    method: "脚本从五份文档抽出所有以 udf 开头的整行示例，逐条判定命令路径，再把每个 -- 参数拿去跟对应叶子的 --help 文本比对"
    evidence: "抽出 136 条完整示例，覆盖 18 个不同命令路径；命令路径全部解析成功，参数核对失败 0 条。人是否读得懂、照着做能不能达到目的，机器验不了，转 manual-test.md"
  - acceptance_id: "AC-007"
    outcome: "passed"
    method: "在末版 46fe42a 上依次跑三条门禁并取退出码，再统计用例总数"
    evidence: "cargo fmt --check 退出码 0；cargo clippy --workspace --all-targets --all-features --locked -- -D warnings 退出码 0；cargo test 退出码 0，四个测试二进制合计 71 passed / 0 failed（基线 65，新增 6）；rustc 1.96.1"
environment: "Windows 11 Pro 26200; rustc 1.96.1; PowerShell 5.1; workspace neon-dev1 at F:/ShanghaiP4/neon/UGA/DEV_1; 12 real tasks"
executed_at: "2026-08-03T14:30:10Z"
supersedes: ar_01KZ40GEQN38H8QAPPW0S3VPZS
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ40MSSATP1D6K1K3CH4WW1A
      digest: sha256:fb9c9a36d8dfa5a4605cabfffb96ba1a5eea3aebeff607d761b9cf6448114c4a
      locator: reviews/code-review.md
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

## 在什么状态上验的

分支 `refactor/tidy-cli-surface`，版本 `46fe42a`，基线 `bad2263`，13 个提交。
代码区干净，只有 `workflow/` 未提交。

环境：Windows 11 Pro 26200，rustc 1.96.1，workspace `neon-dev1`
（`F:\ShanghaiP4\neon\UGA\DEV_1`），真实任务 12 个。

变更集和实现记录对得上：`git log --oneline bad2263..HEAD` 列出 13 个提交，前 11 个与实现
记录的 S1–S11 表格逐条吻合，后两个是这次验收当场发现问题后补的（见下）。

## 验收过程中发现并修掉的三个问题

验收不是走过场，AC-003 和 AC-005 第一轮都没过。

**AC-003 漏了一份文件。** `skills/unrealdevflow-release/SKILL.md` 的必备资产清单还写着
`unrealdevflow.exe`，而 `package-release.ps1` 产出的是 `udf.exe`。照着这份清单核对发布资产
会判成缺件。`.gitignore` 的注释里也还留着 `unrealdevflow skills install`。两处都在 `e0b715d` 修了。
zip 和安装器名按强约束保持不变，只改二进制这一项。

**AC-005 没做完。** 设计写明查询类命令（`build check`、`build status`、`task next`）省略任务
引用时按最近任务解析，动作类必填。三个里只有 `build status` 还是必填——而「`build-check`
可选而 `build-status` 必填，两个都是查任务状态」正是设计第 27 行点名要消掉的那条不一致。
S6 收 build 组时照搬了旧签名。`46fe42a` 改成可省略，走跟另外两个查询命令同一条 `latest_task_ref`
路径，`AGENTS.md` 两处写法跟着改成方括号。

**AC-001 的安装行为此前只有代码走查。** 这次在临时目录里真跑了一遍安装器。

## 逐条结果

七条全 `passed`。AC-001 的完整 Release 下载链路和 AC-006 的人工可读性两项转人工核对，
见 `manual-test.md`。
