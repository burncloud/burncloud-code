# BurnCloud Agent Manager（BAM）
## 产品需求规格说明书 PRD + 技术设计 + 验收标准
**版本：v1.0**

**目标：一次性交付可运行、可测试、可长期使用的完整系统。**

---

# 第一章：项目目标

## 1.1 项目背景

BurnCloud 是一个基于 Rust 的开源项目，存在多个并行开发的 GitHub Issues 和 Pull Requests。

传统工作模式存在以下问题：

1. 需要人工管理多个 Agent 会话。
2. 需要反复提醒 Agent 继续执行。
3. 多个 Agent 同时编译 Rust 导致本地机器严重卡顿。
4. Agent 提交 PR 后容易提前停止。
5. CI 失败需要人工重新触发修复。
6. Issue、PR、Agent、CI 缺少统一状态管理。
7. Agent 重启后容易忘记之前的开发进度。
8. 已有 PR 容易被重复创建。
9. Issue 的前置依赖容易被忽视。
10. 缺少可验证的最终验收机制。

## 1.2 总体目标

开发一个名为 BurnCloud Agent Manager（BAM）的本地自主开发管理系统。

系统通过 GitHub Projects V2 管理任务，通过 GitHub Issues 获取需求，通过本地 Coding Agent 执行代码修改，通过 GitHub Actions 完成编译与测试。

系统应在无人持续监督的情况下执行：

发现任务 → 分配 Agent → 开发 → Commit → Push → PR → CI → 修复 → 验收 → 等待授权 → 合并 → 主分支验证 → 关闭 Issue → 下一个任务。

## 1.3 项目资源

| 项目 | 地址或配置 |
|---|---|
| GitHub Organization | burncloud |
| GitHub Repository | https://github.com/burncloud/burncloud |
| GitHub Projects V2 | https://github.com/orgs/burncloud/projects/1 |
| 主分支 | main |
| 初始 Coding Agent | Codex CLI |
| 默认运行平台 | Windows 11 |
| 可选运行平台 | Linux |
| 管理器开发语言 | Rust |
| 正式 CI | GitHub Actions |
| 持久化存储 | SQLite |
| 管理界面 | GitHub Projects V2 |
| 独立 Web 看板 | 不开发 |

## 1.4 核心原则

1. GitHub 是所有任务及代码状态的权威来源。
2. Projects 是唯一业务管理看板。
3. Issue 定义需求和验收标准。
4. PR 记录代码变更与审查。
5. 本地 Agent 只执行开发相关任务。
6. 正式 Rust 编译和测试在 GitHub Actions 运行。
7. AI 不得将自己的文字报告作为验收证据。
8. 所有 CI 证据必须对应正确的最新 HEAD。
9. 所有任务必须支持中断和恢复。
10. 不得重复创建已有 Issue 或 PR。
11. 不允许两个 Agent 同时写入同一个任务分支。
12. 未经用户明确授权，不得合并任何 PR。
13. GitHub Projects 字段更新不得破坏现有人工管理流程。
14. 不依赖浏览器脚本操纵 chatgpt.com。
15. 不引入无必要的大型服务、数据库或前端框架。

---

# 第二章：系统整体架构

## 2.1 架构图

```text
GitHub Projects V2
        |
        | GraphQL API / Sync
        v
+---------------------------+
| BurnCloud Agent Manager   |
|                           |
| Project Synchronizer      |
| Issue / PR Resolver       |
| Dependency Scheduler      |
| Agent Dispatcher          |
| Worktree Manager          |
| CI Watcher                |
| CI Failure Analyzer       |
| Acceptance Controller     |
| Merge Authorization       |
| Checkpoint Manager        |
+-------------+-------------+
              |
       Agent Adapter
              |
              v
        Codex CLI
              |
              v
        Git Worktree
              |
       Edit / Commit / Push
              |
              v
          GitHub PR
              |
              v
       GitHub Actions
              |
      Fmt / Test / Clippy
      Deny / PostgreSQL
              |
              v
       CI Results / Logs
              |
              v
        BAM 验收控制器
              |
       +------+------+
       |             |
      FAIL          PASS
       |             |
       v             v
    Agent 修复    等待合并授权
       |             |
       +-- 重测      v
                  User Merge
                     |
                     v
                Verify main
                     |
                     v
                 Close Issue
                     |
                     v
                 Next Task
```

## 2.2 技术栈

| 层次 | 技术 | 说明 |
|---|---|---|
| 运行时 | Rust + Tokio | 长期运行异步调度 |
| GitHub API | GraphQL + REST | Projects V2、Issue、PR、Checks、Actions |
| Git 操作 | git CLI | worktree、branch、commit、push |
| Agent | Codex CLI | 首个开发执行器 |
| Agent 扩展 | AgentAdapter Trait | 后续可适配 Claude Code 等 |
| 数据库 | SQLite | 本地检查点和任务锁 |
| 配置 | TOML | 系统配置 |
| 日志 | tracing | 结构化运行日志 |
| CLI | clap | 管理命令 |
| 错误处理 | anyhow / thiserror | 分层错误 |
| Webhook | 可选 HTTP Server | 有公网接入条件时使用 |
| CI | GitHub Actions | 所有正式 Rust 检查 |

## 2.3 部署原则

首个版本采用单机部署。

系统可以作为一个独立 Rust 进程运行。

不强制要求 Docker、Kubernetes、Redis、PostgreSQL 或独立 Web 服务。

Windows 通过后台进程或计划任务启动。

Linux 可使用 systemd。

本地 SQLite 用于检查点、锁和事件去重。

GitHub Projects 用于展示业务状态。

---

# 第三章：详细功能需求表

## 模块 M01：GitHub 身份认证与连接

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M01-01 | 支持 GitHub CLI 认证 | P0 | 能读取当前 gh 凭证 |
| M01-02 | 支持 GitHub App 认证 | P1 | 能使用 Installation Token |
| M01-03 | 支持 GitHub Token 配置 | P0 | 从环境变量或系统凭证管理器读取 |
| M01-04 | 不在配置文件保存明文 Token | P0 | 日志及配置无凭证 |
| M01-05 | 验证仓库读写权限 | P0 | 启动时报告权限 |
| M01-06 | 验证 Projects V2 权限 | P0 | 能读取项目和更新指定字段 |
| M01-07 | 自动处理 API Rate Limit | P0 | 遵守 Retry-After/Reset |
| M01-08 | GitHub API 异常自动恢复 | P0 | 断网恢复后继续同步 |
| M01-09 | Token 失效停止写操作 | P0 | 不重复失败请求 |
| M01-10 | 支持权限自检 | P0 | CLI 能输出可用能力和缺失权限 |

## 模块 M02：GitHub Projects V2 管理

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M02-01 | 读取组织 Projects #1 | P0 | 能获取 Project ID |
| M02-02 | 枚举项目所有条目 | P0 | 正确处理分页 |
| M02-03 | 识别 Issue 条目 | P0 | 获取 Issue Node ID |
| M02-04 | 识别 PR 条目 | P0 | 获取 PR Node ID |
| M02-05 | 读取项目自定义字段 | P0 | 自动发现 Field ID 与选项 |
| M02-06 | 读取 Status 字段 | P0 | 正确解析现有状态 |
| M02-07 | 读取 Priority 字段 | P1 | 支持任务优先级 |
| M02-08 | 同步任务状态 | P0 | 自动更新对应 Item |
| M02-09 | 关联 Issue 与 PR | P0 | 记录真实关联关系 |
| M02-10 | 避免重复添加 Project Item | P0 | 重复同步后数量不变 |
| M02-11 | 不覆盖人工修改的无关字段 | P0 | 仅更新受管理字段 |
| M02-12 | 支持初始化字段 | P1 | 经确认后补建缺失字段 |
| M02-13 | 支持增量同步 | P0 | 不必每次扫描整个仓库 |
| M02-14 | 支持定期完整对账 | P0 | 能纠正遗漏事件 |
| M02-15 | 不擅自修改已有 Projects 工作流 | P0 | 未授权字段保持不变 |

### 推荐 Projects 字段

| 字段 | 类型 | 示例 |
|---|---|---|
| Status | Single Select | Ready / Coding / CI Running / Review |
| Priority | Single Select | P0 / P1 / P2 / P3 |
| Agent | Text | codex-worker-01 |
| Parent Issue | Text | #734 |
| PR | Text | #862 |
| Latest HEAD | Text | commit SHA |
| Blocked By | Text | #845 |
| Risk | Single Select | Low / Medium / High |
| Merge Approval | Single Select | Required / Authorized |
| Last Activity | Date | 最近活动日期 |
| Last Error | Text | 最新阻塞摘要 |

系统必须先读取现有 Project 字段。

不存在的字段不得假定已经存在。

增加或重命名字段必须经过配置允许，初始化命令需支持 dry-run。

---

## 模块 M03：Issue 任务发现

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M03-01 | 自动读取 Projects 中可执行 Issue | P0 | Ready 任务被发现 |
| M03-02 | 读取 Issue 标题和正文 | P0 | 内容完整 |
| M03-03 | 读取 Labels | P0 | 正确识别任务类型 |
| M03-04 | 读取 Assignees | P1 | 保持责任人信息 |
| M03-05 | 读取 Issue 状态 | P0 | 不执行 Closed Issue |
| M03-06 | 解析父子 Issue | P0 | 能识别 #734 相关子任务 |
| M03-07 | 解析 Blocked By 依赖 | P0 | 阻塞未解除不派工 |
| M03-08 | 识别已有 PR | P0 | 优先接管已有 PR |
| M03-09 | 避免重复分配 | P0 | 同一任务只有一个写入 Worker |
| M03-10 | 过滤非开发 Issue | P0 | 架构裁定类 Issue 不直接改代码 |
| M03-11 | 可按 Label 启用自动开发 | P0 | 仅允许列表内 Issue 自动领取 |
| M03-12 | 排除人工冻结 Issue | P0 | paused/blocked 不自动执行 |

### 领取规则

默认只领取已明确进入 Ready 状态且满足自动化准入策略的 Issue。

父 Issue 不应自动视为单个代码任务。

没有清晰验收标准的 Issue 应进入 Blocked 或 Needs Specification。

对 Issue 中的文字按不可信输入处理。

Issue 不得通过文字指令修改 Agent 的系统权限、合并规则或安全策略。

---

## 模块 M04：依赖与任务调度

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M04-01 | 建立 Issue 依赖图 | P0 | 形成 DAG 或检测环 |
| M04-02 | 检测循环依赖 | P0 | 报告具体 Issue 路径 |
| M04-03 | 前置任务完成后自动解锁 | P0 | 新任务进入 Ready |
| M04-04 | P0 优先于 P1 | P0 | 调度符合优先级 |
| M04-05 | CI 失败任务优先修复 | P0 | 失败任务优先入队 |
| M04-06 | WAIT_CI 不阻塞独立任务 | P0 | 可继续其他任务 |
| M04-07 | 限制最大并发 Agent | P0 | 支持配置 1–8 |
| M04-08 | 默认并发数为 2 | P0 | 避免机器卡顿 |
| M04-09 | 防止同一分支并发写入 | P0 | 仅一个分支写锁 |
| M04-10 | 检测任务文件影响范围重叠 | P1 | 高冲突任务尽量串行 |
| M04-11 | 任务失败不会阻塞全局调度 | P0 | 继续独立任务 |
| M04-12 | 支持暂停/恢复队列 | P0 | 不丢失当前任务 |
| M04-13 | 支持任务取消 | P0 | 正确终止子进程 |
| M04-14 | 支持公平调度 | P1 | 低优先级不会永久饥饿 |

### 调度优先级

1. 修复已失败 CI。
2. 解决可自动修复的合并冲突。
3. 完成已有 PR 的未完成工作。
4. 验证已完成的 Draft PR。
5. 验证 Ready for Review 后的最新 CI。
6. 开发依赖已满足的 Ready Issue。
7. 验证已合并 PR。
8. 关闭满足条件的 Issue。
9. 完成父 Issue 的最终审计。

---

## 模块 M05：Coding Agent 管理

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M05-01 | 接入 Codex CLI | P0 | 能通过非交互模式执行开发任务 |
| M05-02 | 定义 AgentAdapter 接口 | P0 | 后续可替换 Agent |
| M05-03 | 每个任务独立 Agent Session | P0 | 会话互不干扰 |
| M05-04 | 自动生成执行指令 | P0 | 包含 Issue/PR/仓库约束 |
| M05-05 | 自动恢复任务上下文 | P0 | 重启后读取检查点 |
| M05-06 | Agent 输出结构化结果 | P0 | 可解析任务状态 |
| M05-07 | 支持 Agent 进程超时 | P0 | 超时被标记且可恢复 |
| M05-08 | 支持 Agent 崩溃恢复 | P0 | 不丢弃任务 |
| M05-09 | 限制每个任务资源预算 | P0 | 有最大时间与轮次 |
| M05-10 | 记录 Agent 执行日志 | P0 | 日志可按 Issue 检索 |
| M05-11 | 禁止 Agent 直接合并 | P0 | Agent 无合并执行入口 |
| M05-12 | 禁止 Agent 改变管理策略 | P0 | 策略文件受保护 |
| M05-13 | 支持重新分配 Agent | P1 | 继承既有 PR 和检查点 |

### AgentAdapter 最低接口

```rust
trait AgentAdapter {
    async fn start_task(&self, task: AgentTask) -> Result<AgentResult>;
    async fn resume_task(&self, task: AgentTask) -> Result<AgentResult>;
    async fn repair_ci(&self, task: AgentTask) -> Result<AgentResult>;
    async fn stop_task(&self, session_id: &str) -> Result<()>;
}
```

具体实现可以使用 Rust async_trait 或其他与项目风格一致的方法。

不能只实现一个打印 Prompt 的空壳 Adapter。

必须真正调用 Codex CLI 并处理退出码、超时、结果与错误。

---

## 模块 M06：Git Worktree 与代码管理

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M06-01 | 为每个任务创建独立 worktree | P0 | 目录互不干扰 |
| M06-02 | 自动获取 main 最新 HEAD | P0 | 创建基线正确 |
| M06-03 | 已有 PR 复用现有分支 | P0 | 不生成重复 PR |
| M06-04 | 自动创建新任务分支 | P0 | 命名唯一 |
| M06-05 | 自动检查工作区污染 | P0 | 不覆盖用户文件 |
| M06-06 | 自动执行 git diff | P0 | 生成变更摘要 |
| M06-07 | 自动提交代码 | P0 | Git Commit 成功 |
| M06-08 | 自动推送到 GitHub | P0 | 远端分支更新 |
| M06-09 | 自动创建 PR | P0 | PR 正确关联 Issue |
| M06-10 | 自动更新 PR 描述 | P0 | 与实际变更一致 |
| M06-11 | 检测远端分支变化 | P0 | 不覆盖他人提交 |
| M06-12 | 解决可确定的合并冲突 | P0 | 修复后重新验收 |
| M06-13 | 清理已完成 worktree | P1 | 不删除未保存变更 |
| M06-14 | 禁止 force push 覆盖他人工作 | P0 | 非授权路径被拒绝 |

### 特殊规则

Agent 不得在共享 main 工作目录直接开发。

不得使用 git reset --hard 丢弃他人未提交的工作。

不得使用强制推送解决普通分支分歧。

PR 关联优先根据 GitHub 原生关系、明确引用、head branch 和 Issue 记录综合确认，不能只依赖标题文本匹配。

---

## 模块 M07：本地资源控制

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M07-01 | 默认禁止本地全量 cargo test | P0 | Agent 不执行重型测试 |
| M07-02 | 默认禁止本地全量 cargo clippy | P0 | 不触发 Workspace 编译 |
| M07-03 | 默认禁止本地 cargo build --release | P0 | 本地不执行 Release 编译 |
| M07-04 | 允许轻量检查 | P0 | git diff --check 可执行 |
| M07-05 | 本地重型测试交给 GitHub Actions | P0 | Push 后触发远端验证 |
| M07-06 | 控制 Agent 进程并发 | P0 | 不超过配置值 |
| M07-07 | 支持 CPU/内存资源保护 | P1 | 超预算停止启动新 Agent |
| M07-08 | 禁止自动无限重试 | P0 | 存在可配置重试上限 |
| M07-09 | 支持独立 Agent 提交策略 | P0 | 本地不再重复执行完整 CI |
| M07-10 | 远端 CI 保持全部严格检查 | P0 | 不能因本地轻量化降低质量 |

### Git Hook 处理要求

自动开发环境必须识别仓库现有 pre-commit/commit-msg hook。

如果现有 hook 会强制编译，系统需要提供独立的 Agent 提交策略。

不得悄悄全局禁用开发者 hook。

如果必须跳过重型本地检查，必须记录跳过原因，并确保远端强制质量门禁完整执行。

---

## 模块 M08：GitHub Actions CI 管理

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M08-01 | 自动发现仓库实际 CI | P0 | 读取现有工作流 |
| M08-02 | 获取 PR 最新 HEAD SHA | P0 | 每次检查前更新 |
| M08-03 | 获取 Workflow Run ID | P0 | 记录真实运行 |
| M08-04 | 获取 Check Run 与 Commit Status | P0 | 不遗漏 Required Check |
| M08-05 | 读取 Job 状态 | P0 | 识别 queued/in_progress/completed |
| M08-06 | 读取失败 Job 日志 | P0 | 获取实际错误 |
| M08-07 | 识别 Fmt 结果 | P0 | 使用真实 Check 名称 |
| M08-08 | 识别 Code Test 结果 | P0 | 确认真实执行 |
| M08-09 | 识别 Clippy 结果 | P0 | 确认真实执行 |
| M08-10 | 识别 Deny 结果 | P0 | 确认真实执行 |
| M08-11 | 识别 PostgreSQL 测试结果 | P0 | 实际执行成功才通过 |
| M08-12 | 检测 SKIPPED | P0 | 必需检查不得当作通过 |
| M08-13 | 检测 CANCELLED | P0 | 不作为通过证据 |
| M08-14 | 检测 TIMED_OUT | P0 | 不作为通过证据 |
| M08-15 | 检测旧 HEAD 结果 | P0 | 不作为当前 HEAD 证据 |
| M08-16 | 检测缺失检查 | P0 | 缺失 Required Check 不通过 |
| M08-17 | 处理路径过滤 | P0 | 不允许产生虚假绿灯 |
| M08-18 | 支持 CI 运行超时 | P0 | 超时进入异常处理 |
| M08-19 | 支持 Runner 故障分类 | P0 | 不盲目修改代码 |
| M08-20 | 避免高频无效轮询 | P0 | 有退避与间隔控制 |

### CI 真实性规则

CI 通过必须满足：

1. 读取 PR 当前真实 head SHA。
2. 检查 required checks 对应的 SHA 与 GitHub 要求的上下文。
3. 检查每个必需 Job 的成功结果。
4. 检查测试实际执行，没有被静默跳过。
5. 如果新提交改变 HEAD，旧证据立即失效。
6. 如果使用 PR merge ref 测试，必须区分 merge SHA 与 PR HEAD，并确认 merge ref 包含当前 HEAD。
7. 不允许只凭 Workflow 总体 Success 推断所有必需测试通过。
8. 不允许使用旧运行记录或日志缓存冒充最新验收。

---

## 模块 M09：CI 失败自动修复

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M09-01 | 解析失败日志 | P0 | 得到失败命令及错误 |
| M09-02 | 对失败分类 | P0 | 编译/测试/lint/环境/架构 |
| M09-03 | 将错误交回原 Agent | P0 | 使用原任务上下文 |
| M09-04 | 自动修改相关代码 | P0 | 产生真实修复 Commit |
| M09-05 | 自动 Push 后重新检查 CI | P0 | 形成闭环 |
| M09-06 | CI Pending 时不重复修复 | P0 | 一个 HEAD 不重复推送 |
| M09-07 | 新错误继续修复 | P0 | 重新进入 FIX_CI |
| M09-08 | 相同错误连续失败时退避 | P0 | 不无限提交 |
| M09-09 | 环境错误不擅自改业务代码 | P0 | 正确归类 |
| M09-10 | 记录每次修复历史 | P0 | Commit、日志可追溯 |
| M09-11 | 达到重试上限暂停 | P0 | 报告明确阻塞 |
| M09-12 | 禁止通过降低测试严格度修 CI | P0 | 违规修改被拒绝 |

### 自动修复策略

默认同一故障指纹最多连续尝试三轮自动修复。

不同故障可以继续处理，但整个任务必须有总轮次与执行预算上限。

超出预算后进入 BLOCKED_RETRY_LIMIT。

只有发现新的证据或条件变化，才允许重新进入自动修复。

---

## 模块 M10：Pull Request 生命周期

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M10-01 | 自动创建 Draft PR | P0 | 初始 PR 为 Draft |
| M10-02 | 自动关联 Issue | P0 | PR 描述包含正确关联 |
| M10-03 | 读取现有 PR 状态 | P0 | Open/Draft/Merged/Closed |
| M10-04 | 自动识别已有 PR | P0 | 不重复创建 |
| M10-05 | CI 失败自动返回开发 | P0 | 状态更新 |
| M10-06 | 检查合并冲突 | P0 | 不能带冲突 Ready |
| M10-07 | CI 通过执行完整验收 | P0 | 不只依赖 CI |
| M10-08 | 验收后转 Ready for Review | P0 | 真正更新 GitHub |
| M10-09 | Ready 后重新确认 CI | P0 | 结果仍对应最新 HEAD |
| M10-10 | 检查阻塞性 Review | P0 | Changes Requested 不能放行 |
| M10-11 | 处理用户 Review 反馈 | P0 | 自动重新开发 |
| M10-12 | 等待用户合并授权 | P0 | 不擅自 Merge |
| M10-13 | Merge 前重新核验 | P0 | 防止竞态 |
| M10-14 | 合并后验证 main | P0 | 确认提交已合入 |
| M10-15 | 最终关闭对应 Issue | P0 | 仅验收完成后关闭 |

---

## 模块 M11：验收控制器

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M11-01 | 读取 Issue Acceptance | P0 | 获取完整验收文本 |
| M11-02 | 校验代码变更范围 | P0 | 不包含未批准范围 |
| M11-03 | 校验必要调用方迁移 | P0 | 有代码证据 |
| M11-04 | 校验旧代码清理 | P0 | 无无效历史空壳 |
| M11-05 | 校验 Required Checks | P0 | 当前 HEAD 全部合格 |
| M11-06 | 校验测试实际执行 | P0 | 无虚假通过 |
| M11-07 | 校验真实 PostgreSQL 测试 | P0 | 需要时必须有执行证据 |
| M11-08 | 校验结构迁移不变量 | P0 | 业务语义无擅自改变 |
| M11-09 | 校验 PR 描述 | P0 | 与实现相符 |
| M11-10 | 输出结构化 Acceptance Report | P0 | JSON + Markdown |
| M11-11 | 缺少证据不得自动 Ready | P0 | 状态保持 Review/Blocked |
| M11-12 | AI 判断不能替代硬门禁 | P0 | 程序规则独立执行 |

### Acceptance Report

必须至少包括：

- Issue 编号。
- PR 编号。
- main SHA。
- PR HEAD SHA。
- Required Checks 列表。
- 每项 CI 的 Conclusion。
- Workflow Run ID。
- 实际测试执行证据。
- PostgreSQL 测试证据。
- Review 状态。
- Mergeability。
- 尚未满足的 Acceptance。
- 最终验收结果。
- 证据生成时间。

---

## 模块 M12：合并授权与保护

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M12-01 | 默认禁止自动 Merge | P0 | 未授权 PR 不可合并 |
| M12-02 | 读取用户授权策略 | P0 | 明确授权才可执行 |
| M12-03 | 合并前验证授权范围 | P0 | 仓库/PR/时间有效 |
| M12-04 | 合并前重验 HEAD | P0 | SHA 未变化 |
| M12-05 | 合并前检查 CI | P0 | 当前 Required Checks 全绿 |
| M12-06 | 合并前检查前置依赖 | P0 | 依赖已满足 |
| M12-07 | 合并前检查 Review | P0 | 无阻塞审查 |
| M12-08 | 支持用户撤销授权 | P0 | 立即停止待合并操作 |
| M12-09 | 记录合并操作审计 | P0 | 授权与操作可追溯 |
| M12-10 | 不得修改 Ruleset 来获得合并权限 | P0 | 不绕过保护 |

### 授权模型

支持两种授权：

1. 单个 PR 的一次性用户授权。
2. 用户显式配置的限定范围预授权。

默认仅启用第一种。

不得根据 Agent 自己的判断扩大预授权范围。

即使获得预授权，也必须逐次执行完整合并门禁。

Agent 自身不持有直接合并权限；合并由独立控制器执行。

---

## 模块 M13：检查点与断点恢复

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M13-01 | SQLite 保存任务状态 | P0 | 重启不丢失 |
| M13-02 | 保存 Issue/PR/Branch/SHA | P0 | 关联关系完整 |
| M13-03 | 保存 Agent Session 信息 | P0 | 可恢复上下文 |
| M13-04 | 保存 CI Run ID | P0 | 不重复触发 |
| M13-05 | 保存最近完成动作 | P0 | 知道执行位置 |
| M13-06 | 保存下一可执行动作 | P0 | 重启继续 |
| M13-07 | 崩溃后扫描孤儿任务 | P0 | 清除过期锁 |
| M13-08 | 恢复前重新读取 GitHub | P0 | 旧检查点不能覆盖现状 |
| M13-09 | 支持幂等任务执行 | P0 | 重放事件不重复 PR |
| M13-10 | 本地损坏时可从 GitHub 重建 | P1 | 不丢失关键关联信息 |

### Checkpoint 格式

```text
BURNCloud AGENT CHECKPOINT

Issue:
PR:
Project Item ID:
Branch:
Main SHA:
PR HEAD SHA:

Task State:
Agent Session:
Last Action:
CI Run ID:
CI Fmt:
CI Test:
CI Clippy:
CI Deny:
PostgreSQL Evidence:

Current Blocker:
Next Action:
Updated At:
```

检查点必须持久保存。

重要阶段同时向 GitHub 写入可追溯记录，例如 PR Check Run、评论或明确的项目字段。

不得在每次轮询时反复发布相同评论。

---

## 模块 M14：任务状态机

任务状态必须使用固定枚举，不允许 Agent 自行创造状态。

| 状态 | 含义 | 下一状态 |
|---|---|---|
| BACKLOG | 尚未准备执行 | READY |
| READY | 可以派工 | RUNNING |
| RUNNING | Agent 正在开发 | WAIT_CI / BLOCKED |
| WAIT_CI | 等待 Actions | FIX_CI / REVIEW |
| FIX_CI | CI 失败待修复 | RUNNING / BLOCKED |
| REVIEW | 完整验收中 | RUNNING / READY_FOR_REVIEW / BLOCKED |
| READY_FOR_REVIEW | 可正式审查 | WAIT_CI / AWAITING_MERGE |
| AWAITING_MERGE | 验收合格、等待授权 | MERGED / RUNNING |
| MERGED | PR 已合并 | VERIFY_MAIN |
| VERIFY_MAIN | 主分支验收 | DONE / BLOCKED |
| DONE | 任务完成 | 终态 |
| BLOCKED | 存在外部阻塞 | READY |
| PAUSED | 用户暂停 | 恢复前状态 |
| CANCELLED | 用户取消 | 终态 |

CI 子状态需单独保存：

- QUEUED
- IN_PROGRESS
- SUCCESS
- FAILURE
- CANCELLED
- TIMED_OUT
- SKIPPED
- MISSING

不要把业务任务状态与 GitHub CI 原生结论混为一个字段。

任何状态转换必须有可验证证据。

---

## 模块 M15：GitHub 事件同步

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M15-01 | 支持定时轮询 | P0 | Windows 本地无需公网入口 |
| M15-02 | 支持 Webhook 接入 | P1 | 正确验证签名 |
| M15-03 | 监听 Issue 变化 | P0 | 下次同步能发现 |
| M15-04 | 监听 PR 变化 | P0 | 新 HEAD 可发现 |
| M15-05 | 监听 Check 变化 | P0 | 更新 CI 状态 |
| M15-06 | 监听 Review 变化 | P0 | Changes Requested 可识别 |
| M15-07 | 监听 Project 状态变化 | P0 | Ready 后可派工 |
| M15-08 | 重复事件去重 | P0 | 不重复启动 Agent |
| M15-09 | 定期完整对账 | P0 | 修复漏事件 |
| M15-10 | GitHub 不可用时安全退避 | P0 | 不产生错误合并 |
| M15-11 | 控制 API 调用频率 | P0 | 避免触发限制 |

### 同步机制

默认通过 REST/GraphQL 增量轮询。

轮询间隔可配置，默认 60 秒，并带退避机制。

在 Windows 本地没有公网入口的情况下，不强制要求 Webhook。

如果部署到长期在线服务器，可以启用 Webhook 降低延迟。

事件接收成功后先持久化，再处理状态。

---

## 模块 M16：CLI、日志与运维

| ID | 需求 | 优先级 | 验收标准 |
|---|---|---|---|
| M16-01 | bam init | P0 | 生成配置和数据库 |
| M16-02 | bam doctor | P0 | 检查 GitHub/Codex/Git/权限 |
| M16-03 | bam run | P0 | 启动调度器 |
| M16-04 | bam status | P0 | 输出运行总览 |
| M16-05 | bam tasks | P0 | 查看任务列表 |
| M16-06 | bam task inspect | P0 | 查看指定任务详情 |
| M16-07 | bam pause | P0 | 暂停新任务 |
| M16-08 | bam resume | P0 | 恢复调度 |
| M16-09 | bam sync | P0 | 手动同步 GitHub |
| M16-10 | bam verify | P0 | 重新运行验收 |
| M16-11 | bam logs | P0 | 查看指定任务日志 |
| M16-12 | bam config check | P0 | 验证配置 |
| M16-13 | bam project inspect | P0 | 检查 Projects 字段 |
| M16-14 | bam project bootstrap --dry-run | P1 | 预览字段变更 |
| M16-15 | bam merge authorize | P0 | 接受明确授权并记录 |
| M16-16 | Windows 后台启动支持 | P0 | 重启后可恢复 |
| M16-17 | Linux systemd 支持 | P1 | 可长期运行 |
| M16-18 | 结构化日志及敏感信息脱敏 | P0 | 不泄露凭证 |

---

# 第四章：内部数据模型

SQLite 至少需要以下数据表。

| 表 | 用途 | 主要字段 |
|---|---|---|
| tasks | 任务主体 | issue_id, project_item_id, state, priority |
| task_dependencies | 任务依赖 | task_id, depends_on_id, dependency_type |
| task_prs | Issue/PR 关系 | issue_id, pr_id, branch, head_sha |
| agent_sessions | Agent 会话 | session_id, task_id, adapter, state |
| worktrees | 工作区管理 | task_id, path, branch, lease_owner |
| ci_runs | CI 运行 | run_id, sha, workflow, status, conclusion |
| ci_checks | 单项检查 | run_id, check_name, status, evidence |
| task_events | 事件记录 | task_id, event_type, payload, timestamp |
| checkpoints | 任务恢复 | task_id, last_action, next_action, updated_at |
| execution_attempts | 执行轮次 | task_id, attempt, error_fingerprint |
| merge_authorizations | 授权记录 | pr_id, authorized_sha, scope, expires_at |
| policies | 策略版本 | policy_id, version, content_hash |
| sync_cursors | 增量同步 | resource, cursor, last_success_at |

### 数据约束

必须设置合理的唯一约束和索引。

同一个 Issue 的活动写入任务不得重复。

同一个 PR 的活动写入锁不得重复。

同一个事件不可重复执行有副作用的动作。

需要支持 SQLite WAL 和事务。

后台任务状态与 GitHub 状态存在差异时，以重新读取 GitHub 的结果为准，并记录冲突。

---

# 第五章：配置文件

使用单一 TOML 配置，示例：

```toml
[github]
owner = "burncloud"
repository = "burncloud"
project_number = 1
default_branch = "main"
auth = "gh"

[manager]
poll_interval_seconds = 60
max_concurrent_agents = 2
max_agents_limit = 8
max_repair_attempts_per_error = 3
max_total_attempts_per_task = 12
resume_on_startup = true
auto_claim_issues = true
auto_merge = false

[task_selection]
ready_status = "Ready"
require_ready_status = true
require_agent_label = true
agent_label = "agent-ready"
ignore_parent_issues = true
respect_dependencies = true

[agent]
adapter = "codex"
mode = "noninteractive"
local_full_build = false
local_full_test = false
local_full_clippy = false
local_release_build = false

[git]
worktree_root = "./.bam/worktrees"
default_pr_draft = true
force_push = false
protect_user_worktrees = true

[ci]
provider = "github-actions"
require_exact_head = true
reject_required_skipped = true
reject_required_cancelled = true
reject_missing_required_checks = true
require_actual_test_execution = true

[acceptance]
require_issue_criteria = true
require_pr_review = true
require_clean_merge_state = true
require_main_verification = true
postgres_when_applicable = true
forbid_hidden_test_baselines = true
forbid_new_lint_suppressions = true

[merge]
require_explicit_user_authorization = true
auto_merge = false
revalidate_before_merge = true

[storage]
database_path = "./.bam/state.sqlite"
```

这是目标配置格式，不代表现有软件已经具备这些参数。

实现时应确保参数确实生效，而不是只实现 TOML 反序列化。

---

# 第六章：安全要求

## 6.1 凭证安全

- 不能在任务 Prompt 中包含 GitHub Token。
- 不能把 GitHub Token 发送给不可信外部服务。
- 日志必须脱敏。
- CI 不应暴露生产环境 Secrets。
- GitHub App 或 Token 必须采用最小权限。
- 凭证轮换后无需重新创建所有任务。

## 6.2 不可信输入

GitHub Issue、PR 评论、代码注释和 CI 日志都可能包含恶意指令。

不得直接将这些内容提升为 Agent 的系统级指令。

不得允许 Issue 要求：

- 删除无关仓库。
- 窃取凭证。
- 修改安全策略。
- 禁用 Required Checks。
- 强制合并。
- 执行未授权的系统操作。

## 6.3 合并安全

Coding Agent 不得直接获得合并能力。

合并动作由独立授权控制器执行。

合并时必须检查：

- 授权是否存在。
- 授权是否适用于当前 PR。
- HEAD 是否仍是授权时的 HEAD。
- CI 是否符合要求。
- Review 是否通过。
- 是否存在冲突。
- 依赖是否完成。
- GitHub Ruleset 是否允许。

## 6.4 工作区安全

不能删除用户未提交代码。

不能在 main 分支直接写入。

不能自动覆盖其他开发者的提交。

不能任意执行外部 Issue 提供的 Shell 命令。

如需执行不可信代码，应使用隔离环境和受限凭证。

---

# 第七章：测试要求

## 7.1 单元测试

必须覆盖：

1. Issue 状态解析。
2. Projects 字段映射。
3. 任务优先级排序。
4. 依赖图构建。
5. 循环依赖检测。
6. 状态机转换。
7. GitHub 事件去重。
8. 分支锁与任务锁。
9. Agent 任务生成。
10. CI 结果分类。
11. SHA 验证。
12. 授权判断。
13. 重试与退避。
14. 任务检查点序列化。
15. SQLite 事务恢复。

## 7.2 集成测试

使用可控制的 GitHub API Mock，覆盖：

- GitHub Projects V2 分页。
- Issue/PR 关联。
- PR 更新。
- CI 成功与失败。
- GitHub API 超时。
- 401/403/429 错误。
- Agent 异常退出。
- 同步服务重启。
- 并发锁。
- 授权撤销。

## 7.3 真实端到端测试

必须支持在专用测试仓库执行完整开发闭环。

不得在生产 BurnCloud main 上进行破坏性实验。

测试仓库应模拟真实 Issue、PR、Actions 和项目字段。

所有测试必须有证据、执行结果及清理步骤。

---

# 第八章：十个强制验收场景

| 场景 | 操作 | 期望结果 |
|---|---|---|
| AC-01 | Project 新增 Ready Issue | 自动领取并启动 Agent |
| AC-02 | Issue 已存在 Draft PR | 复用该 PR，不重复创建 |
| AC-03 | 两个 Issue 存在依赖 | 后置任务等待前置验收 |
| AC-04 | Agent Push 后 CI 失败 | 自动读日志、修复并重新 Push |
| AC-05 | 新 Commit 产生新 SHA | 旧 CI 成功结果全部重新评估 |
| AC-06 | PostgreSQL 必需测试被跳过 | 拒绝验收，不能 Ready |
| AC-07 | 管理器运行中强制终止 | 重启后恢复状态，不重复操作 |
| AC-08 | PR CI 全部通过但未授权 | 保持 Awaiting Merge，不能合并 |
| AC-09 | 用户授权合并后 HEAD 改变 | 原授权不能用于新 HEAD |
| AC-10 | PR 合并且 main 验证通过 | 更新 Project、关闭 Issue、继续下一任务 |

以上十项必须全部通过真实或受控端到端测试。

---

# 第九章：性能与可靠性要求

| 指标 | 验收目标 |
|---|---|
| 默认并发 Agent | 2 |
| 最大可配置并发 | 8 |
| 本地全量 Rust 编译 | 默认禁止 |
| 本地全量 Rust 测试 | 默认禁止 |
| GitHub 增量轮询 | 默认 60 秒 |
| 任务状态持久化 | SQLite |
| 非预期退出恢复 | 必须支持 |
| 同一 Issue 重复领取 | 0 |
| 同一 PR 重复创建 | 0 |
| 未授权自动合并 | 0 |
| 使用旧 HEAD 通过证据 | 0 |
| 静默跳过必需测试 | 0 |
| 因 CI 等待阻塞整个队列 | 不允许 |
| 日志中泄露明文 Token | 0 |
| 不断重复相同失败提交 | 必须受重试预算限制 |

性能测试须在 Mock Agent 和真实 Agent 两种模式下分别执行。

本地资源测试不能通过启动多个 Rust 编译进程来完成。

---

# 第十章：交付物要求

AI 开发完成时，必须交付：

1. 完整 Rust 项目源码。
2. 可执行的 Agent Manager。
3. 完整 TOML 配置示例。
4. SQLite 数据库 Schema 与迁移。
5. GitHub Projects V2 适配器。
6. GitHub Issue/PR 适配器。
7. Git Worktree Manager。
8. Codex CLI Adapter。
9. Task Scheduler。
10. CI Watcher。
11. CI Repair Loop。
12. Acceptance Controller。
13. Merge Authorization Controller。
14. Checkpoint Recovery。
15. CLI 管理命令。
16. Windows 安装与后台运行说明。
17. Linux 运行说明。
18. 完整自动化测试。
19. 端到端验收脚本。
20. 使用手册。
21. 安全与权限配置文档。
22. 实际测试执行报告。
23. 已知限制和未解决问题清单。

不接受只提供架构说明、伪代码、空接口或演示截图。

---

# 第十一章：一次性开发执行指令

你是 BurnCloud Agent Manager 的自主开发工程师。