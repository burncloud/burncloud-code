# DeepSeek Harness 单Agent注册清单

## 唯一注册入口

| 文件 | 中文任务名称 | 周期 | 执行者 |
|---|---|---|---|
| [00-autonomous-agent.md](scheduled-tasks/00-autonomous-agent.md) | BurnCloud 自主研发 Agent | 每30分钟 | 同一个Agent，按实际进度执行必要阶段 |

基础配置只有一个定时任务。01至11均为00按需读取的阶段规范，不能按目录逐文件注册。周期是触发间隔，不是超时或强制重启依据；上轮仍运行时退出，不新建第二Agent。

## 按需阶段

| 编号 | 阶段规范 | 使用条件 |
|---|---|---|
| [01](scheduled-tasks/01-issue-scanner.md) | Issue发现 | 需要新候选或候选事实变化 |
| [02](scheduled-tasks/02-task-planner.md) | 规划 | 缺少有效计划或需求依据变化 |
| [03](scheduled-tasks/03-coding-agent.md) | 开发 | 有有效计划或当前HEAD修复事项 |
| [04](scheduled-tasks/04-pr-reviewer.md) | 自检/正式Review查询 | Diff、需求、目标基线或Review变化 |
| [05](scheduled-tasks/05-ci-guardian.md) | CI诊断 | 当前HEAD远端检查需要更新 |
| [06](scheduled-tasks/06-merge-controller.md) | 合并门禁 | 可能满足条件，且有原有合并授权 |
| [07](scheduled-tasks/07-project-manager.md) | 看板同步 | 授权自动化字段实际需要改变 |
| [08](scheduled-tasks/08-harness-observer.md) | 观察 | 本轮出现运行/证据异常 |
| [09](scheduled-tasks/09-recovery-manager.md) | 恢复 | 实际停止、安全移交或未知动作对账 |
| [10](scheduled-tasks/10-test-case-recorder.md) | 验收记录 | 新真实验收或用例设计变化 |
| [11](scheduled-tasks/11-issue-log-recorder.md) | 问题记录 | 新异常或既有问题实际状态变化 |

所有阶段均无独立周期、没有独立Agent。阶段之间直接复用当前上下文，不等待留言交接或下一触发；每轮无需执行全部阶段。外部CI事件参考不注册，等待CI时保存业务检查点并结束，下轮同一入口核验变化。

## 迁移与工作量

先按[安装清单](installation.md)停用并核验旧01至09任务实际停止，保留原PR/分支和历史留言，再启用00。该PR只改变文档，不操作用户Harness配置。

复用同轮规则、需求、Diff、CI和预算；关键事实变化或外部写操作前重核。变化展示与记录聚合，省去多Agent重复读取/分析和逐阶段回执。真实token/费用须测量，不仅比较触发次数。默认只深处理一个业务增量，其他候选不丢失、下轮继续。
