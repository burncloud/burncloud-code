# BAM v1.2 — 代码级架构、数据契约与实施蓝图

**状态：设计补充，尚未实现**  
**适用：** [PRD v1.0](./bam-prd.md) 与 [模块细化 v1.1](./bam-module-design-v1.1.md)  
**目标：** 将 M01–M16 的实现细化到可分工的 Rust 模块、Schema、API、测试工件和不可绕过的安全门禁。

## 1. 现状、边界和强制架构决策

2026-10-11 读取仓库 Cargo.toml：`burncloud-code` 是现有 package，版本 0.1.9、Rust edition 2021，依赖 `burncloud-common` 与 `burncloud-core`。它不应未经评估被整体替换成 BAM 进程。v1.0 指定“Rust + Tokio/SQLite + Codex CLI、Windows 11 首发，GitHub Projects 为唯一业务看板”。本节区分**现有事实**与**推荐设计**：

- **已有事实：** 现有库 crate 的名称、版本、edition、依赖来自本仓库 Cargo.toml；当前没有证据证明 BAM CLI、SQLite 和 Agent 已实现。
- **推荐设计（待审批后实施）：** 采用独立 `bam` 二进制 crate（可置于仓库 `apps/bam`）；保留已有 `burncloud-code` package、公开 API、release 语义。引入 workspace 或目录结构调整必须先跑现有兼容性检查，并单独在实现 PR 审核。
- **禁止变更：** 不为了 BAM 修改 `burncloud/burncloud` 业务代码、已发布公共 Rust API、质量保护策略；不改 GitHub Projects 用户管理字段的语义。
- **工作目录：** BAM 自己的 `.bam/` 状态与 worktree 路径须加入 Git 忽略；任何运行时私有状态、访问令牌和 CI 日志不得提交 GitHub。
- **分层依赖：** domain 不依赖 github/process/sqlite；ports 定义抽象；infra 实现外部 API；application 编排；cli 只解析参数。

## 2. 推荐的 Rust 工程和职责

```text
burncloud-code/
  Cargo.toml                    # 保留既有 package；变 workspace 须专门迁移
  src/                          # 已有 burncloud-code 源代码，不破坏
  docs/
    bam-prd.md
    bam-module-design-v1.1.md
    bam-implementation-v1.2.md
  apps/bam/                     # 推荐：独立 package / bin (需实施 PR)
    Cargo.toml
    src/
      main.rs                   # entry + exit codes
      config.rs                 # TOML + 检查
      domain/
        ids.rs                  # typed IDs
        task.rs                 # Task/TaskState
        policy.rs               # immutable capabilities
        evidence.rs             # CI/Acceptance
        events.rs               # domain events
        transition.rs           # typed guards
      ports/
        github.rs               # remote interfaces
        agent.rs                # Codex abstraction
        git.rs                  # worktree interface
        storage.rs              # transactions / outbox
        clock.rs                # monotonic test clock
      infra/
        github/{auth,projects,issues,prs,checks,actions,rulesets}.rs
        codex/{process,session,prompt}.rs
        git/{repo,worktree,commit,push}.rs
        sqlite/{schema,repository,lease,outbox}.rs
      application/
        sync.rs                 # M02/M03/M15
        scheduler.rs            # M04
        executor.rs             # M05/M06/M07
        ci_watch.rs             # M08/M09
        acceptance.rs           # M10/M11
        merge.rs                # M12
        recovery.rs             # M13/M14
      cli/{doctor,run,status,task,project,merge}.rs
    migrations/
      0001_initial.sql
    tests/
      api_contract.rs
      crash_recovery.rs
      github_integration.rs
      authorization.rs
      acceptance_e2e.rs
```

路径是**推荐组织结构**，不能视为已存在。第一阶段先确认是否允许 workspace、现有 package 的导出符号和反向依赖；不允许时以单独 sidecar binary 仓库/独立 Cargo manifest 实施同样边界，不得强迫改现有库。

### 2.1 依赖方向与禁止循环

```text
cli -> application -> domain/ports <- infra
                       ^
                  sqlite/adapter
```

ports 只能引用 domain DTO；github adapter 不直接调用 CLI；merge controller 不可复用带有 Merge 权限的 Agent GitHub client。统一 `TaskId` 不得用 Issue number 代替（Issue number 只在仓库内部唯一）。

## 3. 领域模型与接口约定

实现时可重命名类型，但语义及门禁不得更改。所有外部对象 ID 必须保留 GitHub GraphQL Node ID、REST 数字 ID（需要时）、仓库 ID 和可展示 number；任何交叉关联不只靠标题。

```rust
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RepoKey { pub github_id: i64, pub owner: String, pub name: String }

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct IssueKey { pub repo: RepoKey, pub node_id: String, pub number: u64 }

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PrKey { pub repo: RepoKey, pub node_id: String, pub number: u64 }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verdict { Pass, Fail, Blocked, Unknown }

#[derive(Clone, Debug)]
pub struct EvidenceRef {
    pub source: String,
    pub url: String,
    pub object_id: String,
    pub head_sha: String,
    pub observed_at: String, // RFC3339 UTC
}
```

**契约细则：** SHA 必须用合法完整提交标识验证；可区分 PR head SHA 与 merge ref SHA；时间戳为 UTC RFC3339；`Unknown` 永远不是隐式 `Pass`；错误 `BamError { code, retryable, source, task_id?, message }` 不可包含 token；领域模型禁止引入 `anyhow::Error` 作为持久化状态字段。

### 3.1 Port 签名草案

```rust
pub trait GitHubReadPort {
    async fn load_issue(&self, issue: &IssueKey) -> Result<IssueSnapshot, BamError>;
    async fn load_pr(&self, pr: &PrKey) -> Result<PrSnapshot, BamError>;
    async fn load_required_policy(&self, pr: &PrKey) -> Result<RequiredPolicy, BamError>;
    async fn load_checks(&self, pr: &PrKey, head: &str) -> Result<CheckSnapshot, BamError>;
}
pub trait AgentPort {
    async fn execute(&self, task: AgentTask, budget: AgentBudget) -> Result<AgentOutcome, BamError>;
    async fn stop(&self, session: &str) -> Result<(), BamError>;
}
pub trait MergePort {
    async fn merge_if_head_matches(
        &self, pr: &PrKey, expected_head: &str
    ) -> Result<MergeOutcome, BamError>;
}
```

以上是 Rust 2021 的表达性伪签名，具体 trait async 可采用已验证的稳定 Rust 或 `async_trait`；**不得**在复制该代码前声称已可直接编译。生产代码对任何外部写请求还必须传 `OperationId` 与安全策略版本。

## 4. SQLite v1 Schema 草案

SQLite 必须启用 `PRAGMA foreign_keys=ON`、`journal_mode=WAL`、适当 `busy_timeout`。DDL 仅为迁移基线；实际落库前需设计 SQL 的 rollback/upgrade 测试并核对 GitHub 全局 ID 类型。

```sql
CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);
CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  repo_id INTEGER NOT NULL,
  issue_node_id TEXT NOT NULL,
  issue_number INTEGER NOT NULL,
  state TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 2,
  active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)),
  policy_version TEXT NOT NULL,
  observed_head_sha TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX uq_active_issue ON tasks(repo_id, issue_node_id)
  WHERE active=1;

CREATE TABLE task_dependencies (
  task_id TEXT NOT NULL REFERENCES tasks(id),
  prerequisite_repo_id INTEGER NOT NULL,
  prerequisite_issue_node_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  PRIMARY KEY(task_id, prerequisite_repo_id, prerequisite_issue_node_id)
);
CREATE TABLE task_prs (
  task_id TEXT NOT NULL REFERENCES tasks(id),
  pr_node_id TEXT NOT NULL,
  pr_number INTEGER NOT NULL,
  head_repo_id INTEGER NOT NULL,
  head_ref TEXT NOT NULL,
  head_sha TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1 CHECK(active IN(0,1)),
  PRIMARY KEY(task_id,pr_node_id)
);
CREATE TABLE leases (
  resource_key TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  fencing_epoch INTEGER NOT NULL,
  expires_at TEXT NOT NULL,
  heartbeat_at TEXT NOT NULL
);
CREATE TABLE agent_sessions (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  adapter TEXT NOT NULL,
  external_session_id TEXT,
  state TEXT NOT NULL,
  attempt INTEGER NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT
);
CREATE TABLE worktrees (
  task_id TEXT PRIMARY KEY REFERENCES tasks(id),
  owned_path TEXT NOT NULL UNIQUE,
  repo_id INTEGER NOT NULL,
  branch TEXT NOT NULL,
  lease_epoch INTEGER NOT NULL,
  state TEXT NOT NULL
);
CREATE TABLE ci_runs (
  run_id INTEGER NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  pr_head_sha TEXT NOT NULL,
  merge_ref_sha TEXT,
  workflow_id INTEGER,
  status TEXT NOT NULL,
  conclusion TEXT,
  PRIMARY KEY(run_id,task_id)
);
CREATE TABLE ci_checks (
  task_id TEXT NOT NULL REFERENCES tasks(id),
  check_key TEXT NOT NULL,
  evaluated_head_sha TEXT NOT NULL,
  run_id INTEGER,
  conclusion TEXT NOT NULL,
  execution_evidence TEXT,
  observed_at TEXT NOT NULL,
  PRIMARY KEY(task_id,check_key,evaluated_head_sha)
);
CREATE TABLE event_inbox (
  event_key TEXT PRIMARY KEY,
  object_key TEXT NOT NULL,
  version TEXT,
  payload_digest TEXT NOT NULL,
  handled_at TEXT,
  received_at TEXT NOT NULL
);
CREATE TABLE operation_outbox (
  id TEXT PRIMARY KEY,
  operation_key TEXT NOT NULL UNIQUE,
  task_id TEXT REFERENCES tasks(id),
  operation_type TEXT NOT NULL,
  expected_head_sha TEXT,
  status TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  completed_at TEXT,
  remote_result_id TEXT
);
CREATE TABLE checkpoints (
  task_id TEXT PRIMARY KEY REFERENCES tasks(id),
  last_action TEXT NOT NULL,
  next_action TEXT NOT NULL,
  snapshot_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE repair_attempts (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  error_fingerprint TEXT NOT NULL,
  attempt_number INTEGER NOT NULL,
  head_before TEXT NOT NULL,
  head_after TEXT,
  outcome TEXT NOT NULL
);
CREATE TABLE merge_authorizations (
  id TEXT PRIMARY KEY,
  repo_id INTEGER NOT NULL,
  pr_node_id TEXT NOT NULL,
  authorized_head_sha TEXT NOT NULL,
  actor TEXT NOT NULL,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  revoked_at TEXT,
  consumed_at TEXT
);
CREATE TABLE task_events (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  from_state TEXT,
  to_state TEXT,
  event_type TEXT NOT NULL,
  evidence_digest TEXT,
  created_at TEXT NOT NULL
);
CREATE TABLE sync_cursors (
  resource_key TEXT PRIMARY KEY,
  cursor_value TEXT,
  last_success_at TEXT
);
```

**实施检查：** SQLite 单实例锁 + DB lease 不等于跨机器共识；v1.2 仅保证单部署实例，第二实例只能以安全拒绝启动。所有数据库写入使用事务，外部副作用与 SQLite 不在同一个事务，必须通过 outbox 和远端回读实现可恢复的“效果至多一次”目标；不能宣称天然 exactly-once。

## 5. GitHub API 调用矩阵

| 功能 | API 类型 | 调用方向 | 幂等与失败处理 |
|---|---|---|---|
| 查看 Repo、Issue、PR | REST/GraphQL | Read | 401 停写，403 区分权限，分页 |
| Projects 字段和 Items | GraphQL | Read | Cursor 分页，Field/Option ID 发现 |
| 更新 Project Item 字段 | GraphQL mutation | Write | 只允许 managed fields，重读后写，冲突重评 |
| 关联 Issue 与 PR | GraphQL/REST + 引用解析 | Read | Native relation 优先，多候选阻塞 |
| 创建 Draft PR | REST | Write | 先查同 branch/Issue 关联，再 Create；响应丢失后查重 |
| 更新 PR/转 Ready | REST | Write | 最新 HEAD、验收 PASS 作为前置 |
| Actions runs/jobs/logs | REST | Read | run/check 身份绑定，按需分页与限流 |
| Check Runs/Commit Statuses | REST/GraphQL | Read | 检查 SHA、app/context、结论 |
| Rulesets/Branch Protection | REST/GraphQL（按实际权限） | Read | 无法枚举 required -> Unknown；fail closed |
| Reviews/Requested Changes | REST/GraphQL | Read | 请求 Merge 前重读 |
| Merge PR | REST | **独立 Merge Controller** | expected head SHA + 授权记录 + 复检全部门禁 |
| Close Issue | REST | Write | 仅主分支验收成功后；远端确认与审计 |

GitHub API 功能、请求体和权限细节属于实施时按现行文档验证的外部契约，不得用本表替代实际接口测试。重点避免把 “workflow success” 和 “required checks passed” 等同。

## 6. 操作事务与崩溃恢复协议

每个具有外部副作用的动作使用以下顺序：

1. 在 SQLite 事务中生成唯一 `operation_key` 和 outbox `PENDING`。
2. 重读远端对象，比较 repo、PR/Issue、HEAD、目标字段及策略版本。
3. 校验 lease epoch、任务暂停、允许动作集合、安全策略、资源预算。
4. 执行一次外部写操作。
5. 重新读取远端以确认真实结果和生成的对象 ID。
6. 将 outbox 标记 `CONFIRMED`，连同新检查点一次事务提交。
7. 如果步骤 4–5 期间崩溃：重启后先执行步骤 2 的远端关联搜索；**不能盲重放写操作**。

| 崩溃点 | 恢复动作 | 禁止事项 |
|---|---|---|
| 写 outbox 前 | 重建 intent 并检查远端现状 | 假设操作已成功 |
| 写 outbox 后、发 API 前 | 确认远端无效果再执行 | 跳过权限与 HEAD 检查 |
| API 成功但响应丢失 | 按唯一键/关联关系查远端 | 立即重复创建 PR/评论 |
| Push 后重启 | 获取真实 remote ref HEAD | 重置或覆盖远端分支 |
| Merge 授权后 HEAD 改变 | 授权作废，重新请求用户授权 | 沿用旧授权 |
| 主分支检查失败 | VERIFY_MAIN/BLOCKED 保持，展示原因 | 关闭 Issue 伪造完成 |

## 7. Codex CLI 调用与权限协议

**启动前：** 校验可执行文件和版本、非交互参数、仓库身份、worktree 路径、预算、任务写锁、GitHub 能力。CLI 的具体 flags、恢复命令和沙箱兼容性要由 `bam doctor` 实际探测，不硬编码未经验证的参数。

**受控输入：** system policy、Issue acceptance、允许目录、禁止 merge/secret、现有 PR branch、CI 错误证据、轮次预算、输出 JSON schema。Issue、PR 评论、代码和日志视作不可提升权限的数据。

**运行时：** 记录 Session ID、task/attempt、时间、stdout/stderr 脱敏日志；限制同时运行的 Agent，不允许 shell 字符串拼接，终止须清理 Windows/Linux 子进程树。不能仅靠提示词禁止危险行为，必要时通过独立工作目录、权限隔离和受限执行器强制门禁。

**完成后：** 比对 git diff、tracked/untracked、远端 HEAD 和提交记录；记录输出的 evidence URL；只有 GitHub 上真实生效的变更可进入 WAIT_CI。

## 8. CI 验收算法的精确定义

```text
INPUT: PR key, current policy version
pr = get_remote_pr()
head = pr.head.sha
policy = resolve_required_checks(pr.base, rulesets, branch_protection)
IF policy is unknown: RETURN UNKNOWN
checks = collect_checks_and_actions(head, applicable_merge_ref)
FOR EACH requirement IN policy:
  verify required context and expected app/source
  verify check is associated with current HEAD
  verify job/step was actually executed when execution evidence is required
  verify success, not skipped/cancelled/timed_out/missing
  IF uncertainty: RETURN UNKNOWN
  IF conclusive failure: RETURN FAIL
IF applicable PostgreSQL test lacks real execution proof: RETURN UNKNOWN/FAIL
IF latest remote PR head != head: RESTART evaluation
RETURN PASS with SHA-bound evidence
```

**CI 故障分类：** CODE（编译/单测/格式/lint/deny）、INFRA（Runner/磁盘/网络/配额）、SECURITY（保护规则/疑似绕过）、UNKNOWN。仅 CODE 可进入默认修复循环。相同指纹默认最多 3 次，总任务默认 12 次；每次 Push 之后前一 HEAD 的 CI 与验收结论即时失效。

## 9. 用户授权的不可绕过边界

- 授权须明确匹配 `repo_id, pr_node_id, head_sha`，有主体、有效期、一次性消费标记；默认无批量/宽泛授权。
- `bam merge authorize` 只接受独立可信交互/鉴权通道；Agent 的提示词、Issue 文本、PR 评论均不是授权。
- Worker/GitHub credential 无 Merge 权限；Controller 凭证不可传入 Agent 环境。
- Merge API 调用前实时复检授权未撤销、SHA、CI、Review、冲突、依赖、规则；调用时传 expected SHA；远端拒绝则不得变更保护规则重试。
- 用户在 GitHub 页面手动合并是外部事实事件，不是 BAM 合并授权；BAM 只执行主分支验证和后续收尾。

## 10. 参数与 CLI 验收

```toml
[github]
owner = "burncloud"
repository = "burncloud"
project_number = 1
auth = "gh"

[manager]
poll_interval_seconds = 60
max_concurrent_agents = 2
max_agents_limit = 8
max_repair_attempts_per_error = 3
max_total_attempts_per_task = 12
resume_on_startup = true

[task_selection]
ready_status = "Ready"
require_agent_label = true
agent_label = "agent-ready"
ignore_parent_issues = true
respect_dependencies = true

[agent]
adapter = "codex"
local_full_test = false
local_full_clippy = false
local_release_build = false

[git]
worktree_root = "./.bam/worktrees"
default_pr_draft = true
force_push = false

[ci]
provider = "github-actions"
require_exact_head = true
reject_missing_required_checks = true
require_actual_test_execution = true

[merge]
auto_merge = false
require_explicit_user_authorization = true
revalidate_before_merge = true

[storage]
database_path = "./.bam/state.sqlite"
```

**CLI MVP：** `bam init`, `doctor`, `run`, `status`, `tasks`, `task inspect`, `pause`, `resume`, `sync`, `verify`, `logs`, `config check`, `project inspect`, `project bootstrap --dry-run`, `merge authorize`。成功/失败必须定义稳定机器可读 JSON 输出和退出码，命令不能只是占位。

## 11. 测试分层、跟踪矩阵及发布条件

| 测试层 | 核心范围 | 证据 |
|---|---|---|
| 单元测试 | 状态 Guard、DAG、锁、退避、fingerprint、SHA、授权、权限策略 | cargo test 记录/测试报告 |
| 适配器合约测试 | GitHub 分页/权限、Codex 退出码、git 非快进与 dirty worktree | Fake server + 命令模拟 |
| SQLite 故障注入 | 每个副作用前/后崩溃、锁回收、DB 迁移中断、重复事件 | 重启后断言 |
| CI 反例测试 | 旧 HEAD、merge ref、SKIPPED、MISSING、同名 Checks、Runner 故障 | 反例都不得 PASS |
| 安全攻击测试 | 注入“忽略指令/合并”、伪造授权、token 泄露、写入非授权目录 | 授权 0 误用 |
| 端到端 | PRD AC-01…AC-10 + v1.1 T-001…T-020 | 每项真实/受控 E2E 记录 |

最小可机器读取的需求追踪 CSV：
```csv
requirement_id,module,implementation_path,test_id,evidence_url,head_sha,status
M12-01,M12,apps/bam/src/application/merge.rs,T-013,,,NOT_IMPLEMENTED
```

状态仅允许 `NOT_IMPLEMENTED, IMPLEMENTED_UNTESTED, VERIFIED, BLOCKED`；无真实测试运行记录不能是 VERIFIED。

## 12. 分期 PR 的验收边界

| 批次 | 交付 | 强制验收 |
|---|---|---|
| F0 | Rust binary skeleton、配置校验、领域类型、SQLite migration、状态机 | 编译与 migration/state 单测通过 |
| F1 | GitHub Auth、Projects、Issue/PR 同步、轮询与增量对账 | 401/403/429、分页与人工字段保护测试 |
| F2 | 调度器、工作树、真实 Codex Adapter、本地资源策略 | 测试仓库 Draft PR 建立、重复执行无第二 PR |
| F3 | 真实 Actions Check 映射、日志、失败自动修复 | 最新 SHA 真实 CI PASS、旧证据失效 |
| F4 | Acceptance Controller、Review、Merge Authorization | 未授权 Merge 零次、HEAD 变更使授权失效 |
| F5 | 恢复/故障注入、Windows 服务、全量 E2E | AC-01…AC-10 全部验证；P0 追踪无空缺 |

每个阶段可拆多个小 PR；一个 PR 只能声明自己确实达到的门禁；PR Ready for Review **不等于**已合并，更不等于 Issue 完成。不得在生产 `burncloud/burncloud` 的 main 分支执行破坏性 E2E。

## 13. 实现前的开放决策（必须确认或通过环境探测解决）

| 决策 ID | 问题 | 推荐默认行为 |
|---|---|---|
| ADR-001 | BAM 与已有 burncloud-code crate 怎样集成？ | 优先独立 binary，不破坏现有 lib；实施 PR 才决定 workspace |
| ADR-002 | GitHub Projects 实际 Status、Labels 和字段如何映射？ | 动态发现；缺字段不擅自创建 |
| ADR-003 | Required Checks 的实际 RuleSet 及 PG 测试是哪些？ | API 查询；无法获取则 fail closed |
| ADR-004 | Codex CLI Windows 上非交互/恢复/沙箱参数？ | 运行 doctor 探测并固定已验证版本 |
| ADR-005 | 合并授权渠道如何保护免受 Agent 伪造？ | 独立控制器与凭证域，默认仅手动单 PR 授权 |
| ADR-006 | CI 证据保留期与运行日志存储预算？ | 有界保留、敏感信息脱敏，需求由运维配置 |
| ADR-007 | 公共模型与老 crate 的兼容范围？ | 不发布破坏性 API；先做反向依赖和编译验证 |

以上为待实施的技术规范，不是已执行或已通过的事实。实现者必须逐项验证，不能静默降低安全、测试、合并要求。
