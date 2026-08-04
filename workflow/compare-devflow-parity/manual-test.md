---
schema_version: 1
protocol: 1.3.0
artifact: manual-test
artifact_id: ar_01KZ3FA0CY84ESQAC8GFQ6WV3Y
work_item_id: wi_01KZ35W0S9TYQ7V4PFHSDKC5SM
created_at: 2026-08-03T09:29:33.342846Z
producer: aes-validate
result: pending
supersedes: null
dependencies:
  work_item_contract_digest: sha256:49d4bfe999b674c14b3b950b648deb9398a2b8fa776407ce710f864911dfebb1
  artifacts:
    - artifact_id: ar_01KZ3F3TNRPR5CTASREF550CFY
      digest: sha256:98aed2b86bc33dd62895d9db8e819396ae88322104770e93c30e39ece386e8f3
      locator: validation.md
---

# 人工核对清单

这里只放机器证明不了的。自动化测试和验收记录已经覆盖的（`build-check` 的四种结论、
`build-gate` 的放行与拦截、元数据回退的三种坏法、多插件切换与合并）不重复。

跑之前先确认：装的是本分支 `feature/compare-devflow-parity` 编出来的 `unrealdevflow`，
不是 GitHub Release 版本。

勾选方式：`[ ]` 还没测，`[x]` 过了，`[!]` 没过。没过的在下面缩进写你看到的现象，
直接改这个文件就是反馈，不用告诉我。

## 一、编译主项目（最花时间，也最重要）

自动化测试只验到「参数拼对了」，没有真的编过一次。

- [ ] 1.1 跑 `unrealdevflow build-project --workspace <你的workspace>`。
  编译输出应该**实时滚动在屏幕上**。如果是憋到最后一次性冒出来，或者半天没反应，就是没过。

- [ ] 1.2 上一步跑完，去 `<你的UE项目目录>\Saved\Logs\UnrealDevFlow\` 看一眼。
  应该有一个 `BuildProject_light_<数字>.log`，里面是这次编译的完整日志。

- [ ] 1.3 故意改坏一个 cpp 让它编译不过，再跑一次。命令应该报错结束，
  并且屏幕上「UBT 日志：」那行指向的文件里能看到具体错在哪。

## 二、任务只能切自己的项目

这是本次唯一一处用户能感觉到的行为收紧，需要在真实项目上确认没切错东西。

- [ ] 2.1 挑一个任务跑 `unrealdevflow switch <workspace>/<任务>`，正常切换应该成功，
  之后重启 UE Editor 能看到这个任务的代码。

- [ ] 2.2 再跑一次，这次故意指向别的项目：
  `unrealdevflow switch <workspace>/<任务> --project "<另一个UE项目目录>"`。
  应该被拒绝，并且明确说「未修改任何 Junction」。

- [ ] 2.3 拒绝之后回去看 2.1 那个项目的 `Plugins` 目录，Junction 应该还是 2.1 切好的样子，
  一点没动过。

## 三、清理不会假装成功

守卫是按代码路径确认的，没有在真实任务上跑过。**用一个不要紧的任务做，先备份 Host 目录。**

- [ ] 3.1 把这个任务的 `.udf-meta.json` 里 `primary_plugins` 手工改成空数组 `[]`，保存。

- [ ] 3.2 跑 `unrealdevflow cleanup <workspace>/<任务>`。应该被拒绝，
  提示元数据里没有主插件身份、继续做会留下清不掉的 worktree 和分支。

- [ ] 3.3 跑 `unrealdevflow delete <workspace>/<任务>`（不加 `--force`）。同样应该被拒绝。

- [ ] 3.4 跑 `unrealdevflow delete <workspace>/<任务> --force`。这次应该放行，
  但会先打一条警告，说 Host 会删掉、git worktree 注册和分支要你手工清理。

- [ ] 3.5 删完在对应插件仓库里跑 `git worktree list`，确认警告说的是实话——
  确实还留着一条指向已删目录的记录。测完把备份还原回去。

## 四、损坏的任务在真实数据上不会被吞掉

单元测试用的是临时目录的假数据，这里要在真实 Hosts 目录上确认一次。

- [ ] 4.1 备份一个不要紧的任务 Host 目录，然后把原目录的 `.udf-meta.json`
  用记事本删掉最后半行，存盘。

- [ ] 4.2 跑 `unrealdevflow list`。这个任务应该**仍然出现**，不是消失，
  而是在列表下方以警告形式列出来，带目录和出错原因。

- [ ] 4.3 再跑一次 `unrealdevflow list`。如果同目录有 `.udf-meta.json.bak`，
  任务应该已经恢复正常显示，`.udf-meta.json` 也被自动修回来了。测完还原备份。

## 五、装完就能用

- [ ] 5.1 在一个干净目录里跑 `unrealdevflow skills install`，打开
  `.claude/skills/unrealdevflow/SKILL.md` 搜 `build-check`。
  应该能搜到，并且能看到四种结论的说明表。装到别的 provider 目录的那几份同理。
