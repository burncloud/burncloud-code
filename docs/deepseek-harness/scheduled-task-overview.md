# DeepSeek Harness原生定时任务清单

当前交付是DeepSeek Harness可直接复制的中文定时任务Prompt。基础模式共9个定时任务，不依赖n8n、Webhook、外部数据库或新建派发服务。

| Prompt | 中文任务名称 | 周期 | 职责 |
|---|---|---|---|
| [01](scheduled-tasks/01-issue-scanner.md) | BurnCloud Issue 自动巡检 Agent | 每30分钟 | 发现授权候选任务 |
| [02](scheduled-tasks/02-task-planner.md) | BurnCloud 任务规划 Agent | 每30分钟 | 保存实施计划和验收条件 |
| [03](scheduled-tasks/03-coding-agent.md) | BurnCloud 代码开发 Agent | 每10分钟 | 主动读取计划/修复项，续接独占分支 |
| [04](scheduled-tasks/04-pr-reviewer.md) | BurnCloud PR 审查 Agent | 每15分钟 | 差异、风险与需求审查 |
| [05](scheduled-tasks/05-ci-guardian.md) | BurnCloud CI 守护 Agent | 每10分钟 | 主动查询CI、诊断并保存修复项 |
| [06](scheduled-tasks/06-merge-controller.md) | BurnCloud PR 合并控制 Agent | 每10分钟 | 重核当前HEAD全部门禁 |
| [07](scheduled-tasks/07-project-manager.md) | BurnCloud Projects 管理 Agent | 每30分钟 | 同步既有自动化字段 |
| [08](scheduled-tasks/08-harness-observer.md) | BurnCloud Harness 运行观察 Agent | 每30分钟 | 观察任务并按10/11职责维护记录 |
| [09](scheduled-tasks/09-recovery-manager.md) | BurnCloud 故障恢复 Agent | 每30分钟 | 核验实际停止与安全恢复条件 |

周期是触发间隔，不是完成时限，也不是强制重启时间。上轮仍运行时不创建第二个写入者。任务时区显式采用Asia/Shanghai，审计时间同时记录UTC。若Harness支持起始时间偏移，可分散触发；不把偏移能力作为必备功能。

## 开发与失败处理的纯定时路径

02保存有效计划 → 03下一轮读取并核验归属 → 修改/提交/推送 → 04/05各自检查 → 有问题时保存修复记录 → 03下一轮恢复同一分支 → 当前HEAD证据完整后06重核合并。

CI等待时03保存检查点并结束本轮；下次定时运行再次查询真实状态。05直接处理轮询发现的CI失败，不等待Webhook，也不需要独立事件消费者。无工作时各任务尽快结束，不加载整个仓库历史。

## 记录职责与可选参考

| 文件 | 使用方式 | 基础模式是否独立注册 |
|---|---|---|
| [10验收记录](scheduled-tasks/10-test-case-recorder.md) | 08读取此职责并核验真实证据后更新指定文件 | 否 |
| [11问题日志](scheduled-tasks/11-issue-log-recorder.md) | 08读取此职责并按新异常更新指定文件 | 否 |
| [工作选择与互斥](agent-dispatcher.md) | 03在定时任务内部选择工作及核验唯一写入权 | 否 |
| [外部CI事件参考](event-tasks/ci-failure-handler.md) | 仅未来已授权且能力已核验时加速05同一处理流程 | 否，可选增强 |

## 实施边界

- 使用DeepSeek Harness实际提供的定时配置、GitHub访问、运行状态与持久化能力；没有能力时标记具体限制，不虚构新服务或事件接口。
- 写操作前重新读取GitHub状态并核验互斥。首次启动或检查点丢失时完整分页对账，不能丢弃批次外对象。
- 角色职责与最小权限不变：03开发，04审查，05CI诊断，06合并；08仅可额外按10/11边界维护授权记录文件。
- 定时启动本身可能有模型成本；无变化时缩短执行，不承诺零模型调用。
- 本PR只完成Prompt文档，不在Harness中实际注册任务，不修改CI，不配置Webhook或部署其他系统。
