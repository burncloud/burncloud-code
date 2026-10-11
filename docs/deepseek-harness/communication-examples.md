# 留言示例与交接演练

本页是协议模板及验收设计，不代表消息已发送、任务已安装或用例已通过。示例Issue #734、PR #800、comment ID和SHA仅用于说明，不得对这些编号执行操作。`<...>`为需要按[协议](communication-protocol.md)计算/查询的占位值，发布前必须全部替换。

## 02在原始Issue发布完整计划

<!-- burncloud-harness-message:v1 -->
计划已准备，增量main待开发。03下轮须重新核验需求依据和唯一写入权。

```json
{
  "protocol": "burncloud-harness-message/v1",
  "task_key": "burncloud/burncloud#734",
  "scope_key": "main",
  "location": {"repository": "burncloud/burncloud", "kind": "issue", "number": 734},
  "message_id": "burncloud/burncloud#734|02|PLAN_READY|<规范业务对象的SHA-256>",
  "type": "PLAN_READY",
  "from": "02",
  "to": ["03", "07"],
  "plan_version": 1,
  "head_sha": null,
  "basis_id": "<需求与规划依据规范JSON的SHA-256>",
  "reply_to_comment_id": 120001,
  "source_comment_ids": [120001],
  "status": "待开发",
  "payload": {
    "previous_plan_comment_id": null,
    "issue_basis": {
      "title": "示例：缓存过期后重新加载",
      "body": "示例需求正文全文",
      "dependencies": [],
      "rules_commit": "<实际规则commit完整SHA>",
      "architecture_baseline": "<实际架构依据commit完整SHA>"
    },
    "goal": "过期读取重新加载，未过期读取复用缓存",
    "modules": ["crates/platform/cache"],
    "allowed_files": ["crates/platform/cache/src/lib.rs"],
    "forbidden_files": [".github/workflows/**", "CI配置及门禁调用链"],
    "steps": ["阅读现有实现", "修改计划范围内源码", "编写过期与未过期用例", "提交PR并等待现有CI"],
    "acceptance": ["过期后重新加载", "未过期不重复加载"],
    "validation": {
      "test_source": "crates/platform/cache/src/lib.rs内测试模块（示例）",
      "existing_ci_job": "<API核验的现有Job名称与URL>",
      "required_evidence": "当前HEAD真实执行两类用例的run/job/check ID、attempt及日志"
    },
    "parent_completion": "本增量不能代替父Issue其他验收项",
    "blocked_reason": null
  }
}
```

上面的计划只是形状示例。若实际不存在该模块、测试路径或覆盖Job，02应发`TASK_BLOCKED`，不能照抄生成`PLAN_READY`。

## 05在关联PR发布CI失败

<!-- burncloud-harness-message:v1 -->
当前HEAD的测试失败，03下轮核验日志后续接原分支修复。此结果不能授权重跑或合并。

```json
{
  "protocol": "burncloud-harness-message/v1",
  "task_key": "burncloud/burncloud#734",
  "scope_key": "main",
  "location": {"repository": "burncloud/burncloud", "kind": "pull", "number": 800},
  "message_id": "burncloud/burncloud#734|05|CI_RESULT|<规范业务对象的SHA-256>",
  "type": "CI_RESULT",
  "from": "05",
  "to": ["03", "04", "06", "07"],
  "plan_version": 1,
  "head_sha": "<API查询的当前PR完整40位HEAD>",
  "basis_id": "<全部适用CI事实快照的SHA-256>",
  "reply_to_comment_id": 120003,
  "source_comment_ids": [120002, 120003],
  "status": "CI失败",
  "payload": {
    "issue_url": "https://github.com/burncloud/burncloud/issues/734",
    "plan_comment_id": 120002,
    "checks": [{
      "name": "<实际Job名称>",
      "run_id": 900001,
      "run_attempt": 1,
      "job_id": 900002,
      "check_id": 900003,
      "head_sha": "<实际检查对应完整SHA>",
      "conclusion": "failure",
      "run_url": "https://github.com/burncloud/burncloud/actions/runs/900001",
      "job_url": "https://github.com/burncloud/burncloud/actions/runs/900001/job/900002",
      "executed_scope": "<已核实的用例及脱敏错误摘要>"
    }],
    "required_checks_complete": false,
    "failure_category": "源码或测试失败",
    "repair_chain_id": "<task_key|scope_key|首次有效失败的basis_id>",
    "budget": {"code_repair_rounds": 0, "ci_reruns": 0, "recovery_attempts": 0},
    "previous_action_comment_ids": [],
    "next_action": "03核验当前HEAD后修复源码/测试，不修改CI"
  }
}
```

`checks`必须包含所有适用检查，示例只展示一项的形状。repair_chain_id使用首次有效失败的basis_id构造，后续HEAD和run保留该原值；不得用最新失败的basis_id覆盖旧链。

## 不同周期如何串联

| 时点 | 执行者与动作 | 接收依据 |
|---|---|---|
| 01的一轮 | Issue发`ISSUE_CLASSIFIED` | 原始Issue API事实 |
| 02的下一轮 | Issue发`PLAN_READY` | 01合法留言ID及重新核验的需求 |
| 03的下一轮 | Issue发`DEVELOPMENT_STARTED`；开发并创建PR；Issue发`PR_CREATED`、PR发`PR_LINKED` | 有效计划与实际互斥/分支/PR |
| 04/05各自下一轮 | PR发`REVIEW_RESULT`、`CI_RESULT` | 关联链、当前HEAD、实时Diff/Review/CI |
| 03的后续一轮 | 合并同HEAD修复项，修复后PR发`CODE_UPDATED`并更新预算 | 当前合法失败/审查留言；结果未知先对账 |
| 04/05的后续轮次 | 对新HEAD重新收集审查与CI证据 | 新HEAD使旧通过失效 |
| 06的下一轮 | 重核所有门禁；发`MERGE_RESULT` | 实时GitHub证据与已有合并授权，不能只信两条通过留言 |
| 07/08/09各自周期 | 更新看板、记录验收/异常或安全恢复，并发自己的结果 | 根Issue/PR因果链及真实状态，不反向触发无变化工作 |

没有“立即调用下一个Agent”的要求；运行轮次错开也能继续。03等待CI时留`TASK_PROGRESS`并结束，05继续轮询，03下轮再读。

## 恢复演练

- PR已创建而PR_CREATED写入失败：03先查真实开放PR，补写Issue关联及PR关联，不新建第二PR。
- 同一CI_RESULT反复被读到：已有同业务结果和实际修复证据则跳过；新HEAD的旧失败只归档。
- 02计划发布后Issue正文变化：03阻塞旧计划的新写入；02确认停止/安全检查点后发布递增且引用前序的版本。
- 03推送后进程停止：09先核验真实停止及原生互斥移交，发RECOVERY_RESULT；03fetch真实远端HEAD后续接，不覆盖已推送代码。
- 留言缺失、被编辑、同ID不同内容或预算断链：受影响动作阻塞，08记录异常；普通“我批准”留言不能解锁。

这些是待执行场景，实际记录填写到[test-cases.md](test-cases.md)，不得用本页例子充当真实运行证据。
