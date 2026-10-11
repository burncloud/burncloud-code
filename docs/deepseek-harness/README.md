# BurnCloud DeepSeek Harness 自主研发任务集合

本目录提供可直接复制给 DeepSeek Harness 的中文定时任务 Prompt。文件名保留英文与两位数字顺序，正文、任务名称、状态说明和调度格式使用中文；GitHub、API 字段、命令、路径等技术标识保留原样。

## 使用方式

1. 阅读目标仓库 `AGENTS.md` 与[本地执行及 CI 冻结策略](../bam-local-execution-policy.md)。
2. 阅读[角色与交接](agent-topology.md)以及[定时任务总览](scheduled-task-overview.md)。
3. 将 `scheduled-tasks/` 中每个文件的完整内容复制为独立定时任务 Prompt。
4. 采用文件末尾的中文任务名称、每30分钟一次和无人值守模式。
5. 配置已有授权的 GitHub 连接、持久化记录、原子租约与各角色最小权限。
6. 观察真实调度、GitHub 和 CI 记录后，将结果写入[验收记录](test-cases.md)与[问题日志](issue-log.md)。

本次交付是 Prompt 文档。文档存在不代表 Harness 已安装任务、已实现权限隔离或已通过运行验收。必要能力无法核验时只读并记录阻塞。

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

同一对象只有一个有效写入者。调度器必须提供持久化去重、原子锁与租约；Prompt 本身不能保证并发隔离。没有这些能力时相应写操作阻塞，不以普通评论当作原子锁。

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

记录仓库、对象链接、Agent、时间、HEAD SHA、真实动作、证据、状态、阻塞原因、下一负责者、去重键和持久化位置。CI 证据必须包含当前 HEAD 的 run/job/check ID、URL、运行尝试与实际执行范围。

创建 PR、绿色徽章、未执行用例、旧 HEAD 成功及等待都不能当作完成。合并后核实目标分支；父 Issue 必须全部验收才可进入已有授权的关闭流程。本集合没有自动关闭 Issue 的角色。

详细行为对照保留在[现有测试档案](../deepseek-harness-evaluation/README.md)，本目录引用这些档案，不另建重复测试目录。

## 文档入口

- [角色与交接](agent-topology.md)
- [定时任务总览](scheduled-task-overview.md)
- [中文定时任务 Prompt](scheduled-tasks/)
- [验收记录](test-cases.md)
- [问题日志](issue-log.md)
