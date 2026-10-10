# BAM SQLite 数据模型与一致性规范

完整 DDL 建议见 [v1.2 数据库设计](../bam-implementation-v1.2.md)。本文件对迁移、约束、失效处理和审计提出要求；所有 DDL 须作为版本化 `migrations/` 文件实施，不能只在文档中存在。

## 必需实体
| 表 | PK/Unique | 外键/规则 | 状态与恢复含义 |
|---|---|---|---|
| tasks | task UUID；活动 (repo_id,issue_node_id) 唯一 | state 受固定枚举约束 | 核心任务 |
| task_dependencies | task_id + 前置 repo/node 唯一 | 必须有 task | 依赖 DAG |
| task_prs | task + pr_node_id 唯一 | 唯一活跃 branch writer | PR 关联 |
| agent_sessions | session ID 唯一 | task_id | 会话/轮次 |
| worktrees | owner path 唯一 | task_id, lease epoch | 工作目录所有权 |
| leases | resource_key 唯一 | owner/epoch/TTL | 防止并发写入 |
| ci_runs | run ID + task 唯一 | head/merge SHA | 远端运行 |
| ci_checks | task + context + SHA + 来源 | run ref | 必需检查证据 |
| event_inbox | event_key 唯一 | payload digest | 重复事件去重 |
| operation_outbox | operation_key 唯一 | 目标/expected SHA | 副作用补偿 |
| checkpoints | task_id 唯一 | last/next action | 恢复点 |
| repair_attempts | attempt UUID 唯一 | task + fingerprint | 修复预算 |
| merge_authorizations | auth ID 唯一 | repo/pr/head/actor/expiry/revoked | 一次性授权 |
| task_events | event UUID 唯一 | task_id | 审计日志 |
| sync_cursors | resource_key 唯一 | updated_at | 增量同步 |
| policies | version/hash 唯一 | 不可由 Agent 写入 | 生效策略快照 |

## 事务要求
初始化启用 WAL、foreign_keys、busy_timeout。Migration 有 schema_version，先备份再运行，失败事务回滚。所有状态转换 + 对应 event 在同一事务；lease 分配与任务领取在同一原子单元。SQLite 不能原子提交远端 GitHub 动作，必须使用 outbox + 重读远端的可恢复模式，严禁承诺严格 exactly once。

## 状态漂移
当本地 HEAD、PR 关联、check 结论与 GitHub 远端不同时，保存 Conflict 事件，远端当前状态优先。未知合并结果进入 UNKNOWN 并查询，不得再执行合并。SQLite 损坏时进入只读诊断，保持所有外部写入禁止，恢复前不得重建授权为 active。

## 测试
DB-01 migration 初次执行及重复执行；DB-02 事务中间崩溃回滚；DB-03 双写者抢 lease；DB-04 过期 epoch 写入被拒；DB-05 inbox 重放 100 次；DB-06 outbox API 成功但响应丢失；DB-07 备份恢复；DB-08 foreign key 保护；均由 Actions 执行。