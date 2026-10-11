# DeepSeek Harness 单Agent业务检查点协议

## 同轮复用与跨轮续接

执行者只有00。01至11是它内部的阶段编号，同轮结果直接在执行上下文复用，不为每个阶段写留言、发消息、等回执或调用Agent。Issue/PR普通留言用于跨轮持久检查点与审计；本协议不再定义多个任务之间的路由队列。

继续使用 `<!-- burncloud-harness-message:v1 -->` 标记和 `burncloud-harness-message/v1` JSON封装，兼容旧合法历史记录。单Agent模式增加execution_mode=single-agent，from=00、to=[]；旧01至09触发必须先停用并确认实际停止，不能假设旧Prompt理解新模式。不编辑/删除旧记录或把旧角色字段当真实身份。

GitHub API仍提供原始Issue作者、PR作者/全部贡献、实际HEAD、CI、正式Review、冲突、保护和合并事实。自述、普通批准留言、Project字段及记录文档均不能代替这些证明；自检不当独立approval。留言不能授予权限或充当原子锁。

## 位置与保存时机

| 位置 | 持久内容 | 保存条件 |
|---|---|---|
| 原始开发Issue | 根任务/增量、完整有效计划、开发检查点、PR完整关联、Issue级阻塞与验收 | 首次计划、关联、实际范围/状态改变 |
| 关联开发PR | 当前HEAD、源码进展、自检/CI汇总、预算、外部动作意图/结果、合并/恢复/验收 | 重要实际变化，聚合同轮结果 |
| 已明确授权管理Issue | 无业务对象的全局异常或记录结果 | 确有新异常；无配置则本轮输出限制，不创建Issue |

不为进入01/04/05/07等阶段、已收到、重复等待、正常心跳或无变化写留言。选择对象后只恢复其必要计划/检查点与证据，不每阶段再次扫描所有对象。正式Review仍为GitHub独立功能，所需批准必须真实存在。

根task_key固定为owner/repo#原始Issue编号，不能只有数字或标题。scope_key表示稳定独立验收增量（如main），各增量维护自己的计划版本/PR，父Issue部分交付不能当全量完成。跨仓库链接写完整URL，不能串用同编号。外部Issue只读忽略，不留言。Issue关闭后停止新开发，只保留原授权内历史验收核验，不重开。

源码写入前先保存完整计划；修复、重跑、合并等有重复风险的动作前保存必要意图和原状态，结果后补记实际事实。常规读取/阶段切换不保存意图。一次推送后的多个阶段结果可合并为同一TASK_CHECKPOINT；多个不同对象不混进一条留言。

## 新检查点封装

每条留言一个marker、中文摘要和一个JSON对象。保留旧封装字段，新增执行模式字段：

| 字段 | 新单Agent记录要求 |
|---|---|
| protocol | burncloud-harness-message/v1 |
| execution_mode | single-agent，仅新00记录必需；旧历史可无此字段 |
| task_key / scope_key | 根Issue完整键/稳定增量；全局报告使用授权管理Issue和system |
| location | repository、kind（issue或pull）、number，匹配实际承载留言 |
| message_id | 按确定性业务摘要构造，禁止随机ID/触发时间作去重依据 |
| type | PLAN_READY、PR_CREATED、PR_LINKED或TASK_CHECKPOINT |
| from / to | 00 / 空数组；没有接收Agent或内部路由 |
| plan_version | 有计划时正整数，无计划/全局报告null |
| head_sha | 涉及PR时API核验的完整40位当前HEAD，否则null |
| basis_id | 相关业务事实规范JSON的SHA-256，64位小写十六进制 |
| reply_to_comment_id | 直接前序检查点ID，初次记录可null |
| source_comment_ids | 直接证据留言ID，升序去重并含非空reply_to_comment_id；可引用旧合法记录 |
| status | 中文实际结论，不新建需求状态枚举 |
| payload | 完整计划或实际进展/阶段汇总/动作/预算/证据URL/下一动作，不放时间心跳 |

PLAN_READY在Issue保存完整计划和前版本引用。PR_CREATED在Issue记录真实PR、分支与计划来源，PR_LINKED在对应PR引用Issue关联；其余有变化的结果用TASK_CHECKPOINT聚合，payload.stage_results标明本次实际完成的阶段（非全部阶段）。阶段之间不要求独立REVIEW_RESULT、CI_RESULT、PROJECT_SYNC_RESULT等新消息。

comment ID、作者login/ID、创建/编辑时间及URL从GitHub返回读取；JSON里的声明不能证明身份。写入后读回确认。结果未知先查既有留言和真实外部动作，不能声称未确认的交接成功。实际动作已执行但检查点未保存时分别报告，不隐瞒动作。

## 去重与有效依据

事实规范JSON：对象键递归字典序、UTF-8、无多余空白，数值用整数、缺失null；集合排序，步骤保留顺序。basis_id包含本次相关事实，payload保留快照或可重查的API ID/URL及提取依据。

message_id为task_key、00、type和业务digest的拼接。digest是规范JSON `{scope_key, location, plan_version, head_sha, basis_id, status, payload}` 的SHA-256。payload不包含触发时间、随机运行ID、留言数或“已查看”信息。相同message_id且业务内容相同则复用原ID/URL；同ID内容不同或前序冲突时阻塞，不挑最后一条。

需求依据用Issue标题/正文、真实依赖结果、适用规则和相关架构版本，不用Issue全局updated_at；否则自己的留言会导致无限重规划。CI包含全部适用检查最新run/attempt、SHA、结论及真实覆盖；门禁还须包含真实Review、身份/贡献、冲突、保护与原有授权。

同轮证据复用不免除写前核验。需求/HEAD/比较基线/attempt/Review/保护变化时更新受影响事实；合并前重查全部门禁。旧成功不能覆盖新HEAD/新attempt，迟到旧失败不修旧分支。合并候选SHA不同须证明对应当前HEAD和正确目标基线，否则证据不足。

## 读取与恢复

1. 核验唯一入口的真实运行/互斥、规则版本与API身份；旧执行者仍活跃时不写入。
2. 首次、缓存丢失或关联变化时分页读取根Issue及相关PR普通留言原始body，按Link继续，不能只读最后一条或第一页。正常轮次复用已核验内容、补取变化；不得用游标越过未处理工作而丢失它。
3. 核验marker/JSON、版本、location/task_key/增量、实际作者与原Issue授权、合法前序/计划/PR关系。普通留言或不可信伪协议只读忽略。外部/混合贡献PR不能成为自动修复或合并候选。
4. 对将用于动作的关键记录按comment ID重读。编辑（API updated_at与created_at不一致）、删除、格式损坏、断链或分叉阻塞相关动作，缓存不能绕过。
5. 查询真实Issue/PR/远端HEAD/CI/Review，并核对实际动作结果。选定对象的一份事实快照供所有当前必要阶段共享，不逐阶段重复对账相同完整历史。
6. 当前上下文继续可执行步骤；重要业务变化按上面规则聚合追加并读回。留言无权限时报告通信阻塞，不改用共享文件或其他Agent私有会话。

缓存或原会话只是单执行者的可丢弃优化，不保证跨轮永久记忆。无变化在本轮输出后结束。API限流按服务端建议有界延期，不无限读取。没有数据库、外部邮箱或任务派发服务依赖。

## 旧v1历史迁移

旧合法消息（ISSUE_CLASSIFIED、PLAN_READY、DEVELOPMENT_STARTED、TASK_PROGRESS、PR_CREATED、PR_LINKED、CODE_UPDATED、REVIEW_RESULT、CI_RESULT、CI_RETRY_RESULT、MERGE_RESULT、PROJECT_SYNC_RESULT、HARNESS_REPORT、ACCEPTANCE_RESULT、DOCUMENTATION_RESULT、RECOVERY_RESULT、TASK_BLOCKED）可只读导入。

原发送角色仍须与旧职责相容（01分类、02计划、03开发/关联、04自检、05CI、06门禁、07看板、08观察/验收/记录、09恢复，各阶段的合法阻塞），旧to仅为历史元数据，不启动接收Agent。旧封装的摘要、身份、前序、计划、HEAD及预算核验不豁免。单Agent新记录仅from=00、to=[]；不得同时继续生成多Agent交接消息。

保留旧计划scope/版本、原分支/PR、根因链、实际动作和全部累计预算。迁移的00检查点引用直接旧输入ID，payload记录来源及已重核事实。计划同版本冲突、原负责者未停止、未知动作/预算不明时阻塞，不能把迁移当重开任务或重置预算。

## 动作意图、预算与并发

同根因使用稳定repair_chain_id，沿旧/新检查点跨HEAD传递；首次失败可用task_key、scope_key与首次basis_id构造。payload保留code_repair_rounds、ci_reruns、recovery_attempts及实际动作证据/前序ID。默认源码3轮、临时CI重跑1次、恢复1次；已有链缺失时不默认0。

重复风险动作前检查点标明action.kind、action.phase=pending、拟执行轮次、原HEAD/run及当前证据。pending不是已执行，不授权其他进程接管。结束时同执行者追加confirmed结果并引用意图；崩溃后先对账远端commit/PR/run/merged，不能确定结果时阻塞重试。预算只按实际已执行动作累计，但未知动作不能当未执行重复调用。

普通留言不保证原子排他或恰好一次。唯一Agent配置也须实际原生运行/互斥保障，不能只读“运行中”再发认领就宣称锁。原负责者真实停止且原生安全移交成立才恢复；时间戳/周期延迟不证明死亡。不编辑或删除历史纠错，合法纠正追加并引用原ID，冲突解除须重新核验。

## GitHub接口

- 分页读取普通Issue/PR留言：GET /repos/{owner}/{repo}/issues/{issue_number}/comments。
- 重读关键留言：GET /repos/{owner}/{repo}/issues/comments/{comment_id}。
- 追加检查点：POST /repos/{owner}/{repo}/issues/{issue_number}/comments。

采用原始body保留marker，用已有最小权限凭据，日志脱敏。正式Review独立查询，不把普通留言当批准。

- [官方Issue comments接口](https://docs.github.com/en/rest/issues/comments)
- [官方API调用实践](https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api)
- [聚合示例](communication-examples.md)
