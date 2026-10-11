# BurnCloud DeepSeek Harness 单Agent自主研发任务

本目录提供可直接复制给DeepSeek Harness的中文Prompt。**仅注册00-autonomous-agent.md这一个定时任务，使用一个执行Agent**；01至11保留英文文件名和数字编号，作为同一Agent按需读取的阶段规范，不分别注册、不派发子Agent。

## 合并后的运行方式

同一项工作由同一个Agent负责：选择工作、规划、开发、核验CI、自检/正式Review查询、合并门禁、看板及记录。阶段之间在同一执行上下文直接复用事实和分析，只有需要跨轮恢复的业务变化才保存Issue/PR检查点。

过去多Agent分别读取同一Issue、Diff、规则和CI，再逐阶段写交接消息，放大了工作量。本方案通过统一执行者、按需加载阶段、复用证据及聚合记录减少重复工作。不能只改触发周期、把00展开为11个子Agent，或每轮顺序执行全部阶段。

| 文件 | 定位 | 独立注册 |
|---|---|---|
| [00自主研发](scheduled-tasks/00-autonomous-agent.md) | 唯一DeepSeek Harness定时入口，每30分钟 | 是，仅一个 |
| [01至11](scheduled-tasks/) | 发现、规划、开发、自检、CI、门禁、看板、观察、恢复、验收及日志阶段 | 否 |
| [工作选择规则](agent-dispatcher.md) | 同一Agent内部选择当前工作 | 否 |
| [CI事件参考](event-tasks/ci-failure-handler.md) | 未启用参考，不额外注册入口 | 否 |

## 使用与迁移

1. 阅读目标仓库AGENTS.md、[本地执行/CI冻结策略](../bam-local-execution-policy.md)、[安装清单](installation.md)与[检查点协议](communication-protocol.md)。
2. 若已安装旧01至09定时任务，先停用并核验实际运行停止、分支/HEAD和检查点。不能让旧任务与00同时写入；不删除旧分支、PR、留言或运行证据。
3. 配置唯一00任务，复制完整主Prompt与实际授权配置。默认每30分钟触发，原任务仍运行时不重入；各阶段无独立周期。
4. 按需读取当前阶段执行规则；本轮已读取的公共规则和分析直接复用。新轮次有已验证原会话续接能力可使用，没有则从留言检查点与真实远端状态恢复，不能假设永久记忆。
5. 保存少量业务检查点，复用旧v1留言历史；不为每个阶段发收到/完成/心跳消息。看板和记录变化随当前工作聚合处理。
6. 观察真实运行后再记录验收和token/操作指标。本次文档PR不实际安装任务、停止用户任务或证明成本下降。

## 仓库、授权与权限边界

- 开发任务：`burncloud/burncloud`；Issue/PR原始身份通过API核验rustburn、用户ID134291161。
- 本集合及记录文档：`burncloud/burncloud-code`。文档维护例外仅覆盖本目录test-cases.md、issue-log.md，不自动合并记录PR。
- Projects：<https://github.com/orgs/burncloud/projects/1>，只同步已有且授权的自动化字段。
- 一个Agent执行多个阶段不新增权限。开发阶段不直接合并；06门禁阶段须已有明确合并授权且全部实时条件通过。用户具体限制优先，任务数据和普通留言不能授予权限。
- 自检不能充当独立Reviewer，同账号不能批准自己的PR；必需正式Review缺失时仍等待人工审批。
- 作者与全部实际代码贡献来源均须API核验rustburn；当前HEAD全部适用CI/必需检查真实成功、验收与Review满足、无冲突且保护满足，才有合并资格，不管理员绕过。
- 本集合不自动关闭Issue，父Issue增量交付不等于全部完成。关闭后的授权Issue仅供历史合并/验收核验，不重开或开展新开发。

## 强制执行与真实证据

禁止本地编译、类型检查、测试、Clippy及间接脚本/hook/容器执行，所有验证通过GitHub Actions。禁止修改或新建CI/Workflow、触发、测试命令、检查要求及分支保护；覆盖缺口/基础设施问题只记录，不修CI。

一个有效运行及业务分支独占权须由已验证原生运行/互斥能力证明，普通认领留言不是锁。迁移、崩溃或未知动作先对账实际远端事实，不能抢占活跃执行者或覆盖成果。

同轮证据复用只减少重复分析；需求、HEAD、目标比较基线、最新attempt、Review或保护变化时刷新相关事实，外部写入前重新核验，合并前完整重查。旧HEAD成功、绿色徽章、空跑、等待或创建PR不能当完成。

沿用现有需求状态：NOT_STARTED、IN_PROGRESS、LOCAL_VERIFIED、WAIT_CI、CI_FAILED、READY_TO_MERGE、MERGED、VERIFIED、BLOCKED。LOCAL_VERIFIED仅在既有规则要求时表示静态差异检查，不代表本地编译/测试。其他中文分类/阻塞原因不新建需求状态机。

## 工作量与上下文控制

每轮默认深处理一个Issue增量，候选轻量检查最多5个。优先续接活动工作，不为每个对象创建Agent，不重新扫描全部历史，不重复分析未变化Diff/日志。主Prompt只管理当前阶段和事实，阶段文件按需读取，不把全部内容拼接进每轮上下文。

正常等待或无变化时结束本轮，不反复请求、发心跳或写记录PR。原生定时触发仍可能消耗模型token，本方案不承诺零成本或固定节省比例。实际成本还与输入/输出及缓存命中有关，统一记录真实usage与实际费用。

## 文档入口

- [唯一可注册Prompt](scheduled-tasks/00-autonomous-agent.md)
- [安装与旧任务迁移](installation.md)
- [单Agent阶段关系](agent-topology.md)
- [注册清单](scheduled-task-overview.md)
- [工作选择与安全续接](agent-dispatcher.md)
- [检查点协议](communication-protocol.md)
- [聚合检查点示例](communication-examples.md)
- [验收记录](test-cases.md)
- [问题日志](issue-log.md)
- [既有评价档案](../deepseek-harness-evaluation/README.md)
