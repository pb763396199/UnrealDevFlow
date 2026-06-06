# docs 索引总览

## 顶层导航

| 目录 | 用途 |
|---|---|
| `docs/brainstorms/` | 需求/方向头脑风暴 |
| `docs/research/` | 调研、技术对比、专家审核 |
| `docs/plans/` | 具体实施计划（按 supersede 版本演进） |
| `docs/solutions/` | 已落地方案与操作手册 |
| `docs/insights/` | 可复用洞察 / 经验沉淀 |
| `docs/retrospectives/` | 复盘 |
| `docs/roadmap/` | 中长期路线图 |
| `docs/inbox/` | 未分类草稿临时区 |
| `docs/todos/` | 待办与跟踪项 |
| `docs/index/` | 本目录：索引页 |

## 命名约定

- 文件名格式：`YYYY-MM-DD-NNN-{type}-{topic}-{desc}.md`
- `type ∈ {feat, fix, refactor, process}`
- 全部 `kebab-case`，避免空格 / 大写 / 下划线
- 版本演进通过 `-vN` 后缀 + frontmatter `supersedes` 字段双重表达

## frontmatter 必填字段

| 类别 | 必填字段 |
|---|---|
| brainstorm  | `title`, `type`, `status`, `date` |
| plan        | `title`, `type`, `status`, `date`, `origin`（可选 `supersedes`） |
| solution    | `title`, `status`, `date`, `origin`（list of wiki-link） |
| todo        | `title`, `type`, `status`, `date`, `owner` |

`status` 常见值：`draft / active / superseded / done / archived`

## 当前文档

| 文件 | 类型 | 状态 | 审查 |
|---|---|---|---|
| [UnrealDevFlow 多任务并行开发工具设计](../brainstorms/2026-06-05-001-multi-task-parallel-development-brainstorm.md) | brainstorm | active | ✅ 已审查 (go-with-conditions) |
| [UnrealDevFlow 实现计划](../plans/2026-06-05-001-feat-unrealdevflow-implementation-plan.md) | plan | draft | ✅ 已修复所有审查问题 |
