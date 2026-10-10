# BurnCloud Agent Manager — 细粒度模块设计与实施需求

> **全局 P0 强制策略：** [本地零编译、零测试；测试源码提交 GitHub 后仅由 Actions CI 执行](./bam-local-execution-policy.md)。本策略覆盖所有“默认禁止”或“只禁止全量”的旧措辞，不允许 Agent/配置豁免。

**版本：v1.1（细化补充稿）**  
**上位文档：** [BAM PRD v1.0](./bam-prd.md)
**代码级设计：** [BAM v1.2 — Rust 架构、SQLite Schema、GitHub API 与恢复协议](./bam-implementation-v1.2.md)  
**状态：** 需求与实现设计，非现有功能声明。

## 0. 约束与验收原则

本文件细化原 PRD 的 M01–M16。原有 P0/P1 优先级、AC-01…AC-10、23 项交付物继续有效，**不得因本文件缺漏而删减原要求**。GitHub Issue、PR、Projects 和 Actions 是远端事实来源；SQLite 负责本地持久执行状态。强制禁止在本地运行任何编译与测试；没有单独 Web 看板；不操作 chatgpt.com Web 界面。

**绝对禁止：** 未获用户明确授权自动 Merge；使用不属于最新 HEAD 的 CI 证据；未经验证把 SKIPPED/MISSING 的 Required Check 认作 PASS；两个 Agent 同时写同一个任务分支；覆盖用户未提交数据；把 Agent 的自然语言判断当作最终验收证据。

**公共错误与结果：** 所有模块输出 `Result<T, BamError>`；错误携带 `code, category, retryable, task_id, source`。质量结论为 `PASS / FAIL / BLOCKED / UNKNOWN`；证据缺失时为 UNKNOWN 而非 PASS。外部写操作必须“持久化意图→远端核对→执行→远端核验→提交结果”，重启后先对账再续做。

## 1. 模块细分清单

### M01 — GitHub 认证与连接

**输入/输出契约：** `AuthConfig → CapabilityReport / GitHubClient`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M01-A 凭证提供者 | gh auth、环境变量、系统凭证管理器按配置选一；不得向 Agent 传凭证 | 401/过期立即冻结写操作 | 无凭证/错误账户/轮换测试 |
| M01-B 权限探测 | 分别确认 repo read/write、Projects read/write、Actions/Checks、Merge 权限 | 403 与二次限流区别处理 | 缺一个权限也能具体定位 |
| M01-C HTTP/GraphQL 客户端 | 统一限流预算、Retry-After、超时、分页、request ID | 429 退避、网络故障恢复 | 故障注入不得重复产生写操作 |
| M01-D 最小授权域 | Worker 使用无 merge 能力凭证，Merge Controller 独立授权域 | Agent 请求 merge 被拒绝 | 检查配置/进程环境无 merge secret |
| M01-E 凭证脱敏 | 配置、异常、日志、审计不得输出 token | 日志扫描发现 token 应失败 | 模拟 HTTP 错误响应包含 token 时脱敏 |

### M02 — Projects V2 同步

**输入/输出契约：** `ProjectSnapshot → managed FieldPatch`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M02-A 元数据发现 | GraphQL 获取 Project Node ID、字段类型、Field ID、选项 ID | 字段重命名/删除不得误写 | 未知字段返回 SchemaMismatch |
| M02-B 条目分页 | 读取 Issue、PR、Draft Item 并保存 cursor | 分页重复和中断时增量去重 | 千条 Item 完整枚举 |
| M02-C 字段写入 | 只能修改明确管理的字段，比较远端最新值 | 人工改动与自动操作冲突时停止覆盖 | 非受管字段保持原样 |
| M02-D 内容关联 | 通过 Node ID 及 Project Item ID 做稳定映射 | 不得仅匹配标题 | 重复运行后没有重复 Item |
| M02-E 初始化规划 | bootstrap --dry-run 展示字段新增和选项变化 | 未经明确允许不可创建/重命名字段 | 只读运行零远端副作用 |
| M02-F 周期对账 | 轮询增量同步+定期完整扫描 | API 不可用保留旧状态但不能假定新状态 | 遗漏事件可自动恢复 |

### M03 — Issue 发现及准入

**输入/输出契约：** `ProjectItem + Issue → TaskCandidate 或 RejectReason`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M03-A 任务过滤 | 项目 Ready、agent-ready 标签、允许仓库、未关闭 | Paused/Blocked/Closed 不自动执行 | 过滤表驱动测试 |
| M03-B 规格提取 | 读取标题、正文、标签、assignee、验收条件 | 没有明确验收条件转 Needs Specification | 空验收任务不得派工 |
| M03-C 父子及依赖 | 优先 GitHub 原生关系，补充明确引用；父任务聚合 | 跨仓库依赖未查证不认已完成 | 父 #734 不被错误作为单一任务 |
| M03-D 现有 PR 解析 | Development 关系、closing 引用、head 分支及本地记录交叉核验 | 关系模糊时阻塞创建新 PR | 已有 Draft PR 无重复 |
| M03-E 内容安全 | Issue/评论/代码注释是低信任数据，不能覆盖控制策略 | 要求跳过 CI/合并/窃密的文本无效 | 注入测试不改变能力 |

### M04 — 依赖调度和队列

**输入/输出契约：** `TaskGraph + Budget → ScheduleDecision + Lease`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M04-A 依赖图 | 有向边记录前置 Issue，支持环检测、缺失节点 | 环输出具体路径且不执行 | 多分支 DAG 测试 |
| M04-B 就绪计算 | 前置需完成可信 DONE/可证明外部依赖 | 仅 PR 创建或 CI 通过不解锁 | 依赖未满足时零 Agent |
| M04-C 优先级 | CI 修复 > 冲突 > 现有 PR > 新 Ready Issue；考虑 P0-P3 | 不能无限饿死低优先级 | 混合负载调度测试 |
| M04-D 并发预算 | 默认两个 Agent，允许 1–8，WAIT_CI 释放执行槽 | 超限禁止领取 | CPU/内存资源阈值压力测试 |
| M04-E 唯一写锁 | 按 repo/Issue/PR/head branch 建事务租约和 fencing epoch | Lease 过期需远端复核才接管 | 两个进程同时抢任务仅一个成功 |
| M04-F 暂停取消 | Pause 不再领新任务，Cancel 清理已托管子进程 | 恢复时重新核对状态 | 故障与优雅停止测试 |

### M05 — Coding Agent 执行器

**输入/输出契约：** `AgentTask → AgentRunResult`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M05-A Adapter Trait | 提供 start/resume/repair/stop 契约；首版实接 Codex CLI | 未知版本/缺 CLI 返回明确错误 | 不可用假实现替代真实进程 |
| M05-B 命令调用 | 参数数组非 shell 拼接；受控 worktree cwd | 路径有空格/中文也可运行 | 进程退出码与信号捕获 |
| M05-C 指令构造 | 任务、允许范围、验收、CI 证据、最高权限规则分层 | Issue 文本不可覆盖最高规则 | 注入测试 |
| M05-D 会话记录 | 按 task/attempt 保存 session id、日志、运行时长与费用若可测 | 崩溃后依据检查点恢复 | 模拟 kill 后能继续 |
| M05-E 结果验证 | 核实 diff、commit、push、最新 SHA，不以 Agent 文本报告验收 | 无变更但称完成→FAIL | 结构化输出畸形测试 |
| M05-F 安全隔离 | 令牌隔离、预算限制、日志清洗、命令拦截 | 尝试写策略文件/merge 被拒绝 | 恶意子进程测试 |

### M06 — Git Worktree 和 PR 写入

**输入/输出契约：** `Task + RepoSnapshot → Worktree + Commit/PR`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M06-A 仓库身份 | remote URL + GitHub repo ID，main 最新 HEAD | 不能把 fork 当主仓库 | 错误仓库拒绝 |
| M06-B 工作区分配 | 每个 task 唯一路径，记录所有权；不共享 main | dirty/untracked/未知目录不可覆盖 | 人工作业零数据损失 |
| M06-C 已有分支接管 | 先找已有 PR 再安全 checkout 对应 head | fork 无 push 权限进入 Blocked | 已有 PR 不产生新 PR |
| M06-D 变更检查 | git diff --check、范围白名单、敏感路径过滤 | 不允许提交超范围变更 | 拒绝路径穿越和策略篡改 |
| M06-E 提交与推送 | 记录 hook 策略与 commit SHA，禁止 force push | 远端非快进拒绝覆盖 | 模拟并发 push |
| M06-F PR Upsert | 先远端查重，新 PR 默认 Draft，描述包含真实关联 | 网络超时后查远端而非盲重试 | PR 唯一性 |
| M06-G 冲突与清理 | 确定性冲突可修；仅清理自己拥有且干净的 worktree | 不能用 reset --hard 丢弃用户代码 | 有未提交文件拒绝删除 |

### M07 — 本地资源与 Hook

**输入/输出契约：** `CommandIntent → CommandPermit`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M07-A 零编译零测试门禁 | 任何本地编译、cargo check/build/test/clippy/run 与任意单元/集成/E2E 测试均强制禁止 | shell/IDE/hook/Wrapper 均不得绕过；违反即阻断 | CI 中运行完整模拟拦截矩阵 |
| M07-B 轻量检查 | 允许 git diff/status、diff --check 等 | 本地不执行远端完整质量门禁 | 确保只产生轻量进程 |
| M07-C Hook 发现 | 检测 core.hooksPath、pre-commit、commit-msg 内容/副作用 | 不能默默全局禁用开发者 hook | Windows/Linux hook 测试 |
| M07-D 独立提交路径 | Agent 局部 hook 策略需审计理由，远端检查必须补齐 | 不允许 CI 因 skip 静默变绿 | 日志保留跳过原因 |
| M07-E 资源控制 | 最大 Agent 数、CPU/内存阈值 P1、取消进程树 | 达到压力阈值不再启动新任务 | 资源限制测试 |

### M08 — GitHub Actions CI 证据

**输入/输出契约：** `PR HEAD + RequiredPolicy → CiEvidenceSet`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M08-A Required Policy 解析 | 读取真实适用 Branch Protection/Rulesets 的 check name 和来源 | 无权限读取时 UNKNOWN 不放行 | 策略缺失不能默认成功 |
| M08-B SHA 绑定 | 保存 current head/base、run id、check_run id；区分 merge ref | 任何新 HEAD 使旧结论无效 | 旧绿灯不能通过 |
| M08-C 状态规范化 | queued/in progress/completed 和 success/failure/skipped/cancelled/timed_out | SKIPPED/MISSING 不能作为 Required PASS | 所有状态表驱动测试 |
| M08-D 实际执行证据 | 核实 test、fmt、clippy、deny、适用的 PostgreSQL 检查是否真正执行 | workflow 总体成功不能替代子项 | 跳过 PG 拒绝 |
| M08-E 日志取证 | 按真实 run/job/step URL 获取失败日志、脱敏摘要 | 不能混入其他 HEAD | 并发 Run 匹配测试 |
| M08-F Runner 故障 | runner offline/quota/网络故障与代码错误分型 | 基础设施失败不触发盲改源码 | Runner 不可用故障注入 |
| M08-G Watcher | 60 秒默认可调，ETag/退避/水位 | Pending 不重复触发修复 | 429 与长时间等待测试 |

### M09 — CI 失败闭环

**输入/输出契约：** `FailureEvidence → RepairAttempt 或 Blocked`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M09-A 错误分类 | compiler/test/fmt/clippy/deny/PG/runner/env/unknown | 未分类不能猜测具体修改 | 故障类型测试 |
| M09-B 错误指纹 | 标准化诊断码、路径、主要堆栈，持久化 hash | 同错连续默认至多 3 次 | 重复失败预算测试 |
| M09-C 适用性判定 | 只有关联当前 HEAD 且可修代码问题才分派 Agent | 环境问题不改业务源码 | Runner 故障零无关 commit |
| M09-D 修复上下文 | 提供相关 job 日志、历史尝试、影响范围、当前 HEAD | 旧 run 不得充作现有错误 | 并发更新测试 |
| M09-E 安全审查 | 阻止删测试/降严格度/新增未经授权 lint suppress | 违规提交不能标修复成功 | 测试删除攻击模拟 |
| M09-F 重测 | 推送新 SHA、等待全新 CI、重新验收 | 同 SHA Pending 不重复 push | 完整失败→修复→成功闭环 |
| M09-G 预算耗尽 | 同故障 3 轮、总任务 12 轮默认上限 | 进入 BLOCKED_RETRY_LIMIT 而非自旋 | 第 4 次不再启动同错修复 |

### M10 — PR 生命周期

**输入/输出契约：** `PrSnapshot + Gates → PrTransition`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M10-A 发现/创建 | 已有 PR 复用，否则 Draft PR | 多候选关系不确定则阻塞 | 同 Issue 只能一次新建 |
| M10-B 描述同步 | 以真实 diff、测试记录、风险和 Issue 关联更新 | 不能写虚构证据 | PR 内容与变更一致 |
| M10-C CI 与冲突 | 每次新 HEAD 重新读 checks、mergeability | 冲突/旧证据不能转 Ready | 推送新 commit 反向转换 |
| M10-D Review 状态 | 处理 Changes Requested、dismissed、批准范围 | 阻塞 Review 不放行 | Review 变化测试 |
| M10-E Ready Gate | 完整 M11 PASS 才标 Ready for Review | Draft 不因 CI 单独 PASS 转 Ready | 无验收证据时维持 Draft |
| M10-F 合并等待 | 只等待 M12 授权或检测用户外部 Merge | 不得主动调用 Merge | 无授权始终 Awaiting Merge |
| M10-G 合后收尾 | 验证真实合入 main、主分支检查、更新项目、关闭 Issue | 主分支未通过不关闭 | 外部合并与重复事件测试 |

### M11 — 验收控制器

**输入/输出契约：** `Criteria + Evidence → AcceptanceReport`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M11-A 条件拆分 | 每项 Issue Acceptance 编号、期望证据和来源 | 模糊条件进入 UNKNOWN/人工验收 | 不可臆测 PASS |
| M11-B 变更范围 | 检查 diff 路径、调用方迁移、遗留代码清理 | 缺少证据阻塞 | 真实文件/符号证明 |
| M11-C CI 硬门禁 | 复用 M08 对最新 HEAD 的 Required Checks 和实际运行证明 | 缺失/跳过/陈旧均拒绝 | SHA 替换测试 |
| M11-D Review 与冲突 | 复核实际保护规则、阻塞 Review、合并状态 | 有冲突返回非 PASS | 竞态测试 |
| M11-E 结构化证据 | 每项记录 claim、URL/ID、SHA、抓取时间、验证方法 | 不可把 AI 文字当硬证据 | 伪造引用被拒绝 |
| M11-F 报告输出 | JSON + Markdown，verdict PASS/FAIL/BLOCKED/UNKNOWN，固定 reason codes | 任何必需 UNKNOWN 不 PASS | JSON schema round-trip |

### M12 — 合并授权与独立控制器

**输入/输出契约：** `AuthorizedPrSha + LiveGates → MergeOutcome`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M12-A 授权登记 | 用户独立发起、默认单 PR 单 HEAD 单次有效，记录 actor/时间/过期 | 模糊文字不授予权限 | 无授权零 merge |
| M12-B 隔离凭证 | Coding Agent 和 Manager Worker 无 Merge API 能力 | Agent 不得访问授权存储写接口 | 恶意 Agent 请求被拒绝 |
| M12-C 授权撤销 | 撤销生效立即停止待合并流程 | 并发撤销有同步顺序 | 撤销竞态测试 |
| M12-D 最终核验 | Merge 前重新核实 HEAD/CI/Review/依赖/Ruleset/权限 | 任一 UNKNOWN 都拒绝 | 模拟新 SHA |
| M12-E 预期 SHA 合并 | 调用 GitHub merge API 使用 expected SHA 防竞态 | 预检后新推送合并失败 | 竞态故障注入 |
| M12-F 审计 | 保存授权 ID、actor、目标 PR、HEAD、操作结果 | 秘密值不可进入日志 | 事后可完整追查 |

### M13 — 持久化与断点恢复

**输入/输出契约：** `Events + Snapshots ↔ SQLite/External State`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M13-A Schema 与迁移 | tasks、dependencies、prs、sessions、worktrees、ci、events、checkpoints、attempts、approvals、policies、cursors | 迁移失败事务回滚 | schema version 测试 |
| M13-B Inbox 去重 | 每个资源版本/Delivery ID 唯一，先入库后消费 | 重复事件零重复副作用 | 同一事件重放百次 |
| M13-C Outbox 意图 | push/create PR/comment/merge/close 先记意图再执行后确认 | 请求成功但响应丢失需查 GitHub | 网络故障精确一次语义近似 |
| M13-D 检查点 | 状态、下一动作、head SHA、worktree、最近 run id 原子保存 | 崩溃后恢复 | 任何阶段 kill -9 测试 |
| M13-E 锁恢复 | lease epoch、心跳、TTL，回收前复核活动进程/远端 | 不能抢活跃任务 | 双实例测试 |
| M13-F GitHub 对账 | 启动时优先重新读取远端，不能以旧 SQLite 覆盖 | 新 HEAD 覆盖旧快照 | 断网恢复测试 |
| M13-G 损坏应对 | 数据库完整性检查/备份/只读诊断，允许 GitHub 重建关联 | 不能恢复的私有 session 明示丢失 | 损坏注入测试 |

### M14 — 状态机与 Guard

**输入/输出契约：** `State + Event + Evidence → TransitionDecision`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M14-A 固定状态 | BACKLOG READY RUNNING WAIT_CI FIX_CI REVIEW READY_FOR_REVIEW AWAITING_MERGE MERGED VERIFY_MAIN DONE BLOCKED PAUSED CANCELLED | 未知状态拒绝序列化 | 枚举覆盖测试 |
| M14-B CI 子状态 | QUEUED IN_PROGRESS SUCCESS FAILURE CANCELLED TIMED_OUT SKIPPED MISSING 独立存储 | 不可混用业务任务状态 | 映射边界测试 |
| M14-C 转换 Guard | WAIT_CI→REVIEW 需当前 HEAD PASS；REVIEW→READY_FOR_REVIEW 需 M11 PASS | 缺证据不允许跳跃 | 合法非法转换表 |
| M14-D 合并转换 | AWAITING_MERGE→MERGED 需要外部真实 Merge 或 M12 确认 | 文本声称 merged 无效 | 未授权转换拒绝 |
| M14-E 完成转换 | VERIFY_MAIN→DONE 需 main 验证；终态不被轮询自动复活 | 主分支失败不关 Issue | 终态重放测试 |
| M14-F 暂停与阻塞 | 保留恢复前状态和阻塞原因，恢复时重验策略/HEAD | 旧验收不自动复活 | Pause/Resume 故障测试 |
| M14-G 事件日志 | 存 from/to/event/guard/证据版本/原因码 | 转移可追溯 | 重放结果确定 |

### M15 — 事件采集和同步

**输入/输出契约：** `REST/GraphQL/Webhook → DomainEvent`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M15-A Poller | 默认 60 秒、分页、游标、增量水位、限速 | 重启后不漏变化 | 断网两小时恢复测试 |
| M15-B Webhook P1 | HTTP 签名认证、大小限制、事件类型允许列表 | 签名不合法拒绝 | 伪造 payload 测试 |
| M15-C 归一化 | Issue、PR、Review、Check、Project 字段变化映射稳定事件 | 字段缺失记 UNKNOWN | 兼容事件格式变化 |
| M15-D 去重排序 | Delivery ID/对象版本/时间序列，先持久化再入队 | 重复与乱序不能回滚最新状态 | 重放 100 次测试 |
| M15-E 完整对账 | 周期扫描当前活跃任务、开放 PR、项目条目 | 补齐漏事件、不覆盖人工字段 | 失联恢复测试 |
| M15-F 退避 | 429、二次限流、5xx、网络错误指数退避 | 不能不停刷 API | 调用频率统计 |

### M16 — CLI、可观测和服务部署

**输入/输出契约：** `Command → Structured Result`。各子模块必须实现可执行 Rust 逻辑，禁止只输出 prompt 的空实现。

| 子模块 | 开发要求 | 故障与安全分支 | 可验证验收 |
|---|---|---|---|
| M16-A 初始化 | bam init 生成 TOML/数据库/路径，不覆盖用户配置 | 已存在文件只报告 | 幂等初始化测试 |
| M16-B 自检 | bam doctor 检查 gh/Codex/git/Repo/Projects/权限 | 关键能力缺失非零退出 | 错误原因明确 |
| M16-C 主调度 | bam run 单实例运行，优雅停止/恢复 | 双进程不会双领取 | 开关机生命周期测试 |
| M16-D 观察 | bam status/tasks/task inspect/logs 输出 JSON 与人类格式 | 展示真实 SHA/run id/blocker | 无任务/异常任务测试 |
| M16-E 运维 | bam pause/resume/sync/verify/config check/project inspect | 越权操作拒绝 | 命令幂等测试 |
| M16-F 字段管理 | bam project bootstrap --dry-run，实际修改需要独立确认 | 不会未经允许改变 Project workflow | dry-run 零副作用 |
| M16-G 授权命令 | bam merge authorize --pr N --sha FULLSHA，通过可信用户交互 | Agent 无法调用即获批准 | 伪造来源测试 |
| M16-H 后台服务 | Windows 计划任务启动 P0，Linux systemd P1 | 重启后不重复建立 PR | 服务中断恢复 |
| M16-I 观测审计 | tracing JSON，task/attempt/sha/run_id 贯通，脱敏与日志轮转 | 明文 token 数为 0 | 日志扫描与导出测试 |

## 2. 跨模块公共数据契约

| 类型 | 必需字段 | 不变量 |
|---|---|---|
| RepositoryKey | github_repo_id, owner, name | 不能只凭仓库名称唯一标识 |
| IssueKey | repo_id, issue_node_id, number | 跨仓库引用不串号 |
| PrKey | repo_id, pr_node_id, number, head_sha | 每次门禁取远端最新 SHA |
| TaskRecord | task_id, issue_key, pr_key?, state, priority, lease_epoch | 同一 Issue 仅一个活动写入者 |
| Lease | resource_key, owner, epoch, expiry, heartbeat | 失效 lease 不得继续写入 |
| CiEvidence | head_sha, check_name, app_id?, run_id, conclusion, executed, evidence_url | 只验证对应 HEAD 的真实必需检查 |
| Approval | pr_key, head_sha, actor, created_at, expires_at, revoked_at? | 默认单 PR 单 SHA，不能扩权 |
| DomainEvent | event_id, source, object_key, version, received_at | 幂等去重 |
| AcceptanceReport | task_id, issue, pr, head_sha, required_checks, criteria, verdict, evaluated_at | 缺证据不得 PASS |

**SQLite 约束：** 对活动任务的 `(repo_id,issue_node_id)`、活动 PR 分支写入者、事件 delivery ID、外部操作幂等键建立唯一约束；启用 WAL 和事务。每个外部写动作需有稳定幂等键，如 `repo/issue/action/expected_sha`。

## 3. 关键状态转换守卫

| 转换 | 必备证据 | 缺失时 |
|---|---|---|
| BACKLOG→READY | Project Ready、agent-ready、无阻塞、明确 Acceptance | BLOCKED |
| READY→RUNNING | 已取得写入 lease、worktree 安全、预算允许 | 保持 READY |
| RUNNING→WAIT_CI | commit/push 成功，当前 PR 关联明确 | RUNNING 或 BLOCKED |
| WAIT_CI→FIX_CI | 与当前 HEAD 关联的失败检查、修复预算有效 | BLOCKED |
| WAIT_CI→REVIEW | 当前 HEAD 所有 required checks 实际执行并 PASS | WAIT_CI / BLOCKED |
| REVIEW→READY_FOR_REVIEW | M11 验收全部 PASS，Review/冲突无阻塞 | REVIEW |
| READY_FOR_REVIEW→AWAITING_MERGE | 再核验最新 HEAD、Reviews、CI | 返回 REVIEW/WAIT_CI |
| AWAITING_MERGE→MERGED | 外部实际合并事实或 M12 明确授权并合并成功 | AWAITING_MERGE |
| MERGED→VERIFY_MAIN | GitHub 真实 merged commit/main 关联 | BLOCKED |
| VERIFY_MAIN→DONE | main 验证通过、Issue 更新可追溯 | VERIFY_MAIN/ BLOCKED |

任何外部 Push、修改 Project 字段、Review 变化、撤销授权都触发重核验。PAUSED 恢复不得直接沿用旧 CI PASS。

## 4. 测试与验证矩阵

| 测试 ID | 所属模块 | 操作 | 必须观察到的结果 |
|---|---|---|---|
| T-01 | M01 | Token 失效后写 PR | 冻结写入并脱敏日志 |
| T-02 | M02 | 人工修改不受管字段后同步 | 该字段不被覆盖 |
| T-03 | M03/M10 | Issue 已存在 Draft PR | 不重复创建 |
| T-04 | M04/M13 | 双调度器同时领取 | 只有一个有效 lease |
| T-05 | M05 | Agent 声称完成但没有 diff | 不得 PASS |
| T-06 | M06 | 非快进 push/dirty worktree | 不强推、不丢代码 |
| T-07 | M07 | 请求任何本地编译或测试（含脚本间接调用） | 强制拒绝；所有测试由 GitHub Actions 执行 |
| T-08 | M08 | 新 HEAD 沿用旧 CI PASS | 旧证据失效 |
| T-09 | M08 | 必须 PostgreSQL 测试被跳过 | 验收失败 |
| T-10 | M09 | 同一错误连续超过 3 轮 | 停止自动修复 |
| T-11 | M10 | Changes Requested | 不能进入等待合并 |
| T-12 | M11 | Required Check 缺失 | UNKNOWN/FAIL |
| T-13 | M12 | 无明确授权请求合并 | 合并次数为零 |
| T-14 | M12 | 授权后 HEAD 变化 | 原授权无效 |
| T-15 | M13 | Create PR 成功但响应丢失重启 | 远端查重无重复 PR |
| T-16 | M14 | 非法状态跃迁 | 拒绝并记录原因 |
| T-17 | M15 | 相同事件重放 100 次 | 只有一次副作用 |
| T-18 | M16 | doctor 非法权限环境 | 报告缺失能力和非零退出 |
| T-19 | 全链路 | PR 授权合并后 main 通过 | 更新 Project 并关闭 Issue |
| T-20 | 全链路 | Agent 执行时 Pause 后恢复 | 无新领取且进度可恢复 |

以上测试是补充，**不能替换 PRD v1.0 AC-01–AC-10**。每项 P0 需求必须建立追踪矩阵：`requirement_id, module, implementation_path, test_id, evidence_url, current_head_sha, status`，区分未实现、已实现未测试、已通过和受阻。

## 5. 实施阶段与每阶段完成条件

| 阶段 | 实施范围 | 阶段退出门禁 |
|---|---|---|
| F0 | M13/M14/M16 基础骨架、Rust crate、TOML、SQLite migrations、日志 | Schema/state/CLI 单元测试通过 |
| F1 | M01/M02/M03/M15 GitHub 只读同步与对账 | Mock 分页/权限/重复事件测试通过 |
| F2 | M04/M05/M06/M07 调度、真实 Codex CLI、worktree、Draft PR | 测试仓库能真实提交、推送、PR 关联 |
| F3 | M08/M09 CI 取证与失败修复循环 | 失败→真实修复→新 SHA 通过的端到端证据 |
| F4 | M10/M11/M12 生命期、验收、合并授权 | 无授权合并为零、SHA 改变授权作废 |
| F5 | 断点恢复、Windows 长驻、完整对账与回归测试 | AC-01…AC-10 逐项有真实或受控 E2E 证据 |
| F6 | 23 项交付物与 P0 追踪审计 | 不存在伪造完成或忽略的硬门禁 |

## 6. 需要从真实环境发现的事实

- Projects #1 的真实 Field ID、选项 ID、人工状态映射；不可预先假定字段已存在。
- burncloud/burncloud 当前的 Rulesets、保护分支、Required Checks、Actions Job 名称、PostgreSQL 实际运行条件。
- Codex CLI 在 Windows 和 Linux 的非交互、session 恢复、权限与进程终止 API。
- burncloud-code 实际 Rust edition、MSRV、crate 结构及现有 Git hooks。

**文档本身不是软件交付。** 未实现、未测试或未合并的功能不得标记完成；每个 PR 经真实 CI 与完整验收后仍须等待用户合并授权。
