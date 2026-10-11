# Scheduled Tasks

## BurnCloud Autonomous R&D Manager

建议由调度器周期执行。

## Task List

| Task | Frequency | Responsibility |
|---|---|---|
| Issue Scanner | 30 min | 发现任务 |
| Task Planner | 30 min | 分解需求 |
| Project Manager | 30 min | 同步看板 |
| PR Reviewer | 15 min | 审查 PR |
| CI Guardian | 10 min | 监控 CI |
| Merge Controller | 10 min | 合并门禁 |
| Harness Observer | 30 min | 记录 Agent 问题 |
| Recovery Manager | 30 min | 故障恢复 |

## 无人值守规则

- 不等待普通技术决策。
- 不重复创建任务。
- 不依赖人工逐轮确认。
- 无任务时正常结束等待下一次调度。

## 持久化

所有执行结果写入 GitHub 可追踪位置。

Harness 行为问题记录：

- testing/test-cases.md
- testing/issue-log.md
