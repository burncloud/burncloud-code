# BAM 状态机与事件守卫

## 状态
`BACKLOG, READY, RUNNING, WAIT_CI, FIX_CI, REVIEW, READY_FOR_REVIEW, AWAITING_MERGE, MERGED, VERIFY_MAIN, DONE, BLOCKED, PAUSED, CANCELLED` 为业务状态；CI 子状态 `QUEUED, IN_PROGRESS, SUCCESS, FAILURE, CANCELLED, TIMED_OUT, SKIPPED, MISSING` 不可混入业务枚举。

## 转移表
| From → To | 事件 | 必须成立的 Guard | 不通过 |
|---|---|---|---|
| BACKLOG→READY | Project Ready | agent-ready、无依赖阻塞、验收清晰 | Needs Specification |
| READY→RUNNING | lease acquired | 唯一写锁、Agent Budget/Policy PASS | READY |
| RUNNING→WAIT_CI | pushed | commit、远端 SHA、PR 关联确证 | RUNNING/BLOCKED |
| WAIT_CI→FIX_CI | CI failed | 最新 HEAD、可修代码错误、预算剩余 | BLOCKED/WAIT_CI |
| FIX_CI→RUNNING | repair selected | 同分支 lease、原任务上下文 | BLOCKED |
| WAIT_CI→REVIEW | CI passed | required checks 当前 HEAD 真正执行且成功 | WAIT_CI |
| REVIEW→READY_FOR_REVIEW | accepted | scope/criteria/CI/review evidence 均 PASS | REVIEW/BLOCKED |
| READY_FOR_REVIEW→AWAITING_MERGE | verified | HEAD 重读、无阻塞 review/conflict | REVIEW |
| AWAITING_MERGE→MERGED | merge observed | 真实 GitHub merged 或独立授权执行成功 | AWAITING_MERGE |
| MERGED→VERIFY_MAIN | merged commit | 合并事实+main SHA | BLOCKED |
| VERIFY_MAIN→DONE | main accepted | main 证据 + Issue closure 实际结果 | VERIFY_MAIN/BLOCKED |

任何暂停保留 prior_state；解除暂停时重新拉 GitHub、重验策略，不能沿用旧 PASS。外部 push 新 SHA、Ruleset 变化、Review Requested Changes、授权撤回可把 Review/Awaiting Merge 送回重新验证。BLOCKED 必须有 machine-readable reason code；DONE/CANCELLED 只能由显式重开事件重新进入。

## Guard 设计
`TransitionDecision {from,to,event_id,task_version,head_sha,policy_version,required_evidence_ids,verdict,reason}`；有版本号和 lease epoch 的乐观并发控制。转移日志必须与状态原子存储。

## 测试
对每条允许转移至少：正例、缺单项证据反例、新 HEAD 竞态反例、重复事件幂等例。STATE-01 新 HEAD 使 READY_FOR_REVIEW 失效；STATE-02 未授权无法 MERGED；STATE-03 SKIPPED required check 不从 WAIT_CI 晋级；STATE-04 验证主分支失败不能 DONE；STATE-05 恢复 PAUSED 重验。