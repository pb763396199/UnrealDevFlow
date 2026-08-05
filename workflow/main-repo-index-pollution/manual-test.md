---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01KZ93TSVXXHWP6QFS4CGJ8Q94
work_item_id: wi_01KZ921GEY99YYYE1S9GVNC501
created_at: 2026-08-05T14:40:00Z
producer: aes-validate
result: passed
supersedes: null
dependencies:
  work_item_contract_digest: sha256:8e8cfc19cd5d95fa0a6d13766dc6d2bbe7ec238051e1bd0772d5fa2863d4c2ba
  artifacts:
    - artifact_id: ar_01KZ93TSS81QFHTPWTH7QVBZ3J
      digest: sha256:05df4e2f7f37200ea36365c315f4c14a8b9e57d686ba98bdfd5b5bfd58635d88
      locator: validation.md
---

## 这次没有要你手工核对的事

零条目。这个缺陷的每一个判据都是 git 状态，机器验得了：暂存区空不空、有没有未合并条目、
有没有留下 `MERGE_HEAD` 或 `CHERRY_PICK_HEAD`。没有界面，也没有要人判断「看着对不对」的东西。

**有一件事想让你知道，不是核对项**：修复没有回溯清理已经发生的污染。如果哪个插件仓库现在
还留着旧版本造成的残留，装上新版之后它不会自己消失。真碰上了，`git status` 看一眼，确认那些
不是你自己的改动，再决定怎么处置——我不替你动。
