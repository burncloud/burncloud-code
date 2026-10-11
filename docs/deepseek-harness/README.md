# BurnCloud DeepSeek Harness 自主研发任务集合

本目录提供可直接复制给 DeepSeek Harness 的中文定时任务 Prompt。文件名保留英文与两位数字顺序，正文、任务名称、状态说明和调度格式使用中文；GitHub、API 字段、命令、路径等技术标识保留原样。

## 交付范围：DeepSeek Harness原生定时任务

当前工作只完善可以复制到DeepSeek Harness的中文Scheduled Task Prompt与执行说明，不开发新的研发调度系统，也不把n8n、外部数据库、Webhook、worker池或事件接口作为运行前提。

以 **9个DeepSeek Harness定时任务** 为可直接配置的基础模式。03代码开发增加每10分钟检查待开发/待修复工作；任务已运行或没有工作时结束，不重复创建会话。10/11保留记录职责Prompt，由08在当前定时执行中按规则顺序维护指定文档，不另建任务。

| 职责 | Harness触发方式 |
|---|---|
| 01巡检、02规划、07看板、08观察、09恢复 | 每30分钟 |
| 03代码开发、05CI守护、06合并门禁 | 每10分钟 |
| 04PR审查 | 每15分钟 |
| 10验收记录、11问题日志 | 08读取职责Prompt后按实际变化执行 |
| 外部CI事件 | 仅可选增强，不属于基础安装清单 |

保留旧文件名与路径；依据[定时任务清单](scheduled-task-overview.md)配置，不按目录逐文件盲目注册。可选事件协议不影响没有Webhook时的正常定时开发和CI修复路径。

## 使用方式

1. 阅读目标仓库 `AGENTS.md` 与[本地执行及CI冻结策略](../bam-local-execution-policy.md)。
2. 阅读[留言通信协议](communication-protocol.md)、[安装清单](installation.md)、[职责交接](agent-topology.md)与[定时任务清单](scheduled-task-overview.md)。
3. 将01至09完整Prompt逐项复制到DeepSeek Harness的定时任务配置，采用各文件中文名称、周期和无人值守模式。
4. 01在授权Issue追加分类留言，02在同Issue发布完整PLAN_READY，03下一轮读取并开发；创建PR后在Issue/PR双方追加关联。04/05在PR发布REVIEW_RESULT/CI_RESULT，03下一轮续接原分支，06实时重核全部门禁。
5. 08在自己的定时执行中读取10/11职责Prompt，分别核验后维护指定文档。没有变化时不创建文档PR。
6. 跨任务通信唯一使用Issue/PR普通讨论留言，采用统一标记和JSON协议；用实际GitHub API核验事实。原生运行/互斥能力须实际验证，不能证明唯一写入者时阻塞相关写入，不能用认领留言冒充锁。
7. 观察真实Harness触发与GitHub结果后才登记运行验收；当前文档修改不等于已经创建或启用这些任务。

任务派发文档只说明03选择工作和避免重入的规则。外部事件文件保留为可选参考，不自动注册、不作为定时任务运行依赖。

## 跨任务通信

Issue保存分类、完整计划、开发检查点和PR关联；PR保存代码更新、审查、CI、合并与恢复结果。每条消息使用 `<!-- burncloud-harness-message:v1 -->` 标记和JSON，绑定根Issue任务键、增量、计划版本、HEAD及前序comment ID。接收者按自己的周期读取，不需要即时唤醒。

计划依据变化、HEAD变化和最新CI重跑会使旧证据失效。确定性message_id避免同结果重复留言；结果引用输入即为回执，无变化不发心跳。结果未知先查询真实PR/push/重跑/合并和既有留言，恢复不重复创建任务。

留言是交接日志，不能证明CI通过、正式Review批准、合并授权或原子锁。Projects、test-cases.md和issue-log.md仅作展示/审计，不充当隐藏工作队列。无业务对象的全局异常可用已明确配置的授权管理Issue；未配置则只输出限制，不凭空创建Issue。

细节见[协议](communication-protocol.md)和[示例](communication-examples.md)。本目录提供Prompt规范，尚不代表通信能力已在真实Harness验收通过。

## 上下文与运行成本

每个任务只加载自己的Prompt、相关计划/文件和精简证据，不复制全部任务或历史。每轮先做轻量增量检查，默认最多深处理5个变化对象；其余仍保留在原Issue/PR留言中，下轮继续发现，不提前越过未处理对象；没有工作时尽快结束。

DeepSeek Harness定时触发可能已经进入模型执行。Prompt只能减少后续阅读和操作量，不能承诺“无变化时零模型调用”或依靠尚未实现的普通脚本节省费用。外部脚本优化不在本PR实现范围。

本基线默认同一03任务只有一个有效写入者，不假设常驻worker池、后台心跳或可恢复事件邮箱。原生运行/互斥能力须核验；跨任务交接仍只使用留言，不能改用事件邮箱。

## 仓库与授权范围

- 开发任务：`burncloud/burncloud`。
- 看板：<https://github.com/orgs/burncloud/projects/1>。
- 本集合及验收、问题文档：`burncloud/burncloud-code`。
- 授权身份基准为 GitHub login `rustburn`、稳定用户 ID `134291161`（通过 GitHub API 核验）；运行时必须重新核对，不接受同名显示名称或邮箱。
- 自动开发仅限目标仓库中由 GitHub API 核验的 `rustburn` 原始创建、当前开放、未重复认领且无关键依赖阻塞的 Issue。
- 自动合并资格同时要求 PR 作者与全部实际代码贡献者均经核验属于 `rustburn`，当前 HEAD 全部适用 CI 与必需检查成功，满足验收、Review、无冲突及分支保护，并且已有用户合并授权。
- 其他作者、混合贡献、身份或来源不明、门禁不满足的 PR 进入“等待人工审批”；不得修改或关闭非白名单 Issue。
- 用户针对任务的更严格限制优先。定时任务不扩大合并、关闭、CI 修改或破坏性操作的授权。
- `10/11` 仅可在已有文档维护授权内向文档仓库提交指定记录文件的 PR；此例外不改变开发仓库白名单，不自动授权合并文档 PR。

## 强制边界

禁止本地编译、类型检查、测试和 Clippy，包括间接脚本、Git hook、容器及其他语言测试运行器。源码与测试可以在本地编写，所有构建、测试和质量验证由 GitHub Actions 执行。

禁止修改、新建、删除、移动、重命名 Workflow、CI 门禁配置、触发条件、测试命令、路径过滤及分支保护。现有 CI 不覆盖必要测试时记录阻塞，不自行新增或放宽 CI。

PR 审查不修改代码、不合并；CI 守护只诊断与有限重跑；代码开发负责源码和测试修复；合并控制独占合并权限。其余角色只处理自己的元数据或指定文档。

同一对象同一职责只有一个有效写入者，03业务分支独占。须核验Harness实际运行状态和原生互斥能力；Prompt不能替代并发隔离。无法证明唯一写入权时阻塞相应写操作，不用普通评论伪装原子锁。

## 中文状态与机器标识

用户可见结果使用中文。与既有状态机、看板或接口交互时使用既有机器标识，不改写 API 枚举，也不另建状态机。巡检分类和审查结论是角色输出，不是需求追踪状态。

| 中文说明 | 既有机器标识 | 使用范围 |
|---|---|---|
| 未开始 | `NOT_STARTED` | 需求进度 |
| 开发进行中 | `IN_PROGRESS` | 需求进度 |
| 本地静态差异已检查 | `LOCAL_VERIFIED` | 仅既有状态要求时使用；不表示本地编译或测试 |
| 等待 CI | `WAIT_CI` | 需求进度 |
| CI 失败 | `CI_FAILED` | 需求进度 |
| 满足合并门禁 | `READY_TO_MERGE` | 需求进度；须全部证据完整 |
| 已合并 | `MERGED` | 已核实目标分支含代码 |
| 验收完成 | `VERIFIED` | 最终验收真实通过 |
| 任务阻塞 | `BLOCKED` | 依赖、授权或证据缺失 |
| CI 覆盖阻塞 | `BLOCKED_CI_COVERAGE` | 阻塞原因，非新增进度状态 |
| CI 不可用阻塞 | `BLOCKED_CI_UNAVAILABLE` | 阻塞原因，非新增进度状态 |
| 等待人工审批 | `Awaiting Human Approval` | 合并控制结论，非新增需求状态 |

## 执行记录与完成条件

记录仓库、对象链接、Agent、时间、HEAD SHA、真实动作、证据、状态、阻塞原因、下一负责者、message_id、comment ID及完整留言URL。CI 证据必须包含当前 HEAD 的 run/job/check ID、URL、运行尝试与实际执行范围。

创建 PR、绿色徽章、未执行用例、旧 HEAD 成功及等待都不能当作完成。合并后核实目标分支；父 Issue 必须全部验收才可进入已有授权的关闭流程。本集合没有自动关闭 Issue 的角色。

详细行为对照保留在[现有测试档案](../deepseek-harness-evaluation/README.md)，本目录引用这些档案，不另建重复测试目录。

## 文档入口

- [安装与共同配置](installation.md)
- [留言通信协议](communication-protocol.md)
- [留言示例与交接演练](communication-examples.md)
- [角色与交接](agent-topology.md)
- [调度总览与显式入口清单](scheduled-task-overview.md)
- [工作选择与互斥规则](agent-dispatcher.md)
- [可选CI事件参考](event-tasks/ci-failure-handler.md)
- [中文定时任务 Prompt](scheduled-tasks/)
- [验收记录](test-cases.md)
- [问题日志](issue-log.md)
