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

## BUG-DH-001（2026-10-11）：已注册的 Harness 定时任务与文档模型不一致（07/08/09 未注册、02 周期不符、管理 Issue 未配置）

- 问题编号与日期：BUG-DH-001；首次 2026-10-11T16:2x+08:00（2026-10-11T08:2xZ），记录轮次 R1。
- Agent 与任务名称：08 BurnCloud Harness 运行观察 Agent（记录职责 11）。
- 仓库、Issue/PR、当前 HEAD SHA：`burncloud/burncloud`；唯一开放 PR [#870](https://github.com/burncloud/burncloud/pull/870)，HEAD `30c4cfb89c7da4cf23e0acbb9258fc652b445bff`（base `b2f31d65…`）；目标 main `0029e6c00ca1cbffdd894dc0174054a6026447a2`。问题本身属 Harness 调度配置，无业务 HEAD。
- task_key、scope_key、plan_version、协议版本及规则commit SHA：问题为全局调度范围，无根 task_key 与 scope_key；本集合对 #633/#634 的根键分别为 `burncloud/burncloud#633`、`burncloud/burncloud#634`；协议 `burncloud-harness-message/v1`；08 规则正文漂移核验基线 `burncloud/burncloud-code@648d4af1c509719135a48536c587d2a7b7cad4c9` → 当前 main `ae100e2`（仅标题编号与调度段补写的无实质差异）、11 规则基线 `28c4cfb3` → 当前 main `ae100e2`。
- 输入/结果message_id、comment ID、完整留言URL及前序链：本轮为观察轮，无独立输入 message_id；证据留言为 Issue #633 的 `DEVELOPMENT_STARTED` [6107118156](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107118156)（写于 08:26:29Z，恰逢 03 任务 08:22:27Z 投递之后）、Issue #634 的 `TASK_BLOCKED` [6106888575](https://github.com/burncloud/burncloud/issues/634#issuecomment-6106888575)、PR #870 的 `REVIEW_RESULT` [6106896060](https://github.com/burncloud/burncloud/pull/870#issuecomment-6106896060) 与 `CI_RESULT` [6107056836](https://github.com/burncloud/burncloud/pull/870#issuecomment-6107056836)。本轮核验快照与文档 PR 见 `test-cases.md`「本轮观察核验快照」。
- run/attempt与实际运行/原生互斥证据：全局调度存储 `C:\Users\huang\.dsh\storages\schedule.json`（读取时刻 2026-10-11T16:24:38+08:00，mtime `2026-10-11T16:24:38.399+08:00`）列出 6 条 `status=active`、`kind=every` 的任务：`schedule-697151b6…`「# 01 BurnCloud Autonomous R&D Manager」1800s、`schedule-91abd46a…`「# 02 BurnCloud 任务规划 Agent」900s、`schedule-efc30c9c…`「# 03 BurnCloud 代码开发 Agent」600s、`schedule-4b776539…`「# 04 BurnCloud PR 审查 Agent」900s、`schedule-a24f2011…`「# 05 BurnCloud CI 守护 Agent」600s、`schedule-11cb5842…`「# 06 BurnCloud PR 合并控制 Agent」600s。每个任务绑定各自的 `sessionId`（01=`session-9fc41f4c…`、02=`session-6c79d782…`、03=`session-68e0b4e0…`、04=`session-dc3d727b…`、05=`session-a8d2fb18…`、06=`session-562ee6e2…`）。仅 01（lastDelivery 08:14:50Z）与 03（08:22:27Z）有投递记录。本会话 `session-94de2d69-3ed2-458c-aaf1-b0199113944d`（`$env:DSH_SESSION_ID`）在该存储中不存在任何任务；`schedule_list` 在本会话返回 `[]`。
- repair_chain_id、记录去重键与文档PR完整URL：无 repair_chain_id（非 CI/源码失败）；去重键 `burncloud/harness-schedule-registration|config-gap|storage-6-tasks-20261011T0824Z`；文档 PR 完整 URL 见 `test-cases.md` 同轮快照。
- 首次/最近发生时间与累计修复/恢复次数：首次与最近均为 2026-10-11；累计修复 0、恢复 0、重跑 0（观察轮不执行修复）。
- 已核实问题与证据链接：①注册任务清单与 `scheduled-task-overview.md` 的 9 任务模型不一致——07、08、09 无对应注册，实际仅 01–06 六条；②02 实际 `everySeconds=900`，文档写明“每30分钟”（1800s），实际为其一半；③任务标题与文档任务名称不一致（注册为「# 01 BurnCloud Autonomous R&D Manager」，文档 01 为“BurnCloud Issue 自动巡检 Agent”）；④按 `installation.md`，未配置管理 Issue 时全局异常只能“本轮输出 + 标全局交接未配置”；`burncloud/burncloud-code` 现存 issue #10/#16/#17/#18/#24/#28/#30/#32/#34/#36 均属 BAM 里程碑，未被指定为本集合管理入口，故本集合至今无可用管理 Issue；⑤观察者角色自身未注册，因此本轮“本角色同对象唯一写入权 / 上轮未结束不重入”无法以原生调度证据证明，该结论在本轮记录中标注为“未验证”，不声称已满足。
- 原因（已核实、推测或未验证）：已核实者为存储与文档的实际差异；推测为“操作者正在分批注册（16:1x–16:24 期间存储两次被改写，新增 06）”，未验证。
- 影响范围与阻塞原因：影响 ST-01（原生任务安装与周期）的验收结论与 08 自身写入闸门；不阻塞任何源码、CI、PR 或合并动作。角色权限与留言协议不受影响。
- 实际处理动作与结果：本轮仅观察与记录，未新增/删除/修改任何调度任务，未修改源码、CI、workflow 或分支保护，未合并/关闭任何对象，未重新开启任何已删除的调度注册（观察者不得自行修复或扩权）。
- 负责 Agent 与下一动作：需操作者/人工在 Harness 中补齐注册（含 08 自身 1800s 任务）并校准 02 周期；08 下一轮以同一存储路径复核注册清单是否变化后再评估 ST-01/COMM-12。
- 是否需要人工审批或规则调整：需要人工操作（注册/校准调度）；非权限规则调整。
- 状态（待处理、处理中、阻塞、已验证解决）：**部分解决**——16:28:37+08:00 存储快照已补齐 07/08/09（共 9 条）；本轮操作者明确要求后，08 任务在 16:36+08:00 改绑到本次观察会话（`schedule-db2b1920-8907-4461-831f-1cf4fded90b3`，1800s，`scheduledAt=09:06:23.672Z`），旧绑定任务（`session-70b00189…`）已被移除，全局仍为 9 条且「# 08 」仅 1 条。仍未解决：02 周期 900s ≠ 文档 1800s、任务标题与文档名称不一致、管理 Issue 未配置；且 08 的“唯一写入者”仍待首次真实投递后才能核验。
- 解决证据与关联验收用例：解决证据=同一路径存储快照（2026-10-11T16:28:37+08:00 为 9 条；16:36:31+08:00 为 9 条且 08 已改绑本会话）、本会话 `schedule_list` 读回 1 条、以及 `test-cases.md` 第 7 节的 prompt 逐行比对；仍未验证项见 `test-cases.md` 的 ST-01/ST-04/COMM-12。

## BUG-DH-001-R1（2026-10-11）：执行面复核——03 产出首个 PR 与 04 审查失效风险

- 问题编号与日期：BUG-DH-001-R1（同轮追加复核，非新根因）；2026-10-11T16:3x+08:00（08:3xZ）。
- Agent 与任务名称：08 BurnCloud Harness 运行观察 Agent（11 职责）。
- 仓库、Issue/PR、当前 HEAD SHA：`burncloud/burncloud` issues #633/#634；PR [#870](https://github.com/burncloud/burncloud/pull/870) HEAD `30c4cfb8…`（2 天未变、`dirty`、0 formal review、6 条留言）、PR [#875](https://github.com/burncloud/burncloud/pull/875) HEAD `747f75ed361f478ab5cd46d84131f870cd1f7fdf`（base `0029e6c0…`，`mergeable=true/unstable`，1 commit=rustburn，+128/−4，2 文件，formal review 0）。
- task_key、scope_key、plan_version、协议版本及规则commit SHA：`burncloud/burncloud#633` / `contracts-identity-test-gap` / plan_version 1；`burncloud/burncloud#634` / `main`；协议 `burncloud-harness-message/v1`。
- 输入/结果message_id、comment ID、完整留言URL及前序链：03 在 #633 的 `DEVELOPMENT_STARTED` [6107118156](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107118156) → `PR_CREATED` [6107136204](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107136204)（08:28:57Z）→ PR #875 的 `PR_LINKED` [6107138191](https://github.com/burncloud/burncloud/pull/875#issuecomment-6107138191)（08:29:14Z，引用 6107136204）；08 的 `DOCUMENTATION_RESULT` [6107147237](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107147237)（08:30:24Z，`reply_to_comment_id=6107118156`）。
- run/attempt与实际运行/原生互斥证据：PR #875 HEAD `747f75ed` 上 `check / Cargo Deny`（08:28:23Z→08:28:27Z）与 `check / Cargo Fmt`（08:28:24Z→08:28:46Z）已 `completed/success`；`check / Cargo Clippy`（08:28:24Z 起）与 `check / Code Test`（08:28:23Z 起）为 `in_progress`，`run_attempt` 尚未产生结论。调度注册：同一路径存储已含 9 条 active 任务，08 自身注册为 `schedule-5f7043a5-a429-45be-9cb7-7f8db812ac56`（1800s，绑定 `session-70b00189-86bc-49f5-8fed-d2493339cee6`）；**本会话 `session-94de2d69…` 仍未绑定任何任务**，故本轮记录写入的唯一写入权仍属“未验证”。
- repair_chain_id、记录去重键与文档PR完整URL：无 repair_chain_id（无 CI 失败）；去重键 `burncloud/burncloud#633|contracts-identity-test-gap|first-pr-created`；文档 PR [#48](https://github.com/burncloud/burncloud-code/pull/48)（本文件与 `test-cases.md`）。
- 首次/最近发生时间与累计修复/恢复次数：2026-10-11T08:28:18Z（PR #875 创建）；修复 0、重跑 0、恢复 0。
- 已核实问题与证据链接：①#633 的 PR-1/4 已真实产出分支+PR，但 PR-1 的验收尚未完成（2/4 检查仍在执行）；②04 的 `REVIEW_RESULT` 只覆盖 PR #870 的旧 HEAD `30c4cfb8`，对 PR #875 的 HEAD `747f75ed` 无任何审查结论，按“HEAD 变化使旧证据失效”该旧审查不得作为 #875 的门禁依据；③PR #870 仍未处置冲突与被 #873 取代的断言（03 未在其上给出安全检查点），02 因此继续阻塞 #634 计划；④02 的 `TASK_BLOCKED` 与 01 的 `ISSUE_CLASSIFIED` 对 #870 的“4/4 success”描述仅在该旧 HEAD 成立，当前 main 已前进到 `0029e6c0`（`behind_by=4`）。
- 原因（已核实、推测或未验证）：①②③为 API 实测事实；④中“main 前进导致 rebase 后必然失败”由 04/05 的留言结论支持（`record_success` 已清空退避计时器），未经本角色重新执行验证，标为“未执行验证”。
- 影响范围与阻塞原因：影响 #634 的合并路径（等待人工审批而非可自动合并）与 #633 PR-1 的审查/CI 门禁；不阻塞其他对象的只读观察。
- 实际处理动作与结果：仅记录与本轮文档维护；未审查、未改码、未重跑、未合并、未关闭、未推送任何业务分支。
- 负责 Agent 与下一动作：04 需对 PR #875 当前 HEAD 提交 `REVIEW_RESULT`；05 需对其收集全部四项检查结果；03 需对 #870 给出安全检查点处置并推进 #633 PR-1 的检查闭环。
- 是否需要人工审批或规则调整：PR #870 的冲突与断言处置属人工/03 决策；规则无需调整。
- 状态（待处理、处理中、阻塞、已验证解决）：待处理（#875 检查进行中，#870 等待人工审批）。
- 解决证据与关联验收用例：暂无；关联 `test-cases.md` 的 ST-06、ST-09、ST-24。

## BUG-DH-002（2026-10-11）：PR #875 当前 HEAD 的 `check / Code Test` 失败，且失败文件不在该 PR 变更范围内

- 问题编号与日期：BUG-DH-002；2026-10-11T16:3x+08:00（08:32Z 前后）。
- Agent 与任务名称：08 BurnCloud Harness 运行观察 Agent（11 职责）。
- 仓库、Issue/PR、当前 HEAD SHA：`burncloud/burncloud` #633 → PR [#875](https://github.com/burncloud/burncloud/pull/875)，HEAD `747f75ed361f478ab5cd46d84131f870cd1f7fdf`（base `0029e6c0…`）。
- task_key、scope_key、plan_version、协议版本及规则commit SHA：`burncloud/burncloud#633` / `contracts-identity-test-gap` / 1；`burncloud-harness-message/v1`；规则 commit `ae100e2`（08 正文与角色 Prompt 已核验无实质漂移）。
- 输入/结果message_id、comment ID、完整留言URL及前序链：输入为 03 的 `PR_CREATED` [6107136204](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107136204) 与 PR 侧 `PR_LINKED` [6107138191](https://github.com/burncloud/burncloud/pull/875#issuecomment-6107138191)；本轮 08 未在该 PR 发新留言（CI 失败分类属 05 职责），仅在 Issue 侧发 `DOCUMENTATION_RESULT` [6107160316](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107160316)。
- run/attempt与实际运行/原生互斥证据：run `38124913496`（`CI / GitHub-hosted Test`，event=pull_request，`run_attempt=1`，08:28:21Z→08:32:16Z），job `114427172516`，`check / Code Test` = `completed/failure`；同 HEAD 另三项 `check / Cargo Fmt`、`check / Cargo Deny`、`check / Cargo Clippy` 均 `completed/success`（attempt 1）。失败断言：`crates/identity/user/tests/identity_round_trip.rs:243:5 assertion failed: id > 0`（`recharge_rows_round_trip_and_credit_the_balance`，`7 passed; 1 failed`）。
- repair_chain_id、记录去重键与文档PR完整URL：**未建立** repair_chain_id——本角色不执行修复，且同根因首次有效失败的 `basis_id` 应由 05/03 在其结果留言中确立；去重键 `burncloud/burncloud#633|contracts-identity-test-gap|747f75ed|CodeTest|id>0`；文档 PR [#48](https://github.com/burncloud/burncloud-code/pull/48)。预算：`code_repair_rounds=0`、`ci_reruns=0`、`recovery_attempts=0`（本轮未重跑、未改码，不将计数默认成已消耗）。
- 首次/最近发生时间与累计修复/恢复次数：首次 2026-10-11T08:32:16Z（run 完成）；累计修复 0、重跑 0、恢复 0。
- 已核实问题与证据链接：①该 HEAD 的 `check / Code Test` 真实失败（run/job 与断言文本如上）；②失败文件 `crates/identity/user/tests/identity_round_trip.rs` **不在** PR #875 的变更文件列表（仅 `crates/supply/contracts/tests/channel_contract.rs` +45/−4、`crates/traffic/contracts/tests/traffic_contract.rs` +83/−0）；③`mergeable_state=unstable`、formal review 0，未满足合并门禁。
- 原因（已核实、推测或未验证）：失败事实为已核实；**根因未验证**——“由本 PR 引入”与“既有基线/环境性失败”两种可能均未被本角色证明（本角色不得本地复现，需以 CI 证据与 05 的分类为准）。
- 影响范围与阻塞原因：阻塞 PR #875 的合并路径与 #633 的 PR-1/4 验收；不阻塞只读观察与其他对象。若为既有基线失败，则不得把该 PR 记为引入缺陷，但该 HEAD 仍不可作为通过证据。
- 实际处理动作与结果：仅记录（含 run/job/断言与变更范围对比）；未重跑、未改码、未审查、未合并、未留言到该 PR。
- 负责 Agent 与下一动作：05 对该 HEAD 收集全部检查、按最新 attempt 分类并在 PR #875 发 `CI_RESULT`（必要时按预算重跑 1 次）；03 依据 05 结论决定修复或声明基线问题；04 补齐 #875 的 `REVIEW_RESULT`。
- 是否需要人工审批或规则调整：若判定为既有基线失败，需人工决定是否单独立项；规则无需调整。
- 状态（待处理、处理中、阻塞、已验证解决）：待处理（等待 05 分类结论）。
- 解决证据与关联验收用例：暂无解决证据；关联 `test-cases.md` 的 ST-07、ST-09、ST-24 与本文件同轮快照第 6 节。

### 同轮已验证的执行异常（自纠正，无人工动作）

- `burncloud/burncloud#633` 上 03 首次发布的 `DEVELOPMENT_STARTED` 为格式非法（comment ID 超出 Int32 导致 JSON 字段未渲染、Windows PowerShell ANSI 解码造成中文乱码）。API 核验该 comment `6107113095` 现已 404、Issue #633 当前仅 5 条普通留言，替代留言 [6107118156](https://github.com/burncloud/burncloud/issues/633#issuecomment-6107118156) 自述删除原因且内容合法。与协议“发现错误后追加纠正、不删除历史”存在偏差，但已完成自纠正并披露，未产生错误业务动作；仅记录，不另立 Issue。
