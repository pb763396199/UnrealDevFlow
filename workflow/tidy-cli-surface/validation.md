---
schema_version: 1
protocol: 1.3.0
artifact: validation
artifact_id: ar_01KZ5DE0ZABZ72Y1SY2ZWA9BV0
work_item_id: wi_01KZ3FKQQ6FB98FMPVY6MD3XE9
created_at: 2026-08-04T03:35:16.714909Z
producer: aes-validate
outcome: passed
acceptance:
  - acceptance_id: "AC-001"
    outcome: "passed"
    method: "读 Cargo.toml 的 [[bin]] 名；按用户机器的真实目录形态在临时目录里造场景（unrealdevflow.exe + unrealdevflow.cmd + unrealdevflow.installed-20260630.exe + unrealdevflow-installer.ps1），跑一遍安装器看剩下什么；再造一个 cargo 必然失败的场景，确认安装器不会把旧二进制装出去；最后 grep 源码与 README 确认配置目录、安装目录、两个环境变量名和安装器资产名没被改"
    evidence: "Cargo.toml:11-13 的 [[bin]] name 是 udf；安装器打印三行 removed the superseded（.cmd / .exe / .installed-20260630.exe 各一行），装完目录只剩 udf.exe、skills/ 和 unrealdevflow-installer.ps1——发布资产因为不匹配 ^unrealdevflow(\.installed-.*)?$ 而保住；把 RUSTUP_TOOLCHAIN 指向不存在的工具链时，安装器抛 cargo build --release --locked failed with exit code 1 并中断，目标目录为空（第一轮这里会静默装出上次留下的旧二进制）；install.ps1:17 的默认 InstallPath 仍是 %USERPROFILE%\.unrealdevflow\bin；src/config.rs:34 的 CONFIG_DIR_ENV 与 src/build_policy.rs:26 的 ENGINE_ROOT_ENV 都没改；README.md:21 的一行安装 URL 仍指向 unrealdevflow-installer.ps1；用户真实的 ~/.unrealdevflow/bin 全程没动。第一轮这条判 passed 是错的——当时只塞了一个 unrealdevflow.exe，正好是安装器硬编码要删的那个文件名，等于拿被测代码的假设去造数据"
  - acceptance_id: "AC-002"
    outcome: "passed"
    method: "跑 udf --help，取命令段"
    evidence: "命令段恰好五行：workspace / task / build / skill / help，没有任何平铺的动词命令。帮助中文化之后段落标题从 Commands: 变成 命令:，条目不变"
  - acceptance_id: "AC-003"
    outcome: "passed"
    method: "全仓 grep 六个旧命令名、把 unrealdevflow 当可执行文件用的写法，以及第二轮补上的『调用自身可执行文件时用了旧子命令名』（udf skills ），排除 .git/target/workflow/dist/docs/.omx 与 Release notes"
    evidence: "三轮共命中三处真问题并修掉：skills/unrealdevflow-release/SKILL.md:29 的必备资产清单写着 unrealdevflow.exe 而打包脚本产出的是 udf.exe；.gitignore:6 的注释；scripts/install.ps1:223 敲的 udf skills install——S7 单数化时漏的，实测报 unrecognized subcommand 而脚本仍打印成功。末版复跑，剩余命中只有 scripts/install.ps1 里故意保留的遗留清理逻辑。docs/releases/v0.2.0.md 的改名对照表是验收明写的唯一例外。第一轮扫的是旧命令名，没扫『调用自身可执行文件的地方』，所以漏了第三处"
  - acceptance_id: "AC-004"
    outcome: "passed"
    method: "--format 在 cli.rs 是 global 参数，22 个叶子都收得到；枚举源码里全部 output::emit 的命令名跟 22 个叶子对表；抽 11 条命令（含隐藏的 aw-status 与两条 --dry-run）实跑 --format json，数 stdout 上的 JSON 文档个数并确认 data 非空"
    evidence: "emit 的命令名并集恰好 22 个：workspace 6 + task 8 + build 5 + skill 3；11 条实跑各只有 1 个 JSON 文档、异常 0 条；写操作返回它做了什么：task delete --dry-run 返回 data.dryRun=true 与 data.plugins 一项，task merge --dry-run 返回每个插件的分支、仓库、worktree、待合并 11 个提交与领先落后数。隐藏命令 aw-status 原来吐两个文档（自己打印的固定契约加安全网补的信封），ea9fa55 用 mark_emitted 修掉，现在是 1 个且保持 AgentWatcher 在读的原形状"
  - acceptance_id: "AC-005"
    outcome: "passed"
    method: "对九条接受任务引用的命令跑 --help，读用法行里的参数名与方括号尖括号；再对着设计第 120-122 行核对可选性分类"
    evidence: "九条用法行全部写 TASK_REF，grep 全仓不再有 TASK_ID；可选性：task next、task finish、build check、build status 可省略，task switch / merge / cleanup / delete 与 build task 必填。第一轮 build status 是必填，与设计要求的查询类可省略不符，46fe42a 改成可省略并走 latest_task_ref；复跑 udf build status --format json 省略参数时解析到 neon-dev1/19-aesworld-roadmap"
  - acceptance_id: "AC-006"
    outcome: "passed"
    method: "脚本从五份文档抽出所有以 udf 开头的整行示例，逐条判定命令路径，再把每个双横线参数拿去跟对应叶子的 --help 文本比对；帮助文本全面中文化之后重跑一次；另外扫 27 个帮助页有没有残留的英文骨架"
    evidence: "抽出 137 条完整示例，覆盖 18 个不同命令路径，命令路径全部解析成功、参数核对失败 0 条；27 个帮助页（根 + 4 个组 + 22 个叶子）里 Usage:/Commands:/Options:/Arguments:/Print this message/Print help/Print version 七个英文骨架残留 0 处。人读不读得懂、照着做能不能达到目的，机器验不了，转 manual-test.md"
  - acceptance_id: "AC-007"
    outcome: "passed"
    method: "在末版 ea9fa55 上依次跑三条门禁并取退出码，再统计用例总数"
    evidence: "cargo fmt --check 退出码 0；cargo clippy --workspace --all-targets --all-features --locked -- -D warnings 退出码 0；cargo test 退出码 0，四个测试二进制合计 71 passed / 0 failed（基线 65，新增 6）；rustc 1.96.1"
environment: "Windows 11 Pro 26200; rustc 1.96.1; PowerShell 5.1; workspace neon-dev1 at F:/ShanghaiP4/neon/UGA/DEV_1; 12 real tasks; second round, after manual check round 1"
executed_at: "2026-08-04T03:50:00Z"
supersedes: ar_01KZ40N1BGZ6GJRGHMTZM585QH
dependencies:
  work_item_contract_digest: sha256:67ad0154f02e36b1cee584f2046af59d2efcb31ccf6d313d1ae88366a0760e91
  artifacts:
    - artifact_id: ar_01KZ5DCTJAWBDSR926HPJGZ2DF
      digest: sha256:7297f60b661b872f2a195086a5f7cb30fa18e70147dd7bb4338329cc1eeb8040
      locator: reviews/code-review.md
  subject:
    kind: change_set
    digest: sha256:74cf36a04a4116b1603a55ffb325f397709574ec851f759e5daff7f4f200a2d1
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: ea9fa55ac58c7ecfd6f6b2fb7e9a05f52047102a
    revision: ea9fa55ac58c7ecfd6f6b2fb7e9a05f52047102a
    tree: 924a56dfeb98558e7ff3bc46f743abfca25aebe2
    content_digest: sha256:74cf36a04a4116b1603a55ffb325f397709574ec851f759e5daff7f4f200a2d1
    branch_or_pr: refactor/tidy-cli-surface
    workflow_excluded: true
---

## 在什么状态上验的

分支 `refactor/tidy-cli-surface`，版本 `ea9fa55`，基线 `bad2263`，16 个提交。
代码区干净，只有 `workflow/` 未提交。

环境：Windows 11 Pro 26200，rustc 1.96.1，workspace `neon-dev1`
（`F:\ShanghaiP4\neon\UGA\DEV_1`），真实任务 12 个。

## 第二轮验收，因为第一轮的方法有缺陷

人工核对第一轮打回三条，其中两条本该由 AC-001 拦住。

**AC-001 的验证方法是错的。** 我在临时目录里只塞了一个 `unrealdevflow.exe`——那正好是
安装器代码里硬编码要删的那个文件名。等于拿被测代码的假设去造测试数据，必然通过，什么都
没验到。用户机器上真实的形态是 `unrealdevflow.cmd`（指向仓库的开发垫片）加
`unrealdevflow.installed-20260630.exe`，安装器一个都没删，所以旧名还能敲通。

这一轮改用用户机器的真实目录形态重造场景：四个文件，三个旧入口加一个同名前缀的发布资产，
验的是「该删的都删了、不该删的没动」。

**AC-003 扫的范围不对。** 我扫的是 `build-project` 这类旧命令名，没扫「调用自身可执行文件
的地方」。`scripts/install.ps1:223` 敲的 `udf skills install` 就这么漏过去了——S7 把
`skills` 单数化时没跟上，命令报 `unrecognized subcommand`，而脚本无条件打印成功。
这一轮把 `udf skills ` 这个组合也加进扫描模式。

**AC-004 也补了一个漏网的。** 隐藏命令 `aw-status --format json` 吐两个 JSON 文档。
它不在 22 个叶子里，所以第一轮按范围没查它，但「一条命令一个文档」这条不变量它也该守。

## 逐条结果

七条全 `passed`。人工核对第二轮 4 条待测，见 `manual-test.md`。
