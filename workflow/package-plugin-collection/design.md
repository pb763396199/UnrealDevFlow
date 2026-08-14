---
schema_version: 1
protocol: 1.3.0
artifact: design
artifact_id: ar_01KZZ8HKF8BM9SEN2S6VEJ7KP3
work_item_id: wi_01KZZ8HKAQ08JBV7YZ9NG6SF35
created_at: 2026-08-14T04:30:06Z
producer: aes-brainstorm
result: accepted
supersedes: null
dependencies:
  work_item_contract_digest: sha256:dc9921869990a1b73089b132507e40c0d18751a799265e8f3a0a9b327aead7ab
  artifacts: []
---

# 设计

## 问题

现有实现固定查找 `<名字>/<名字>.uplugin`。UnrealMCP 是集合根目录，真正的描述文件位于多层子目录，因此命令在进入工具链检查前就失败。

## 方案

先递归建立“插件名到相对目录”的索引。参数命中同名单插件时保持旧行为；参数命中没有同名描述文件但包含插件的目录时，将目录下全部插件展开为 seed。依赖闭包通过索引按插件名解析，暂存时保留相对目录结构，发布输出仍以用户给出的集合名作为一个目录。

## 取舍

没有要求用户手写 26 个插件名，因为这会泄漏集合内部结构，也无法稳定复用。没有把所有插件拍平到一级目录，因为 UnrealMCP 的目录组织本身可能承载资源和相对位置约定。

## 失败边界

集合为空或名字不存在时明确失败；递归范围内出现重名插件时明确列出两个目录并拒绝猜测。

## 完成标准

真实 UnrealMCP check 为 ready；plan 展开 26 个插件的 78 个阶段，全部使用 `-NoMutex`，不生成 execution ID。
