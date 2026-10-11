# 单Agent聚合检查点示例

本页仅是模板和待执行演练，不代表已发留言、已安装或已验证节省token。Issue #734、PR #800、comment ID为示例，不得按编号执行操作；所有尖括号占位值须按真实API与[协议](communication-protocol.md)替换。

## 一轮内开发、自检与CI结果共用检查点

同一Agent已在Issue保存完整计划并关联PR。本轮推送后完成Diff自检、查询CI，无需分别发布三个内部交接消息。只在关联PR追加一次如下业务检查点：

<!-- burncloud-harness-message:v1 -->
代码已推送，自检暂无阻塞项，当前HEAD仍等待CI和正式Review。下轮先查询变化，不重新分析未变Diff。

```json
{
  "protocol": "burncloud-harness-message/v1",
  "execution_mode": "single-agent",
  "task_key": "burncloud/burncloud#734",
  "scope_key": "main",
  "location": {"repository": "burncloud/burncloud", "kind": "pull", "number": 800},
  "message_id": "burncloud/burncloud#734|00|TASK_CHECKPOINT|<业务对象SHA-256>",
  "type": "TASK_CHECKPOINT",
  "from": "00",
  "to": [],
  "plan_version": 1,
  "head_sha": "<实际PR完整40位HEAD>",
  "basis_id": "<相关事实规范JSON的SHA-256>",
  "reply_to_comment_id": 120003,
  "source_comment_ids": [120002, 120003],
  "status": "等待CI",
  "payload": {
    "issue_url": "https://github.com/burncloud/burncloud/issues/734",
    "branch": "<实际独占分支>",
    "plan_comment_id": 120002,
    "stage_results": {
      "03": {"push": "已通过API核实", "commit_sha": "<实际提交SHA>"},
      "04": {"self_check": "暂无阻塞项", "formal_review": "未满足", "review_urls": []},
      "05": {"all_applicable_checks_complete": false, "checks": [{
        "name": "<实际Job名称>", "run_id": 900001, "run_attempt": 1,
        "conclusion": null,
        "url": "https://github.com/burncloud/burncloud/actions/runs/900001"
      }]}
    },
    "repair_chain_id": null,
    "budget": {"code_repair_rounds": 0, "ci_reruns": 0, "recovery_attempts": 0},
    "next_action": "同一Agent下轮核验CI与正式Review变化"
  }
}
```

checks必须包含全部适用检查，例子只展示形状。零预算仅代表示例的新开发尚无失败链；旧工作预算不明时阻塞，不能照抄为0。自检通过不是正式批准；CI等待不算通过。

## 唯一执行者如何推进

| 当前事实 | 同一Agent的动作 | 留言 |
|---|---|---|
| 新授权Issue、缺计划 | 一次读取需求/依赖，规划后直接开发 | 源码写入前Issue保存完整PLAN_READY |
| 创建PR | 核验真实PR并双向关联 | Issue PR_CREATED、PR PR_LINKED各一次，崩溃只补缺失一半 |
| 推送并自检、查询CI | 同轮复用需求、HEAD、Diff分析与检查结果 | PR一个聚合TASK_CHECKPOINT |
| 下轮CI仍等待且无变化 | 查询变化后结束 | 不发等待心跳，不重分析Diff |
| 当前HEAD出现源码失败 | 汇总日志和预算，同轮切回03 | 修复前必要pending意图；真实推送后confirmed结果 |
| 可能合并 | 实时重核全部门禁与原有授权 | 必要意图/结果；未满足且原因未变不重复记录 |
| 看板/验收/日志变化 | 使用本轮证据同轮处理，可聚合一个文档PR | 结果并入业务检查点，不为每阶段写回执 |

不要把表中阶段重新建成独立Agent或模型调用。一个执行上下文里取得的事实供后续步骤使用，只有关键事实变化或写入前才重核对应实时证据。

## 迁移与异常演练

- 旧01至09仍有活跃运行：不启用竞争写入，核验实际停止和原生互斥后迁移。
- 旧计划/CI/恢复留言完整：保留原分支、PR、版本和预算，00检查点引用历史comment ID，旧to不再派发。
- PR已创建但关联写一半：查真实PR再补缺失关联，不新建PR。
- pending动作后崩溃：查真实提交/run/merged结果，未知则阻塞重试，不假设未执行。
- 新HEAD或新attempt：旧成功失效，预算延续；未变的其他分析可复用。
- 关键留言编辑/删除或预算断链：阻塞相关写入，不以缓存绕过。

这些用例尚未真实执行，实际结果记录到[test-cases.md](test-cases.md)。
