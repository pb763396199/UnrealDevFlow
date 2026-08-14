---
schema_version: 1
protocol: 1.3.0
artifact: plan
artifact_id: ar_01KZZ8HKKWZC79ZE6B0EYN61K9
work_item_id: wi_01KZZ8HKAQ08JBV7YZ9NG6SF35
created_at: 2026-08-14T04:30:06Z
producer: aes-plan
result: ready
supersedes: null
dependencies:
  work_item_contract_digest: sha256:dc9921869990a1b73089b132507e40c0d18751a799265e8f3a0a9b327aead7ab
  artifacts:
    - artifact_id: ar_01KZZ8HKF8BM9SEN2S6VEJ7KP3
      digest: sha256:dc841a8b2e3f7ad33f1ae0f05975a5c7b729c37407431ebf7042150ceb711e09
      locator: design.md
---

# 实施计划

| 步骤 | 修改位置 | 行为变化 | 验证与证据 | 退回方法 |
| --- | --- | --- | --- | --- |
| 1 | `tests/package_lifecycle.rs` | 增加嵌套集合回归用例 | 新用例先失败后通过 | 删除用例 |
| 2 | `src/commands/package.rs` | 建递归索引、展开集合、保留暂存层级 | Package 定向测试通过 | 恢复单层解析 |
| 3 | 全仓与真实 Host | 验证兼容、Mutex 和真实计划 | fmt、clippy、全量测试、真实 check/plan | 回到步骤 2 |

## 风险

| 风险 | 控制方法 |
| --- | --- |
| 重名插件解析错误 | 索引阶段立即拒绝重名 |
| 普通单插件回归 | 保留同名单文件优先，并复跑既有测试 |
| 集合打包占用全局 Mutex | 每个展开步骤继续断言 `-NoMutex` |
