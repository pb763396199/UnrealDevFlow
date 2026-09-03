---
schema_version: 1
protocol: 1.3.0
artifact: research
artifact_id: ar_01M1B7ZVMZARF55F5GS4BRZWWT
work_item_id: wi_01M1B7SC2BD0Y4GMA8KG2EGBWQ
created_at: 2026-08-31T06:34:00Z
producer: aes-research
result: complete
topic: 独立复核
dependencies:
  work_item_contract_digest: sha256:f7aeaedd7b0a0eaf96f1cd22835f0b0889bd2f4f33e23993948dab53de528229
  artifacts:
    - artifact_id: ar_01M1B7ZA1ZDR02TGP7GHJ4M0YH
      digest: sha256:9a994a595303201b171cc515a5a57bbde46994345d0fa4762df782306c3c6ff0
      locator: research/profile-research.md
    - artifact_id: ar_01M1B7ZHG8VNX6YR9J5P36EYMQ
      digest: sha256:d9dd5b00a14eb00cbbc84a67a75ba44a416c395a1a97bb0c653f949ef93affb7
      locator: research/source-research.md
    - artifact_id: ar_01M1B7ZKEHST4M0DAW8DGX3P4H
      digest: sha256:d88e2d0a0867479e97e96a58c33bb52487b94e7f15c42dfec7fca3a5b4bea995
      locator: research/uat-research.md
    - artifact_id: ar_01M1B7ZNFFN4YRFFT2ZMC958VA
      digest: sha256:1706e77e488964be97b090a77cfea2edfa0f4a219cb59b40b4b8f0a509811e1e
      locator: research/delivery-research.md
    - artifact_id: ar_01M1B7ZQW49AZ2AA11QV6BZ1NE
      digest: sha256:2150a7004554a8096bca5657cd4d4091193ed3bd2f45606cc6211a86d3dc366a
      locator: research/execution-research.md
    - artifact_id: ar_01M1B7ZSSZ97KJQHC3X8NKATM9
      digest: sha256:f21f99693694325de5eb31fe2ffd2022ea4066482dc86af8922a3618e97fbfb0
      locator: research/compatibility-research.md
---

# 六份调查的独立复核

## 结论与边界

独立 verifier Huygens（01a05681-f835-7b01-bce1-20f4ae36494b）返回 PASS，无阻断问题。
主会话按 aes-research 整理这份证据，作为六张 research 票批量收尾的依据。
复核者没有写记录、修改配置、运行 Cargo 或 UE；此结论不代表项目副本打包已验证。

## 实际核对

| 检查 | 方法 | 结果 |
| --- | --- | --- |
| 六份调查的事实边界 | 逐份 Get-Content，核对源码 | 均区分事实、推断和未知，没有宣称真实 UE 副本已验证 |
| 首次失败日志 | 逐行解析 UAT 耗时/退出码及 Source/Target 配对 | 25m47s，25，11/4/17 |
| 第二次失败日志 | 同上 | 21m53s，25，11/4/17 |
| 第三次失败日志 | 同上，统计 summary 前 error | 96m28s，25，11/5/18；36 条依赖错误加 1 条 LoadErrors |
| 新增错误 | Select-String T_Point、Base_Bridge001 | 358 行找不到 T_Point；9922 与 15979 行包摘要无效 |
| 成功对照 | Select-String FULL COOK、GetFeatureID、ExitCode | 1946 行 FULL COOK；16696 行退出 0；Shader 警告仍存在 |
| task 实例 | create.rs:373、385 | task_uid 可重用，created 必须参与绑定 |
| 旧 task 语义 | package.rs:645 至 673 | Host 项目、Win64、Development |
| 公共执行结构 | execution.rs 与 source_context.rs | 可复用 ExecutionPlan 与 SourceContext |
| loose 参数 | UE 5.5 ProjectParams.cs:798 至 801 | skippak 隐含 Pak=true，不能表示 loose |

原始文件完整路径见 ../reference/reference.md。来源日志统计脚本按 summary 前错误计数，
避免把 LogInit 重复打印当成第二份错误；依赖边按 source/target 集合去重。

## 复核修正

成功日志的退出码定位补至 16696，GetFeatureID 定位补至 12053/12054。
这些只是行号精度修正，不改变调查结论。

## 未覆盖

- 没有实测独立 UE 项目副本或运行时 smoke。
- 没有验证 Windows 交付崩溃恢复。
- 没有证明外部数据不被其他进程修改。
- 设计阶段可以采用上述方向，实施计划仍需对应可行性测试。

