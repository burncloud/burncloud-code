# DeepSeek Harness 调度总览

本表是安装任务的显式清单，与各Prompt末尾的“执行配置”一致。目录或文件存在不代表任务已启用，不允许对 `scheduled-tasks/*.md` 自动逐文件注册Cron。

## 8个定时入口

| 文件 | 中文任务名称 | 周期 | 固定触发偏移 | 本轮工作范围 |
|---|---|---|---|---|
| [01-issue-scanner.md](scheduled-tasks/01-issue-scanner.md) | BurnCloud Issue 自动巡检 Agent | 每30分钟 | 周期起点+0分钟 | Issue增量发现、白名单和依赖分类 |
| [02-task-planner.md](scheduled-tasks/02-task-planner.md) | BurnCloud 任务规划 Agent | 每30分钟 | 周期起点+2分钟 | 只规划新候选项或失效计划 |
| [04-pr-reviewer.md](scheduled-tasks/04-pr-reviewer.md) | BurnCloud PR 审查 Agent | 每15分钟 | 周期起点+3分钟 | 变化PR的需求、Diff和风险 |
| [05-ci-guardian.md](scheduled-tasks/05-ci-guardian.md) | BurnCloud CI 守护 Agent | 每10分钟 | 周期起点+1分钟 | 当前HEAD CI对账、故障补偿和有限重跑 |
| [06-merge-controller.md](scheduled-tasks/06-merge-controller.md) | BurnCloud PR 合并控制 Agent | 每10分钟 | 周期起点+4分钟 | 已有完整证据PR的最终门禁 |
| [07-project-manager.md](scheduled-tasks/07-project-manager.md) | BurnCloud Projects 管理 Agent | 每30分钟 | 周期起点+6分钟 | 变化项目项的既有自动化字段 |
| [08-harness-observer.md](scheduled-tasks/08-harness-observer.md) | BurnCloud Harness 运行观察 Agent | 每30分钟 | 周期起点+8分钟 | 健康、成本、违规与新记录 |
| [09-recovery-manager.md](scheduled-tasks/09-recovery-manager.md) | BurnCloud 故障恢复 Agent | 每30分钟 | 周期起点+12分钟 | 已确认停止或异常任务的安全恢复 |

偏移是建议配置，用于分散同一时刻唤醒；按调度器统一的时间锚点计算，不是前一任务完成时间的偏移。时区显式使用 `Asia/Shanghai`，接收和审计时间同时保存UTC。不同周期互不依赖，无须等待前一轮全部完成；任务尚在运行时租约控制重入。

## 2个事件入口

| 文件 | 中文任务名称 | 触发方式 | 执行行为 |
|---|---|---|---|
| [03-coding-agent.md](scheduled-tasks/03-coding-agent.md) | BurnCloud 代码开发 Agent | 派发器的新开发/审查修复/CI修复事件 | 续接原会话，持续推进可执行工作；等待时保存检查点并释放槽 |
| [ci-failure-handler.md](event-tasks/ci-failure-handler.md) | BurnCloud CI 失败处理 Agent | 校验后的GitHub Webhook；05定时对账补偿同一入口 | 验证、去重和路由；不独立启动第二条修复链 |

代码开发事件启动后可以持续运行，但不设置每30分钟重启。CI失败入口处理的是外部Webhook，不新增GitHub Actions Workflow；没有事件接收能力时05维持远端验证闭环，并记录即时入口未启用。

## 控制能力与记录职责

| 文件 | 调用者 | 调用时机 | 独立注册 |
|---|---|---|---|
| [任务派发协议](agent-dispatcher.md) | 02、事件入口、09、执行槽释放回调 | 有效计划、修复消息、安全恢复或空闲槽 | 否，调度器内部控制能力 |
| [10-test-case-recorder.md](scheduled-tasks/10-test-case-recorder.md) | 总调度聚合06/08的新证据 | 新用例或实际验收结果变化 | 否，记录职责 |
| [11-issue-log-recorder.md](scheduled-tasks/11-issue-log-recorder.md) | 总调度聚合08/09/CI异常 | 新异常或处理结果变化 | 否，记录职责 |

## 周期选择理由

Issue与规划每30分钟足以补偿新需求变化，避免每轮全仓分析。PR审查每15分钟缩短修复反馈等待，但只对实质变化调用模型。CI和合并每10分钟降低结果传递延迟；失败Webhook走即时入口，CI轮询仍负责漏收补偿。看板、观察与恢复每30分钟提供常规对账，现场worker通过心跳和检查点提供更细粒度安全信号。

记录没有新事实就无需运行；按变化回调比每日或每30分钟空跑更合理。安全检查与运行报告沿用角色已有职责，不单独增设定时任务。

## 调度与成本规则

- 所有入口中文命名、无人值守，不等待普通人工确认。
- 先读取增量游标和持久化状态，再判断有无变化；无变化时不调用模型、不评论、不重复写入。
- 轮询默认最多深处理5个变化对象，余项保存到持久队列；分页和对账游标只能在相应对象持久化后推进，防止漏项。
- 首次启动、检查点丢失或无法证明增量完整时做分页全量对账；不把未扫描范围当成无变化。
- 缓存用于减少重复分析；任何写操作，尤其合并，必须重新读取GitHub真实状态。
- 默认2个开发执行槽；等待CI不占模型槽。修复预算、原子租约和文件归属见派发协议。
- 启动任务与新Webhook接收必须使用既有能力和授权；缺持久化、原子锁或最小权限时阻塞相应写操作。
- 不承诺“10分钟内必定合并”或“30分钟内必定恢复”；这些是触发周期，不是完成时限。
