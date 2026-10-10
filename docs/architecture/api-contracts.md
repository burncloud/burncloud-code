# BAM API 与领域契约

## 核心标识
`RepoKey { id, owner, name }`、`IssueKey {repo_id,node_id,number}`、`PrKey{repo_id,node_id,number}`、`TaskId`、`OperationId`、`LeaseEpoch`。SHA 是完整 commit hash；对 merge ref 结果要同时保存 merge SHA、当前 PR head SHA、父提交关系证明。UTC 时间用 RFC3339。

## Port 契约表
| Port | Operation | 前置 | 返回/错误 |
|---|---|---|---|
| CredentialProvider | resolve / capabilities | 身份来源已配置 | CapabilityReport；AuthMissing/AuthExpired |
| ProjectPort | discover/read/update_managed | Project ID 权限、受控字段白名单 | ProjectSnapshot、WriteConflict |
| IssuePort | load/relationships | repo key + issue node ID | IssueSnapshot、NotFound |
| PrPort | find_related/read/create_draft/update | 权限、idempotency key、当前 HEAD | PrSnapshot、AmbiguousAssociation |
| CheckPort | policy/checks/runs/logs | 当前 PR HEAD 和适用 Ruleset | CiEvidenceSet、UnknownPolicy |
| AgentPort | start/resume/repair/stop | worktree owner + lease + budget | AgentOutcome、Timeout |
| GitPort | worktree/commit/push/inspect | worktree 是自己拥有、远端 HEAD 已读 | Diff/Commit、NonFastForward |
| StorePort | transact/inbox/outbox/lease | SQLite 事务 | CommitResult、Conflict |
| MergePort | merge(expected_sha,approval_id) | M12 全部 guards 在独立权限域通过 | MergeOutcome、ApprovalInvalid |
| ClockPort | now/monotonic | 可测试时钟 | 时间数据 |

## 函数语义
所有返回 `Result<T,BamError>`；错误携带 `code, category, retryable, context_ids`，不携带凭证；同一个 operation_key 必须引用同一个目标动作和 expected SHA；远端写后失联必须优先 Query Remote，不得重发有副作用请求。

## GitHub API 调用映射
- GraphQL Projects V2：组织 Project、字段/选项、条目游标、字段 Mutation。
- REST / GraphQL：Issue、PR、关联、审查、Rulesets/Branch Protection。
- REST Actions/Checks/Statuses：Run ID、Job ID、Check ID、检查来源 App、SHA、实际测试证据和日志。
- REST Pull Merge：**只有**独立 Merge Controller 可用，必须 expected SHA 防 TOCTOU。

API 的具体 endpoint/schema 与权限 scope 必须在实施时按实际 GitHub 文档和仓库验证；本文件不是已验证的 SDK 定义。实施例必须使用 Mock 401/403/429、分页、重复请求与网络断连测试。