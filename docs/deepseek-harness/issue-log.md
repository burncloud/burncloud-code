# DeepSeek Harness 问题日志

## 目的与维护位置

记录异常事实、阻塞与处理证据，避免重复失败。本文件由08按 `11-issue-log-recorder.md` 职责维护，位置为文档仓库的 `docs/deepseek-harness/issue-log.md`；不另建 `testing/` 路径，也不是独立调度入口。

## 记录规则

按根task_key、scope_key及稳定根因/repair_chain_id去重，HEAD与run作为证据版本，不因为新SHA重复登记同根因。记录必须区分已核实事实、推测和未验证事项；未真实重跑、恢复、合并或验收时不得写成完成。

重大变化通过独立文档 PR 保存；同一问题无变化时不重复提交、评论或创建 Issue。日志和 PR 描述必须脱敏。维护记录不授权修改源码、CI、权限规则、其他 Agent 分支或自动关闭 Issue。

## 记录格式

- 问题编号与日期：
- Agent 与任务名称：
- 仓库、Issue/PR、当前 HEAD SHA：
- task_key、scope_key、plan_version、协议版本及规则commit SHA：
- 输入/结果message_id、comment ID、完整留言URL及前序链：
- run/attempt与实际运行/原生互斥证据：
- repair_chain_id、记录去重键与文档PR完整URL：
- 首次/最近发生时间与累计修复/恢复次数：
- 已核实问题与证据链接：
- 原因（已核实、推测或未验证）：
- 影响范围与阻塞原因：
- 实际处理动作与结果：
- 负责 Agent 与下一动作：
- 是否需要人工审批或规则调整：
- 状态（待处理、处理中、阻塞、已验证解决）：
- 解决证据与关联验收用例：

本文件是审计输出，不是03/09的工作输入队列。08更新文档PR后在原业务Issue/PR或已配置管理Issue追加DOCUMENTATION_RESULT，接收任务从留言恢复交接。留言不能确认写入时不宣称日志已交接。无业务对象且未配置管理Issue时只记录本轮限制，不自动创建故障Issue。

## 当前记录

尚未登记本集合的真实 Harness 运行故障。本次发现的文档缺失、中文格式不一致及旧维护路径由文档 PR 修正；不据此宣称 Harness 运行问题已解决。既有观察事实保留于[测试与对照档案](../deepseek-harness-evaluation/README.md)。
