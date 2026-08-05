---
schema_version: 1
protocol: 1.3.0
artifact: validation
artifact_id: ar_01KZ90SP8SX7A064YVG6VEGZZF
work_item_id: wi_01KZ8TFVAGQNAZH74W8JDZG6GA
created_at: 2026-08-04T11:50:00Z
producer: aes-validate
outcome: passed
executed_at: 2026-08-04T11:50:00Z
environment: "Windows 11 Pro 26200; rustc 1.97.1（rust-toolchain.toml 固定）; 临时 git 仓库，未接触用户的真实插件库"
acceptance:
  - acceptance_id: "AC-001"
    outcome: "passed"
    method: "不给 --type 建任务，读实际创建出来的 git 分支名；再在两个不同 workspace 下用同一个 task-id 各建一次，比对生成规则"
    evidence: "端到端 T1：分支是 feature/add-thing，不含 workspace 名 ws；集成测试 default_branch_name_carries_the_change_type_and_not_the_workspace 除了断言等于 feature/save-bug，还单独断言分支名不含 workspace 字符串。T3：ws 和 ws2 的默认生成规则都是 <type>/<task-id>，同一个 task-id 会撞名，所以第二个必须显式给 --branch——这本身就证明了工程名不再参与生成"
  - acceptance_id: "AC-002"
    outcome: "passed"
    method: "两条路径各建一个任务：--type 指定类型让工具补全其余部分；--branch 直接给完整分支名。都读实际创建出来的 git 分支名"
    evidence: "端到端 T2：--type fix 建出 fix/squash-bug；T4：--branch release/nothing-to-do-with-it 原样生效。集成测试 change_type_picks_the_branch_prefix 与 the_branch_ledger_records_which_task_owns_the_branch 各覆盖一条。udf task create --help 里两个参数都在，--type 的说明写明了「给了 --branch 时被忽略」"
  - acceptance_id: "AC-003"
    outcome: "passed"
    method: "给一个不在词表里的取值，看是否在建 Host 之前就被拒，以及错误信息里有没有列出可选值"
    evidence: "udf task create x --type nonsense 输出 error: invalid value 'nonsense' for '--type <CHANGE_TYPE>' 并列出 [possible values: feature, fix, hotfix, refactor, docs, chore]，六个一个不少；clap 在解析阶段就拒绝，命令体根本没执行。集成测试 an_unknown_change_type_is_refused_before_anything_is_created 另外断言 Host 目录没被建出来"
  - acceptance_id: "AC-004"
    outcome: "passed"
    method: "构造两种历史形态的分支（task/<ws>/<id> 和 task-<id>），都不写台账，跑 cleanup；merge 那边靠原有 15 个用例加两条新的守卫测试"
    evidence: "端到端 T5：task-legacy-flat 和 task/ws/legacy-ns 两个分支都被正常清掉，走的是名字反推那条腿；merge_yes 里原有的 rebase / squash / ff-only 与 cleanup 系列 15 个用例在末版全过，它们用的都是老形态分支名。merge.rs 原有三条判定一条没删，新加的台账判定只在 task_uid 存在时收紧，老元数据退回原逻辑"
  - acceptance_id: "AC-005"
    outcome: "passed"
    method: "建一个分支名完全不含 task-id 的任务，删掉 Host，跑 cleanup；完整任务引用和裸 task-id 两种输入各验一次"
    evidence: "端到端 T4（完整引用 ws/ghost）：release/nothing-to-do-with-it 被精确找到并删除，清理后该分支不存在。裸引用第一轮是失败的——分支留在仓库里而命令报成功，评审记为阻断问题 1，b83b1e3 修复；回归测试 cleanup_reaches_the_ledger_even_when_the_task_is_named_without_its_workspace 做过负向验证：回退代码修复后 FAILED，装回修复后通过"
  - acceptance_id: "AC-006"
    outcome: "passed"
    method: "脚本从五份文档抽出所有以 udf 开头的整行示例，逐条判定命令路径，再把每个双横线参数拿去跟对应叶子的 --help 文本比对"
    evidence: "抽出 138 条完整示例，覆盖 18 个不同命令路径，命令路径全部解析成功、参数核对失败 0 条。五份文档都加了 --type 的取值说明，AGENTS.md 和 skills/unrealdevflow/SKILL.md 另外写了分支命名规则与台账机制的正文"
  - acceptance_id: "AC-007"
    outcome: "passed"
    method: "在末版 b83b1e3 上依次跑三条门禁并取退出码，再统计用例总数"
    evidence: "cargo fmt --check 退出码 0；cargo clippy --workspace --all-targets --all-features --locked -- -D warnings 退出码 0；cargo test 退出码 0，四个测试二进制合计 85 passed / 0 failed（基线 71，新增 14）；rustc 1.97.1"
supersedes: null
dependencies:
  work_item_contract_digest: sha256:33710c8da277ec096e0ee4c641b5343bb94e79e99040433659321b2812de8ed9
  artifacts:
    - artifact_id: ar_01KZ90RQWTN4CZY3Q76MQ2T7GK
      digest: sha256:a40283c6e1f4067851f31e5cd47fe7112948e7867ac9385f1bfc5e942e137651
      locator: reviews/code-review.md
  subject:
    kind: change_set
    digest: sha256:c1c33cc92a83b89fd2a710045a8fc89b914c838547a7a493e020b3c21120fd49
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 395a2625650bfe9bf9a84483b1be31f6d7363830
    revision: b83b1e3aff1ed4392771d65172816decfb4dd089
    tree: 853bc40544d290bbfcb2816e6e0d5a6124f2364d
    content_digest: sha256:c1c33cc92a83b89fd2a710045a8fc89b914c838547a7a493e020b3c21120fd49
    branch_or_pr: feature/branch-naming
    workflow_excluded: true
---

## 在什么状态上验的

分支 `feature/branch-naming`，版本 `b83b1e3`，基线 `395a262`，两个提交。
代码区干净，只有 `workflow/` 未提交。

全部验证在临时 git 仓库里做，没有接触用户的真实插件仓库。

## 逐条结果

七条全 `passed`。

AC-005 第一轮是失败的：分支名完全自定义的任务，用裸 task-id 做孤儿清理时分支留在仓库里，
而命令报「cleaned up successfully」。这条被评审记为阻断问题 1，`b83b1e3` 修完之后复验通过。
两条回归测试都做过负向验证——先回退代码修复确认测试会失败，再装回修复确认通过。

## 转人工的部分

无。这次改动的可观察结果都能用命令和 git 状态验到，没有需要人眼判断的项。

`manual-test.md` 因此是零条目的清单，正文说明了为什么没有人工项。
