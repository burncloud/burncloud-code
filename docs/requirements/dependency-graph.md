# BAM 开发依赖图与调度规范

## 边的方向
`A -> B` 表示 A 依赖 B；B 必须达到 DONE（或明确定义的跨仓库外部里程碑）才允许 A 派工。没有可靠的 GitHub 依赖关联必须标记 UNKNOWN；不存在的 Issue 不能视作已完成。

## 建议的第一批 DAG
```text
F0/config -----------+
F0/domain-types -----+--> F1/github-client --> F1/projects --> F1/issue-pr-reader
F0/sqlite-migrations +--> F0/lease ------+           |                  |
F0/state-engine -----+-------------------+------> F2/scheduler <---------+
F0/cli/doctor -----------------------------> F2/agent-adapter
F2/worktree ------------------------------> F2/git-push-and-draft-pr
F1/github-client --> F3/check-resolver --> F3/ci-evidence --> F3/repair
F3/ci-evidence + F2/git-push-and-draft-pr --> F4/acceptance --> F4/merge-guard
F4/merge-guard --> F4/verify-main --> F5/e2e-and-release
```

## DAG 校验
| 条件 | 行为 |
|---|---|
| 自依赖或环 | 阻断全部受影响节点，报告闭环完整路径 |
| 前置 Issue 关闭但 main 尚未验证 | 若该 Issue 由 BAM 管理，仍需可信 DONE 证据 |
| 前置节点被重开 | 下游未启动的任务重新阻塞；运行中的任务暂停新写入重评 |
| 跨仓库依赖 | 用 repo ID + Issue Node ID 键；权限不足时 UNKNOWN |
| 外部手动完成 | 要求事件及可追溯验收凭证，不能依赖自然语言 |
| 两任务同时修改同一 PR branch | 强互斥；使用 lease epoch 防过期 Worker 写入 |
| WAIT_CI | 不占 Agent 槽，允许其他独立 READY 任务运行 |

## 调度优先序
CI 失败修复 → 可确定的冲突修复 → 已存在 PR 未完成工作 → Draft 验收 → Ready 复核 → 新 Ready Issue → main 验证 → Issue 关闭 → 父 Epic 审计。实现时加入年龄权重，避免长期饿死低优先级任务。

## 验收
DAG-01 识别 A→B→C→A；DAG-02 检测跨仓库缺失依赖；DAG-03 前置 DONE 自动解锁；DAG-04 Pending CI 不阻塞独立任务；DAG-05 2 个调度实例不同时领取；DAG-06 父 Epic 的子任务不全部 DONE 时不得关闭。测试代码在远端 Actions 执行。