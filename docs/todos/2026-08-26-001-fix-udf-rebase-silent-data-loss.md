---
title: udf rebase 静默丢弃目标分支增量代码
type: fix
status: active
date: 2026-08-26
owner: 待认领
---

# udf rebase 静默丢弃目标分支增量代码

> 在实际使用 `udf task merge --strategy rebase` 时发现的问题，事后经 `git range-diff` 与逐文件逐行 diff 复核确认。记录用于后续排查修复，不依赖本次会话上下文也应能看懂问题、复现路径和排查方向。

## 现象

在插件仓库执行 `udf task merge <task-ref> --plugin <plugin> --strategy rebase --yes` 后，工具报告 `Replay completed successfully` 与 `Branch ... rebased successfully`，整个过程没有任何冲突提示。但事后用 `git range-diff` 和逐文件逐行 diff 复核，发现 rebase 结果里静默丢失了目标分支（dev）上刚合入的一份代码，working tree 里对应文件的内容回退成了那次更新之前的旧版本。

丢失内容具体为目标分支上的一个提交，涉及 4 个文件，净新增约 341 行代码（一个新函数与一套完整的交互事件处理逻辑）。执行 rebase 的功能分支的全部提交，经 `git show --name-only` 逐个核实，都没有改动过这几个文件路径，双方改动没有交集，正常情况下不应发生任何内容替换或丢失。

这是一个静默数据丢失问题。命令没有报错，没有警告，没有冲突提示，报告执行成功，但实际上悄悄丢弃了目标分支上其他开发者的代码。如果没有养成 rebase 后用 range-diff 或逐文件 diff 复核的习惯，这类丢失很难被及时发现，等发现时可能已经过了很久，甚至已经 push 到远端污染了共享分支。这比"报冲突要求人工解决"危险得多。

## 复现环境

- 插件仓库：AesWorld（主仓库路径 `F:\ShanghaiP4\neon\Plugins\AesWorld`），主 worktree checkout 在 dev 分支
- 命令：`udf task merge <task-ref> --plugin AesWorld --strategy rebase --yes`
- 功能分支：13 个提交，均未涉及下方丢失内容所在的文件路径
- 丢失内容：dev 分支提交 `fa69fa45c`（"支持拾取道路标线端点创建连接线"）引入的改动，涉及以下 4 个文件：
  - `Source/EarthModeler/Private/InteractiveTool/EarthSplineInteractiveToolBase.cpp`
  - `Source/EarthModeler/Private/InteractiveTool/EarthSplineInteractiveToolBase.h`
  - `Source/EarthModeler/Private/InteractiveTool/Tools/EarthAddCustomTool.cpp`
  - `Source/EarthModeler/Private/InteractiveTool/Tools/EarthAddCustomTool.h`
- rebase 结果里这 4 个文件的内容，回退成了 `fa69fa45c` 的父提交（即这次更新之前）的版本

## 复现路径与关键线索

1. 第一次执行 `udf task merge ... --strategy rebase --yes` 时，工具报错退出：`Rebase failed: ... Cannot replay task branch '...': target worktree is not clean`，并打印出主仓库当时的 `git status`。里面同时列出：
   - 上述 4 个 `Source/EarthModeler/...` 文件，状态为 `M `（staged modified，index 与 worktree 一致，相对 HEAD 有改动）
   - 另一批与本次任务完全无关的未提交内容（属于别的任务的 workflow 目录删除和未跟踪文件）

   事后核查发现，这 4 个 `M` 状态文件的 working tree 内容，实际等于 `fa69fa45c` 的父提交（fast-forward 之前的旧版本）。也就是说，工具在这次失败的尝试里，已经把主仓库本地 dev 分支的 HEAD ref 指针 fast-forward 移动到了 `fa69fa45c`，但没有把这 4 个文件的 working tree/index 同步更新到位，检测到别的路径不干净后就中止退出了，只完成了一半的 fast-forward。

2. 手动清理：用 `git stash` 挪走无关的未提交内容，再用 `git checkout HEAD -- <4个文件>` 把这 4 个文件强制同步到当时 HEAD（`fa69fa45c`）的内容，使 `git status` 完全干净。

3. 重新执行 `udf task merge ... --strategy rebase --yes`，这次工具输出：

   ```
   Fetching latest from origin...
   Fast-forwarding current branch from upstream 'origin/dev'...
   Merging plugin 'AesWorld' using Rebase strategy...
   Task metadata base 6a9e1245 differs from actual merge-base for target branch 'dev'; replaying from actual merge-base 4bce338d.
   Replaying 13 commit(s) from 'feature/xxx' onto 'dev' (fa69fa45)
     Backup ref: refs/udf/merge-backups/dev/1787729186
     Replay base: 4bce338d (actual merge-base)
   Replay completed successfully
   Branch 'feature/xxx' rebased successfully
   ```

   全程报告成功，没有冲突提示。但事后核查，rebase 结果里那 4 个 EarthModeler 文件又变回了旧版本内容，即上文"现象"一节所述的丢失。

4. 对照实验：从干净的 `git checkout -b tmp fa69fa45c` 开始，依次用纯 `git cherry-pick` 手工重放同样的 13 个原始提交（不经过 udf），得到的结果完全正确，4 个 EarthModeler 文件与 `fa69fa45c` 保持一致，没有丢失任何内容。说明问题不在 git 本身的 cherry-pick 机制，而在 udf 的 rebase/replay 实现。

## 根因猜测（未验证，仅供排查方向参考）

猜测一：udf 的 rebase/replay 实现在执行 cherry-pick 序列之前，某个环节复用了主仓库当前 working tree 的磁盘文件内容作为基底或上下文，而不是纯粹基于 git 对象库操作。第一次"fast-forward 到一半就中止"的失败尝试，让 working tree 残留了过期的文件内容（上文第 1 点提到的那 4 个文件）。即使之后手动用 `git checkout HEAD --` 把 working tree 表面上恢复成和 HEAD 一致，如果 udf 内部另有一份缓存或快照记录了那次失败尝试时的脏状态（例如某个临时目录、内部索引，或者从 `refs/udf/merge-backups/...` 之类的备份点取错了基底），就可能导致重放时用了错误的基底，把 dev 后来通过 fast-forward 刚合入、但从未"干净落地"过的这部分内容重新覆盖掉。

猜测二：重放算法本身没有问题，但"检测 worktree 是否干净"这一步和"实际执行 fast-forward 加 replay"这两步之间存在状态不一致的窗口。第一次失败退出时，fast-forward 没有被完整回滚（只挪了 ref，没同步 worktree/index）；第二次重试时，工具可能误判"当前 worktree 已经是全新、干净的 fa69fa45c 状态"而直接复用了残留的、不完整的中间态数据结构去做 replay，没有重新完整校验。

以上两点都只是根据现象做的推测，需要看 udf 源码里 `task merge`/rebase replay 部分的实现（fast-forward 步骤和 cherry-pick/replay 步骤之间的状态传递）才能确认真正原因。

## 排查方向提示

从复现路径看，触发条件里有一步比较关键：先经历一次因 worktree 不干净而中止的失败尝试（这次尝试里 fast-forward 半途而废，ref 已移动但 worktree/index 未同步），再在同一个 worktree 上重试。可以优先检查这条路径涉及的代码：

- fast-forward 步骤失败时，是否有完整回滚 ref 的逻辑
- replay 前"worktree 是否干净"的校验，等价的是"worktree 与当前 HEAD 内容一致"，还是只检查了 index 状态
- replay 使用的基底具体从哪里取，是否可能取到上一次失败尝试遗留的中间态

## 风险等级

高。静默数据丢失，没有任何错误、警告或冲突提示，命令报告成功，但实际丢弃了目标分支上其他开发者的代码，且只在恰好经历过"fast-forward 半途中止后重试"这种路径时触发，不易复现，也不易在日常使用中被发现。
