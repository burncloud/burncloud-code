# DeepSeek Harness 问题日志

## 目的与维护位置

记录异常事实、阻塞与处理证据，避免重复失败。本文件由00在同一执行Agent中按 `11-issue-log-recorder.md` 职责维护，位置为文档仓库的 `docs/deepseek-harness/issue-log.md`；不另建 `testing/` 路径，也不是独立调度入口。

## 记录规则

按根task_key、scope_key及稳定根因/repair_chain_id去重，HEAD与run作为证据版本，不因为新SHA重复登记同根因。记录必须区分已核实事实、推测和未验证事项；未真实重跑、恢复、合并或验收时不得写成完成。

重大变化通过独立文档 PR 保存；同一问题无变化时不重复提交、评论或创建 Issue。日志和 PR 描述必须脱敏。维护记录不授权修改源码、CI、权限规则、其他 Agent 分支或自动关闭 Issue。

## 记录格式

- 问题编号与日期：
- 唯一执行Agent、任务名称、实际运行ID及当前阶段：
- 仓库、Issue/PR、当前 HEAD SHA：
- task_key、scope_key、plan_version、协议版本及规则commit SHA：
- 聚合检查点message_id、comment ID、完整留言URL及前序链：
- run/attempt与实际运行/原生互斥证据：
- repair_chain_id、记录去重键与文档PR完整URL：
- 首次/最近发生时间与累计修复/恢复次数：
- 已核实问题与证据链接：
- 原因（已核实、推测或未验证）：
- 影响范围与阻塞原因：
- 实际处理动作与结果：
- 同一执行Agent的负责阶段与下一动作：
- 是否需要人工审批或规则调整：
- 状态（待处理、处理中、阻塞、已验证解决）：
- 解决证据与关联验收用例：

本文件是审计输出，不是独立开发/恢复Agent的工作输入队列。00更新聚合文档PR后在原业务Issue/PR或已配置管理Issue追加TASK_CHECKPOINT，后续轮次从留言恢复。检查点不能确认写入时区分文档已提交与结果未保存。无业务对象且未配置管理Issue时只记录本轮限制，不自动创建故障Issue。

单Agent观察还应记录重复对象/规则读取、重复Diff或日志分析、重复会话/Agent及内部交接消息的事实证据。token总量、缓存命中、输出和实际费用分别记录；推测的成本原因不标成已核实。

## 当前记录

尚未登记本集合的真实 Harness 运行故障。单Agent合并解决文档中的多入口与逐阶段交接设计；未实际迁移或测量，不能声称运行故障已解决或token已下降。既有观察事实保留于[测试与对照档案](../deepseek-harness-evaluation/README.md)。
