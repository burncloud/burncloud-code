# DeepSeek Harness 定时任务总览

本表与每个任务末尾的“定时执行”一致。任务文件是可复制 Prompt；只有在 Harness 中实际注册、持久保存并观察到触发记录后，才能报告定时任务已启用。本次文档修改不自动安装或启动任务。

| Prompt 文件 | Agent | 执行周期 |
|---|---|---|
| [01-issue-scanner.md](scheduled-tasks/01-issue-scanner.md) | Issue 自动巡检 | 每30分钟一次 |
| [02-task-planner.md](scheduled-tasks/02-task-planner.md) | 任务规划 | 每30分钟一次 |
| [03-coding-agent.md](scheduled-tasks/03-coding-agent.md) | 代码开发 | 每30分钟一次 |
| [04-pr-reviewer.md](scheduled-tasks/04-pr-reviewer.md) | PR 审查 | 每30分钟一次 |
| [05-ci-guardian.md](scheduled-tasks/05-ci-guardian.md) | CI 守护 | 每30分钟一次 |
| [06-merge-controller.md](scheduled-tasks/06-merge-controller.md) | PR 合并控制 | 每30分钟一次 |
| [07-project-manager.md](scheduled-tasks/07-project-manager.md) | Projects 管理 | 每30分钟一次 |
| [08-harness-observer.md](scheduled-tasks/08-harness-observer.md) | Harness 运行观察 | 每30分钟一次 |
| [09-recovery-manager.md](scheduled-tasks/09-recovery-manager.md) | 故障恢复 | 每30分钟一次 |
| [10-test-case-recorder.md](scheduled-tasks/10-test-case-recorder.md) | 验收记录 | 每30分钟一次 |
| [11-issue-log-recorder.md](scheduled-tasks/11-issue-log-recorder.md) | 问题日志 | 每30分钟一次 |

## 调度规则

全部使用中文任务名称、无人值守模式。每30分钟触发一次，不表示必须同时写入或同一轮完成全部阶段。

调度器负责触发、持久化去重和租约；总调度负责交接与文件归属。上轮尚未结束时跳过相同对象；空闲轮正常结束，不无限轮询、不重复创建任务。

各角色只做自己的动作。等待 CI 时保留对象归属、释放运行槽，下一轮重新读取真实状态。数据缺失、权限不足或基础设施阻塞时记录并继续无依赖冲突任务。

定时 Prompt 不等于已实现的锁、权限隔离或故障恢复机制。Harness 不提供必要能力时阻塞相应写操作，不能用评论中的“已认领”替代原子锁。
