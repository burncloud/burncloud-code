# DeepSeek Harness 验收记录

## 范围与证据

本文件记录9个原生定时任务及其留言通信的验收设计，由08按10职责维护。不是工作队列或独立任务。既有[行为用例](../deepseek-harness-evaluation/test-cases.md)与[运行档案](../deepseek-harness-evaluation/runs/)保留，以链接引用。

以下全部为“未执行”；文档审查和本PR的远端CI不代表真实Harness触发或留言通信验收通过。旧编号保留，曾涉及私有邮箱/worker池的未执行设计现按原生留言方案明确，不要求新增系统。

## 原生任务与权限验收

| 编号 | 场景 | 预期结果 | 状态 |
|---|---|---|---|
| ST-01 | 原生任务安装与周期 | 01至09按30/15/10分钟配置；10/11仅08同轮执行，真实触发有证据 | 未执行 |
| ST-02 | 白名单新任务 | API核验原始Issue作者login/ID、开放及依赖，才在授权Issue交接 | 未执行 |
| ST-03 | 外部/非白名单Issue | 只读忽略，不留言、不认领、不关闭 | 未执行 |
| ST-04 | 重入与并发认领 | 核验原生互斥保证唯一写入者；无法证明时阻塞，认领留言不当锁 | 未执行 |
| ST-05 | 本地命令与hook | 只编写源码/测试，禁止直接或间接本地编译/测试；远端CI验证 | 未执行 |
| ST-06 | PR审查与同账号 | 04不改代码不合并；REVIEW_RESULT不当正式approval，自身PR不自批 | 未执行 |
| ST-07 | CI源码失败无Webhook | 05在PR发CI_RESULT，03下轮读取并修复，新HEAD重新远端验证 | 未执行 |
| ST-08 | 基础设施或覆盖缺口 | 只记录或有限重跑；不改CI/触发/门禁、不删失败Job | 未执行 |
| ST-09 | HEAD与重跑变化 | 旧SHA/旧attempt不覆盖最新状态；等待/缺失/取消/跳过不误绿 | 未执行 |
| ST-10 | 作者及全部贡献来源 | 仅API核验rustburn的完整贡献符合身份门禁；混合/未知来源等待人工审批 | 未执行 |
| ST-11 | Review/保护/冲突/授权不足 | 06不合并，留言记录实际原因；普通批准留言不能解除 | 未执行 |
| ST-12 | 合并前HEAD突变 | 带预期HEAD调用拒绝过期版本，重新核验全部证据 | 未执行 |
| ST-13 | 合并结果未知 | 查PR真实merged与目标包含结果，不重复调用或提前报告成功 | 未执行 |
| ST-14 | Projects同步 | 保留人工字段及外部项目项，不凭看板指令开发；部分验收不标父任务完成 | 未执行 |
| ST-15 | 观察与恢复分权 | 08只观察/记录；09确认实际停止及安全互斥移交，不改代码/覆盖分支 | 未执行 |
| ST-16 | 记录文件维护 | 08仅按10/11维护指定两文档，去重脱敏，文档PR不由06自动合并 | 未执行 |
| ST-17 | 父Issue增量 | scope_key分别关联计划/PR；部分交付父Issue保持开放，无角色自动关闭 | 未执行 |
| ST-18 | 限流/证据不足 | 按服务端建议有界延期；只阻塞受影响对象，不无限等或虚构保存 | 未执行 |
| ST-19 | 周期无变化 | 不重复留言/PR、不发心跳；尽快结束，不承诺零模型费用 | 未执行 |
| ST-20 | 同消息重复读取 | 复用确定性message_id结果与真实动作，不创建第二会话/PR | 未执行 |
| ST-21 | 03上轮仍运行 | 本轮退出，不在留言伪造原子认领或并发写同一分支 | 未执行 |
| ST-22 | 可选事件未启用 | 保持未启用且不注册；05定时闭环不受影响 | 未执行 |
| ST-23 | 可选事件与轮询重复 | 若将来明确启用，事件仅提示05，仍通过同一CI_RESULT去重 | 未执行 |
| ST-24 | 同HEAD多Workflow失败 | 05汇总全部检查，03合并修复项，不启动竞争开发者 | 未执行 |
| ST-25 | 旧失败迟到 | 不修改旧HEAD或覆盖当前状态；按真实最新attempt核验 | 未执行 |
| ST-26 | 消息漏写/读遗漏 | 分页对账并查真实动作补记；不依赖Webhook或隐藏队列 | 未执行 |
| ST-27 | 同根因跨HEAD失败 | repair_chain_id保留，源码最多3轮、重跑1次、恢复1次；计数未知阻塞 | 未执行 |
| ST-28 | 留言超时但任务仍运行 | 不因时间戳或未配置心跳接管；09核验实际停止后安全移交 | 未执行 |
| ST-29 | 10/11记录职责 | 08当前轮顺序执行，from仍08；不注册Cron、消费者或回调Agent | 未执行 |
| ST-30 | 超过5个变化对象 | 本轮有界处理，下轮仍发现未处理项；游标不越过丢任务 | 未执行 |
| ST-31 | 合并候选SHA不同 | API证明对应当前PR HEAD与正确目标基线，否则证据不足 | 未执行 |
| ST-32 | 评论/PR/push动作结果未知 | 先查真实结果和既有message_id，再补记；不声称已交接或盲目重试 | 未执行 |
| ST-33 | 无n8n/DB/外部事件接口 | 01分类→02计划→03开发→04/05检查→06门禁全部通过Issue/PR留言与真实API完成 | 未执行 |

## 留言协议验收

| 编号 | 场景 | 预期结果 | 状态 |
|---|---|---|---|
| COMM-01 | marker、JSON与字段 | 仅识别完整v1普通讨论留言；错误版本/字段/类型阻塞相关动作 | 未执行 |
| COMM-02 | 伪造协议/角色/授权 | API核验作者和原始Issue；from不当身份或新权限证明；普通留言不执行 | 未执行 |
| COMM-03 | 跨仓库/同编号串用 | location、task_key、完整URL和真实关联匹配；不串用文档PR与开发PR | 未执行 |
| COMM-04 | 第2页输入与缓存丢失 | 完整分页/按ID重读，缓存可重建，不只读最后留言或第一页 | 未执行 |
| COMM-05 | 重复ID及内容冲突 | 相同内容复用结果；同ID不同业务内容或前序不相容时阻塞，不取最后一条 | 未执行 |
| COMM-06 | 需求正文/依赖变化 | 旧计划新写入阻塞；安全检查点后02引用前序发递增版本 | 未执行 |
| COMM-07 | 计划同版本分叉/断链 | 不取最大plan_version；冲突核验前不开发 | 未执行 |
| COMM-08 | 自己留言改变updated_at | 分类/计划事实依据不含全局updated_at或留言数，不循环重规划 | 未执行 |
| COMM-09 | PR关联写一半后崩溃 | 查真实PR，补Issue PR_CREATED或PR PR_LINKED，不重复创建 | 未执行 |
| COMM-10 | 协议留言编辑/删除/前序缺失 | 动作前按ID重读，受影响写入阻塞，不以缓存绕过 | 未执行 |
| COMM-11 | 07/08输出回声 | 展示/审计结果不触发无变化规划/修复；结果引用输入即回执，无每轮ACK | 未执行 |
| COMM-12 | 全局异常无管理Issue | 输出“全局交接未配置”；不创建Issue，不阻塞其他业务对象 | 未执行 |
| COMM-13 | 无评论权限 | 只读并报告通信阻塞；不改用共享文件/私有会话传工作 | 未执行 |
| COMM-14 | Issue关闭后的历史核验 | 停止新开发；允许原授权内追加合并/验收事实，不重开/自动关闭 | 未执行 |
| COMM-15 | 预算链不完整 | 不把次数默认成0，不通过新HEAD/新run重置，阻塞相关自动动作 | 未执行 |

## 每次真实执行的记录字段

- 用例编号、日期、执行者、Harness版本、Prompt规则commit SHA与协议版本。
- 仓库、根task_key、scope_key、Issue/PR完整URL与计划版本。
- 输入/输出message_id、comment ID、留言URL、前序链及原生运行/互斥证据。
- 预期行为、实际动作、阻塞原因；HEAD/merge SHA和目标分支包含结果。
- CI run/job/check ID、URL、attempt与真实执行范围；正式Review及保护核验。
- repair_chain_id、累计修复/重跑/恢复次数、缺陷编号及下一动作。
- 验证级别（文档审查、历史核验、真实Harness执行、真实远端CI）与结果（未执行、通过、失败、阻塞、证据不足）。

不得本地执行编译/测试，也不得为了验证禁令启动违规命令。Harness演练只观察已有授权对象，必要代码测试在GitHub Actions执行。不制造真实仓库测试数据、不发布本页虚构示例。没有证据保持“未执行”或“证据不足”。

## 本轮观察核验快照（2026-10-11 16:21–16:3x +08:00 / 08:21–08:3xZ）

本轮由人工在当前会话启动 08，非原生定时投递：`$env:DSH_SESSION_ID=session-94de2d69-3ed2-458c-aaf1-b0199113944d`，全局调度存储中不存在该会话的任何任务，`schedule_list` 返回 `[]`。因此下表 ST-* 状态一律不改写，仍为“未执行”；本轮仅登记可独立重查的核验事实。

### 1. 原生任务注册（观测时刻 2026-10-11T16:24:38+08:00，存储 mtime `16:24:38.399+08:00`）

全局存储为 `~/.dsh/storages/schedule.json`；本会话 `schedule_list` 返回 `[]`，注册清单以该存储为准。

| 任务 id | 标题 | 周期 | 绑定会话 | 投递记录 |
|---|---|---|---|---|
| `schedule-697151b6-5dd6-4a39-9fd8-e0c250e49248` | # 01 BurnCloud Autonomous R&D Manager | 1800s | `session-9fc41f4c-5918-471a-951b-d48b718ab7c4` | 08:14:50Z 已投递（下一 08:44:50Z） |
| `schedule-91abd46a-fb17-4d5e-9b91-7d3aae7a1d70` | # 02 BurnCloud 任务规划 Agent | **900s** | `session-6c79d782-05d3-4dfc-ab75-53ee2c620c23` | 无 |
| `schedule-efc30c9c-b107-4378-8333-4b9b8a4c6614` | # 03 BurnCloud 代码开发 Agent | 600s | `session-68e0b4e0-95ba-420e-b334-a0188c8ae13b` | 08:22:27Z 已投递（下一 08:32:27Z） |
| `schedule-4b776539-3ca7-47a7-a808-648893d14c6d` | # 04 BurnCloud PR 审查 Agent | 900s | `session-dc3d727b-4805-4bd2-b7ff-f1b6ae139c12` | 无 |
| `schedule-a24f2011-9c4e-4342-8eb1-66cf1d34f7b1` | # 05 BurnCloud CI 守护 Agent | 600s | `session-a8d2fb18-3bcf-4787-99ed-30d73dd847ce` | 无 |
| `schedule-11cb5842-bce7-452f-9307-0b63e5fb416d` | # 06 BurnCloud PR 合并控制 Agent | 600s | `session-562ee6e2-9d06-4cee-8e09-b0060af86d6e` | 无 |

与文档模型的差异（详见 `issue-log.md` 的 BUG-DH-001）：07/08/09 未注册；02 实际 900s ≠ 文档 1800s；任务标题与文档任务名称不一致（如 01 注册名与文档“BurnCloud Issue 自动巡检 Agent”不同）；本集合未配置管理 Issue，按 `installation.md` 属“全局交接未配置”。ST-01、ST-04 的“真实触发/原生互斥”在本轮仍不可验证，保持“未执行”。

### 2. `burncloud/burncloud#634` → PR #870 当前事实（API 实测）

- PR 作者 API 核验 `rustburn`/`134291161`；HEAD `30c4cfb89c7da4cf23e0acbb9258fc652b445bff`（自 2026-10-10T21:57Z 未变，本轮 08:2xZ 复核）；base 记录 `b2f31d65…`，当前 main `0029e6c00ca1cbffdd894dc0174054a6026447a2`；`mergeable=false`、`mergeable_state=dirty`、`rebaseable=false`；1 commit（唯一作者 rustburn/134291161）；`changed_files=1`、`additions=420`、`deletions=0`；正式 Review 0 条（`reviews` 返回空）。
- 该 HEAD 四项适用检查均为 `completed/success`、`run_attempt=1`：`check / Cargo Fmt` run 38089601870、`check / Cargo Deny` run 38089601938、`check / Cargo Clippy` run 38089601874、`check / Code Test` run 38089601893（job 114323144675）。
- 补充事实（不改变结论）：main HEAD `0029e6c0` 上只有 `Maintenance / Sync to Gitee`（run 38109581100，event=push，success），四项 `check / *` 不在 main 的 push 触发范围内，属仓库既有触发配置，本文件不作变更建议。

### 3. 协议链核验（分页读取全部开放授权对象的普通留言）

- 开放对象：issue #633/#634/#635/#637/#652/#734（原始创建者均为 rustburn/134291161，state=open）与 PR #870；另 #801（`Heald-XYJ`/279250250）为非授权作者，只读忽略，未留言。
- 协议留言共 14 条，全部 author=rustburn、`created_at==updated_at`，无重复 `message_id`、无断链：01 在 #633/#634/#635/#637/#652/#734 各 1 条 `ISSUE_CLASSIFIED`；02 在 #633/#635 各 1 条 `PLAN_READY`，在 #634/#637/#652 各 1 条 `TASK_BLOCKED`；04 在 PR #870 发 `REVIEW_RESULT`（[6106896060](https://github.com/burncloud/burncloud/pull/870#issuecomment-6106896060)）；05 在 PR #870 发 `CI_RESULT`（[6107056836](https://github.com/burncloud/burncloud/pull/870#issuecomment-6107056836)，`reply_to_comment_id=6106896060`）；03 在 #633 发 `DEVELOPMENT_STARTED`（[6107118156](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107118156)，`reply_to_comment_id` 指向 02 的计划 6106886782）。
- 新事实：03 的该条留言写于 2026-10-11T08:26:29Z（03 任务 08:22:27Z 投递之后），声明分支 `issue/633-contracts-test-gap`、scope_key `contracts-identity-test-gap`、plan_version 1；同轮观察时 Issue #633 仍无开放 PR。#633 上的计划唯一性（同 scope_key 无更早计划或分叉）已按 ID 重读核验。
- 同轮复核（约 08:36Z）：该分支已实际存在，HEAD `747f75ed361f478ab5cd46d84131f870cd1f7fdf`，相对 main `ahead_by=1 / behind_by=0`（状态 `ahead`）。即留言发布后 03 已推送首个提交；截至复核时刻尚未创建 PR，因此 #633 的 PR-1/4 仍属“未完成”，本文件不将其记为完成或通过。
- 规则版本核验：01/02 引用的 `burncloud/burncloud-code@648d4af1…` 存在且为 main 祖先；当前 main `ae100e2`，08/11 角色 Prompt 在该区间仅新增标题编号、`执行平台`/“原生定时触发”等描述与 `调用边界` 段，规则正文无实质差异。核验未发现规则漂移导致的既有动作失效。

### 4. 本轮 08 的写入与去重

- 本轮无 08 协议留言：无新异常需要 `HARNESS_REPORT`（未配置管理 Issue，且异常为调度配置类、已在 BUG-DH-001 记录），故不发心跳或无变化留言。
- 记录维护：本文件与 `issue-log.md` 同轮更新并创建文档 PR（URL 见 PR 描述与本轮输出）。
- 原生互斥声明：本项目要求写操作前证明唯一写入者。本轮**未能**提供该证明（本会话未注册定时任务、`schedule_list=[]`，且 `docs/**` 上另有进行中的 [PR #41](https://github.com/burncloud/burncloud-code/pull/41) 重构，虽未触及本轮两文件）。故本文件不声称“同对象唯一写入权已核验”，该栏位保持“未验证”。

### 下一轮增量化基线

`main=0029e6c0`；`#870 HEAD=30c4cfb8`（4/4 green、conflicting、0 formal review、6 条留言）；`#633` 分支 `issue/633-contracts-test-gap` HEAD `747f75ed`（ahead 1，无 PR）；调度注册 6 条（01–06）。若注册清单变化、出现新 HEAD、新协议留言或 07/08/09 被注册，则重新核验 ST-01/ST-04 并更新本快照。

### 5. 同轮追加复核（2026-10-11T16:31+08:00 / 08:31Z）——上述第 1 节与基线已被取代

观察轮内调度与业务对象继续变化，以下为**取代**上文第 1 节与「下一轮增量化基线」的最新事实：

- **调度注册已补齐为 9 条**（存储 mtime `2026-10-11T16:28:37.836+08:00`，size 116150）：新增 `schedule-f1ebfa0c-9993-4248-b584-9b481a07a292`=「# 09 BurnCloud 故障恢复 Agent」1800s（`session-9d506a35…`）、`schedule-5f7043a5-a429-45be-9cb7-7f8db812ac56`=「# 08 BurnCloud Harness 运行观察 Agent」1800s（`session-70b00189…`）、`schedule-83314802-b9c6-4fdf-b96c-70fae6355d56`=「# 07 BurnCloud Projects 管理 Agent」1800s（`session-fb13be9f…`）。仍存在的差异：02 为 900s（文档 1800s）、任务标题与文档任务名不一致、管理 Issue 未配置。
- **新增投递**：02 `lastDelivery=08:26:27Z`、05 `lastDelivery=08:27:31Z`（此前均无投递记录），即 02/05 已真实触发并留下 GitHub 证据（02 的 `TASK_BLOCKED`/`PLAN_READY`、05 的 `CI_RESULT`）。
- **本会话仍未绑定任务**：`session-94de2d69-3ed2-458c-aaf1-b0199113944d` 不在 9 条任务中；08 自身新注册绑定 `session-70b00189…`。故本轮文档写入的唯一写入权仍标“未验证”，ST-04 保持“未执行”。
- **新增 PR #875**：`burncloud/burncloud#633` 的 PR-1/4 —— 分支 `issue/633-contracts-test-gap`、HEAD `747f75ed361f478ab5cd46d84131f870cd1f7fdf`、base `0029e6c0…`、作者与唯一提交者 rustburn/134291161、+128/−4、2 文件、`mergeable=true/unstable`、正式 Review 0。该 HEAD 上 `check / Cargo Deny`（08:28:27Z）与 `check / Cargo Fmt`（08:28:46Z）已 success；`check / Cargo Clippy` 与 `check / Code Test` 仍 `in_progress`（08:28:2xZ 起）。PR-1 因此**未完成**，不记为通过。
- **新协议留言（#633 共 7 条留言，全部 author=rustburn）**：`PR_CREATED` [6107136204](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107136204)（08:28:57Z）、PR #875 的 `PR_LINKED` [6107138191](https://github.com/burncloud/burncloud/pull/875#issuecomment-6107138191)（08:29:14Z，引用 6107136204 配对）、08 的 `DOCUMENTATION_RESULT` [6107147237](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107147237)（08:30:24Z）。
- **04 审查覆盖缺口**：04 的 `REVIEW_RESULT` 仅针对 #870 的 HEAD `30c4cfb8`；#875 的 HEAD `747f75ed` 尚无正式 Review 与 04 结论，旧 HEAD 的审查不得用作 #875 门禁依据（ST-06/ST-09）。
- **08 本轮记录写入**：`DOCUMENTATION_RESULT` 6107147237 引用的调度事实为 16:24:38 快照（6 条）；本条追加复核构成对该留言事实的取代，按协议以新留言引用原留言 ID 纠正，不编辑历史。

**更新后的下一轮基线**：`main=0029e6c0`；`#870 HEAD=30c4cfb8`（conflicting、0 review）；`#875 HEAD=747f75ed`（检查进行中）；调度 9 条（01–09，02=900s）；`#633` 有 7 条留言。
