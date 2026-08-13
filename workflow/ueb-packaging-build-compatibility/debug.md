---
schema_version: 1
protocol: 1.3.0
artifact: debug
artifact_id: ar_01KZXE664C2F3T5YHVKBYWECNG
work_item_id: wi_01KZWHP5WGDDBVMW0WVRJTAJZE
created_at: 2026-08-13T10:56:00Z
producer: aes-debug
result: fixed
supersedes: ar_01KZXE6607RD14QVTTPJJF8J77
dependencies:
  work_item_contract_digest: sha256:57b10aa47e84304ef4ecf7f309eea2b72f99bc3f110ea3a68a92e08c803731b4
  artifacts:
    - artifact_id: ar_01KZX9F39MRS0R9RANVB02Y67R
      digest: sha256:1822c4fef51cf5fe9533e12ca51e344f5348840aab577a4a0b15b1de36dc0943
      locator: plan.md
  subject:
    kind: change_set
    digest: sha256:3d13fc82e5166b396e5d98a2ade107529797777faa244e8ce76e9b341bafdc41
    repository: https://github.com/pb763396199/UnrealDevFlow.git
    base_revision: 5138373f7a15d01c95a06022c153f73c8be7b3e6
    revision: df02180b3ec0e11d3584bde3214eb073f5b05316
    tree: af0bd4ffee4f0295817642cae110f872a093ef26
    content_digest: sha256:3d13fc82e5166b396e5d98a2ade107529797777faa244e8ce76e9b341bafdc41
    branch_or_pr: research/ueb-packaging-build-compatibility
    workflow_excluded: true
---

# 插件打包占用全局 Mutex

## 复现

运行 `udf package plugin AesWorld --workspace neon-dev1` 后，另一个 Host 编译报告全局 UBT Mutex 被该打包任务占用。进程树显示 PID 44564 的三个直接 UBT 命令带 `-WaitMutex`。

## 根因

插件源码和项目中间产物已复制到每次执行独立的 staging，但命令生成器仍套用了普通 Host 构建的等待策略。隔离执行和全局等待策略互相矛盾。

## 修复

- 终止 PID 44564 及其子进程，释放错误占用。
- 三个插件 UBT 目标固定使用 `-NoMutex`。
- 新增 `isolated_plugin_matrix_never_takes_engine_global_mutex`，逐条拒绝 `-WaitMutex`。
- 排查其他路径。Host、主项目、BuildCookRun 和源码引擎会共享中间产物，保留受控 Mutex；Installed Build 交给 BuildGraph 管理。

## 证据

- 修复后 `udf build check --workspace neon-dev1` 返回 `ready` 和 `mutexStatus: available`。
- 修复后的插件 plan/check 三个 argv 都含 `-NoMutex`，没有 `-WaitMutex`。
- 全量测试、格式和 clippy 检查通过。
