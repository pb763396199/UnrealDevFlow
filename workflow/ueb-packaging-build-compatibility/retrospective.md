---
schema_version: 1
protocol: 1.3.0
artifact: retrospective
artifact_id: ar_01KZYY96047Q0VEAXR9F36GVZ2
work_item_id: wi_01KZWHP5WGDDBVMW0WVRJTAJZE
created_at: 2026-08-14T01:30:52Z
producer: aes-retrospect
result: complete
supersedes: null
dependencies:
  work_item_contract_digest: sha256:57b10aa47e84304ef4ecf7f309eea2b72f99bc3f110ea3a68a92e08c803731b4
  artifacts:
    - artifact_id: ar_01KZXE664C2F3T5YHVKBYWECNG
      digest: sha256:04a584b598db223026d9fbcdbe48b227166b8c9bb7428bbc53798efde6e1a453
      locator: debug.md
    - artifact_id: ar_01KZXF0RM7P1CG7JKN30BZ7C22
      digest: sha256:8027660d9da7046e19c42744a58da798ee5015c186ec7a6cebe6158c9d60a807
      locator: validation.md
    - artifact_id: ar_01KZXF4RHVPCG94F66FPSNTEPY
      digest: sha256:c7872924a039babf25a3d62354f5694e14797fc3148d35ed91c2401f7746261f
      locator: delivery.md
---

# UEB 发布编译兼容任务复盘

## 边界

这份复盘覆盖 UEB 能力研究、Package 命令实现、真实 AesWorld 验证、Mutex 故障修复和任务收口。代码从 `8f7ba3b` 开始，交付记录由 `ea69872` 落盘。度量脚本没有安装，因此本文不填写 token、轮数或耗时估算。

## 经过

1. 先读取 UEB 命令实现，把项目发布、插件发布、源码引擎构建和 Installed Build 映射到 UDF 的一级、二级命令。
2. 实现 `package`、`build engine`、`build plan` 和 `workspace inspect`，并补齐执行记录、manifest、状态与受管清理。
3. 用 `neon-dev1` 和 AesWorld 跑真实插件矩阵。Editor Development 完成 1661 个动作，Game Development 在 `AesRasterAdapterTests.cpp:31` 暴露插件源码错误。
4. 插件发布最初把普通 Host 的 `-WaitMutex` 策略带进了隔离 staging，PID 44564 占用全局 UBT Mutex。修复后，三个隔离插件目标固定使用 `-NoMutex`，Host、项目和 BuildCookRun 保留受控 Mutex。
5. 独立评审首轮提出 5 个问题，修复后复审批准。六条验收标准全部由固定用例通过。
6. 收口时又发生两次流程返工：`blocking_findings` 写成数字 `0`，不符合协议要求的空列表；代码先快进到 `dev`，随后才回任务分支提交 workflow 记录。交付完成后，`state.json` 已给出 `next_skill: aes-retrospect`，我仍然停止，直到用户指出才补写本文件。

## 做法评估

| 做法 | 结果 | 证据 | 判断 |
| --- | --- | --- | --- |
| 先研究 UEB 源码再定 UDF 分类 | Package 成为独立一级分类，同名二级命令共用解释与状态语义 | `design.md`、AC-001 至 AC-005 | 有用，避免只复制命令行而破坏 UDF 的交互分类 |
| 用真实 AesWorld 跑插件矩阵 | Editor 通过，Game 找到 `std::ldexp` 源码错误 | `implementation.md`、`validation.md` | 有用，证明发布验证必须覆盖非 Editor Target |
| 隔离 staging 复用普通 Host Mutex 策略 | 打包进程阻塞了另一条 Host 编译 | `debug.md` 中 PID 44564 和三个 `-WaitMutex` 命令 | 无效，文件隔离后仍沿用了错误的并发策略 |
| 为 Mutex 修复增加参数级回归测试 | 三个插件目标都要求 `-NoMutex` 并拒绝 `-WaitMutex` | AC-006、`isolated_plugin_matrix_never_takes_engine_global_mutex` | 有用，把事故条件固定成可重复检查 |
| 把代码评审放在真实执行之后 | 首轮 5 个问题在交付前全部修复 | `reviews/code-review.md` | 有用，但评审太晚，基础的状态和清理契约应该更早核对 |
| 依赖记忆手写收口元数据和步骤顺序 | 元数据类型错误，分支落地顺序返工，复盘漏跑 | `ea69872` 的收口提交、最终 `state.json`、本次用户纠正 | 无效，执行者没有逐项消费工具给出的状态和协议字段 |

## 根因

Mutex 事故源自抽象边界选错。实现按“都是 UBT 命令”复用了等待策略，却没有按“是否共享中间产物”选择并发策略。隔离 staging 拥有独立中间产物，因此不应申请引擎全局 Mutex。Host、主项目和 BuildCookRun 会共享项目或引擎中间产物，仍需要受控 Mutex。

收口返工和漏复盘源自执行方式不稳定。我把协议当作背景说明，凭记忆安排步骤，没有把 `commit-guard`、`finish-check` 和 `state.json.next_skill` 当作逐步驱动。工具已经给出正确约束，但我在不正确的时间读取，或者没有继续读取下一步。

## 可复用改进

| 改进 | 收益 | 风险 | 负责人 | 验证方法 |
| --- | --- | --- | --- | --- |
| 每条新 UE 执行路径先标记中间产物所有者，再选择 Mutex 策略 | 防止隔离任务占用全局锁，也防止共享目录被并发写坏 | 所有权判断错误会放开本应串行的构建 | 发布命令实现者 | 每种路径都有参数测试；隔离路径拒绝 `-WaitMutex`，共享路径拒绝无依据的 `-NoMutex` |
| 发布能力至少覆盖 Editor、Game、Shipping 的计划与执行检查 | 提前发现只在非 Editor Target 出现的源码或依赖问题 | 真实矩阵耗时更长 | 验收执行者 | 验收记录逐个保存 target、退出码、日志和制品 |
| 收口按工具输出推进，不凭记忆重排 | 减少元数据类型错误和保护分支返工 | 工具状态过期时需要先重算索引 | 收口执行者 | `commit-guard`、`finish-check`、`merge-check` 按顺序通过，最终 `git log` 无 merge commit |
| 交付回复前读取 `state.json.next_skill` | 避免已交付任务遗漏复盘 | `state.json` 可能过期 | 当前会话负责人 | 先用 `status` 判断索引是否新鲜；下一步为 `aes-retrospect` 时必须有 `retrospective.md` 或明确的无复盘理由 |
| 是否把复盘改为自动串联，另开 `aes-writing-skills` 任务评估 | 把依赖执行者记忆的动作变成显式规则 | 强制复盘可能产生应付记录，与现行“复盘不是门禁”冲突 | AES Workflow 维护者 | 新任务比较自动提示、自动执行和强制门禁三种方案，并用历史任务验证噪声与遗漏率 |

## 结论

本任务的代码和验收结果成立，过程有三次可确认的返工：Mutex 策略错误、收口元数据与分支顺序错误、交付后漏做复盘。前两次已经由测试和门禁固定，第三次由本文件补齐。把复盘自动串联进流程仍是未实施的规则改动，需要独立任务评审。
