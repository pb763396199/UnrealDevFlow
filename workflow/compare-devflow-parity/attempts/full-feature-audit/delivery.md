---
schema_version: 1
artifact: delivery
artifact_id: ar_01KZ3FFKCX7M2NCMNAYEAZ3EM1
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
attempt_id: at_01KZ35WCEAJTX3MNDKH02BCYDS
created_at: 2026-08-03T09:32:36.637219Z
producer: aes-finish
outcome: delivered
landing_branch: "dev"
landing_revision: c704c8c6d1ee4b69420d762126fc4a52cf9e758d
supersedes: ar_01KZ3FCN0WZ0YZK77ZZDXDTRTJ
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts:
    - artifact_id: ar_01KZ3F3TNRPR5CTASREF550CFY
      digest: sha256:f9a8260dde174ad7513518594a3d2700dfea26d0d1c7335677a8bca697354474
      locator: validation.md
  subject:
    kind: change_set
    digest: sha256:f1827294286884456780b511c3b7c8cbfd5dfd73d6a2ecff9226dfc95bd4604a
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: e5fff6055ae6e214e1566ab02bfd011370aa0232
    revision: c704c8c6d1ee4b69420d762126fc4a52cf9e758d
    tree: cf653a656e26d8c7446377d135bdb11fca0dcdd6
    content_digest: sha256:f1827294286884456780b511c3b7c8cbfd5dfd73d6a2ecff9226dfc95bd4604a
    branch_or_pr: feature/compare-devflow-parity
    workflow_excluded: true
---

## 交付什么

`unrealdevflow` 装一个命令就能用，不再需要装 UWF。UWF 侧领先的 1956 行引擎增强和 815 行受控构建
都搬了回来，依赖树里没有任何 `uwf-*` crate。顶层命令从 17 个变成 20 个。

变更集：基线 `e5fff60`，末版 `c704c8c`，分支 `feature/compare-devflow-parity`，11 个代码提交。
代码 20 个文件、2971 行增、296 行删，另有 5 份文档。

## 落到哪，落成什么样

**已落到本地 `dev`，落地版本 `c704c8c`。** 分支是从 `dev` 直接长出来的，期间 `dev` 没有新提交，
所以走的是 `--ff-only` 快进，没有产生合并提交。

**等价性判断：完全相同，不是等价。** `dev` 的头和被审查、被验收的版本是同一个提交
`c704c8c6d1ee4b69420d762126fc4a52cf9e758d`，`feature/compare-devflow-parity` 指向同一处。
没有 rebase、没有 squash、没有冲突解决，不存在需要判断等价的余地。

落地后在 `dev` 上复跑了三件套：`cargo fmt --check`、
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test`
退出码都是 0，65 个用例全过，与落地前一致。

**没有推到 origin。** `origin/dev` 仍停在 `e5fff60`。推送是对外动作，留给用户决定。

目标分支是 `dev` 不是 `master`：`master` 还停在 `a2d3042 feat: initial UnrealDevFlow v0.1.0`，
是发布用的旧分支。

## 凭什么可以落

- **代码评审 `approved`**（`ar_01KZ3EW91M84KGH2GHJPBJM4RK`）。第一版给的是 `changes_requested`，
  两条阻断（`build-project` 可能编错项目、v1 元数据下 `cleanup`/`delete` 静默跳过清理）
  已在 `9dddf37` 和 `c704c8c` 修掉并复审通过。没有被接受的风险，不需要授权信息。
- **验收 `passed`**（`ar_01KZ3F3TNRPR5CTASREF550CFY`）。AC-001 到 AC-005 五条全部通过，
  每条都有方法和证据，没有豁免项。
- **发布三件套在末版通过**：`cargo fmt --check`、
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`、`cargo test`
  退出码都是 0，65 个用例全过。
- **`finish-check` 报 `eligible: true`**，变更集与 Git 实际状态精确一致（`exact_revision: true`）。

## 落地之前要知道的三件事

**一、有一处用户可见的行为收紧。** `switch <task> --project <其他项目>` 现在会被拒绝。
任务绑定的项目取自元数据里冻结的 `TaskContext.default_project`，`switch main` 不受这条约束。
命令名没有改动，已发布的接口没有破坏。

**二、`rust-version` 从 1.85 提到了 1.89。** 这是 `ipc-lock 0.1.4` 的硬要求。CI 用 stable 工具链，
不受影响；但如果有人固定用 1.85 工具链构建，会失败。

**三、人工核对清单还没人跑。** `manual-test.md` 里 5 组共 14 条，`result: pending`。
协议不要求勾完才能收口，但里面第一组（真实编译一次主项目）是本次最大的未覆盖面：
`build-project` 从策略解析到 UBT 退出码的整条链路只有代码走查，没有实跑过。

## 怎么退回去

因为是快进落地且没有推送，整体回退最直接：`git checkout dev && git reset --hard e5fff60`。
`origin/dev` 没动过，所以这个操作不会影响任何其他人。

想只回退其中一部分，这 11 个提交彼此独立，按需要 `git revert <提交>` 即可。

有两处回退需要额外动作，不能只靠 git：

- **元数据 schema。** `write_meta` 现在会在每个 Host 目录留下 `.udf-meta.json.bak`。
  回退代码之后这些文件不会自动消失，但旧代码不读它们，留着无害，可以手工删。
- **Junction 状态。** `switch` 改过的 Junction 是文件系统状态，不在 git 里。
  如果回退时某个项目正挂在任务上，回退后手工跑一次 `unrealdevflow switch main` 复位。

## 未决的、不属于本次交付的

三条 CLI 层面的问题在评审和实现记录里都写明了，都会动已发布行为，需要另立任务：
`--format` 只有 5 个命令真的支持、`build` 与 `build-project` 的命名是反的、
位置参数 `<TASK_ID>` 与 `<TASK_REF>` 不一致。本次只把帮助文案改成不说谎，没有改行为。
