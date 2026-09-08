---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01M1Y6N59V3BEHVPA4V0V2ZT57
work_item_id: wi_01M1XH0G01JN3HHH886TS7ZMW4
created_at: 2026-09-07T23:09:20+08:00
producer: aes-brainstorm
result: accepted
supersedes: ar_01M1XHETDT2NJ5JXMTTYS20RFM
dependencies:
  work_item_contract_digest: sha256:d3df2aee22bad7a05acda4ad8ef2acb4afda08c21b9f245b6912657a3fb8108d
  artifacts:
    - artifact_id: ar_01M1XHETDT2NJ5JXMTTYS20RFM
      digest: sha256:157b42df776ef2c074b075260b2d2b49a0a9a4a7a6397283456011d565ad25c4
      locator: design.md
---

# 插件 staging 占用收缩

## 观察到的事实

`%TEMP%\UDF` 当前有 6 个插件打包 staging，共 202.7 GiB。每个 staging 的 `Plugins` 占 10.5 至 34.9 GiB，`Intermediate` 约占 7 GiB。现有 `prepare_plugin_stage()` 会复制整个插件闭包；插件交付成功后只保存 execution，未删除该 staging。一个 delivery 过程异常时还可能把 execution 留在 `running`。

## 方案对照

| 方案 | 做法 | 结果 | 取舍 |
| --- | --- | --- | --- |
| 只在成功后删除 | 继续复制完整 Plugins，交付后删除 stage | 不再长期堆积，但单次打包仍额外占 35 GiB 左右 | 不能解决并发时的大峰值 |
| 私有生成目录加只读 Junction | stage 保留插件根和生成目录，Source、Content、Config、Resources、Shaders、ThirdParty 等子目录建立 Junction | 单次 stage 只保留编译输出，成功后立即删除 | 需要验证 UBT 不向只读源目录写生成文件 |
| 整个插件根建立 Junction | `Plugins/<name>` 直接指向源插件 | 复制量最小 | UBT 会把 Intermediate/Binaries 写回源插件，破坏任务隔离 |

## 采用方案

采用第二种。每个插件 staging 根目录是私有普通目录：复制 `.uplugin` 和其他根文件，创建私有 `Intermediate`、`Binaries`、`Saved`，其余直接子目录建立 Junction 到源插件。这样 UBT 的生成文件仍与源插件隔离，读取 Source、Content 等大目录不再复制。

打包交付后先沿用现有逐文件摘要校验。摘要全部一致时删除私有 staging，并把 execution 写成 `cleanupResult=succeeded`。交付失败时写 `failed` execution，保留 staging、日志、journal 和 backup，不做删除。日志仍按现有保留规则管理。

## 边界

- 不用整个插件根的 Junction。
- 不删除当前机器中 24 小时保护期内的失败或中断 staging。
- 不改变最终输出、manifest、delivery recover 或 Cook cache 的保护规则。
- 只改新版本二进制的后续打包行为；已由旧版二进制创建的目录继续由 inventory 和显式 clean 处理。

## 影响面

| 位置 | 影响 |
| --- | --- |
| `src/commands/package.rs` | 生成插件 stage、交付失败记账、成功后的 stage 回收 |
| `tests/package_lifecycle.rs` | 验证 Junction、私有生成目录和成功清理 |
| `tests/skills/aes-workflow/test_package_cache_lifecycle.py` | 验收桥接新增两条 case |
| `README.md`、`skills/unrealdevflow/SKILL.md` | 说明插件 staging 的新生命周期 |

动手前用 `rg -n "prepare_plugin_stage|plugin_stage_root|cleanup_policy" src tests` 复算以上调用面。
