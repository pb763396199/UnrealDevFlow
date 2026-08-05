---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01KZ90VZVTBJHCDYE5XRSTD0RF
work_item_id: wi_01KZ8TFVAGQNAZH74W8JDZG6GA
created_at: 2026-08-04T11:55:00Z
producer: aes-validate
result: passed
supersedes: null
dependencies:
  work_item_contract_digest: sha256:33710c8da277ec096e0ee4c641b5343bb94e79e99040433659321b2812de8ed9
  artifacts:
    - artifact_id: ar_01KZ90SP8SX7A064YVG6VEGZZF
      digest: sha256:7b4e3ec63a76695d17a56259790fb6d66bdf7851046204590c56ff6175a5348b
      locator: validation.md
---

## 这次没有要你手工核对的事

零条目，原因写在下面。

这次改动的每一个可观察结果都是命令输出或 git 状态，机器验得了：

- 分支叫什么，`git branch` 说了算。
- 台账记了什么，`git config --get` 说了算。
- 孤儿清理有没有把分支清掉，清理前后 `git branch --list` 一比就知道。
- 非法类型有没有被拒，看退出码和 stderr。

没有界面、没有需要人判断「看着对不对」的东西，所以清单是空的。

**唯一想请你留意的不是核对项，是知情**：以后建任务时，工具会往你主插件仓库的
`.git/config` 里写一行，形如：

```
[branch "feature/save-bug"]
	udftask = neon-dev1/save-bug
```

删分支时 git 自己带走，正常用完不留东西。你要是看到这行，它就是干这个的。
