# 任务分解与 Issue 模板 v1.3

## 分工原则
以单一可验证业务结果定义 Issue，单 Issue 只能存在一个活动写入 Agent；复杂父 Issue 不直接派工。Issue 必须有 owner/repo、模块 ID、前置依赖、允许路径、输入输出、实现步骤、GitHub CI 测试、验收和风险。若未完成验收标准的编写，则状态为 Needs Specification。

## 交付波次（均为待开发，不能当作已完成）
| 波次 | Epic | 原始模块 | 最小独立任务 | 完成门禁 |
|---|---|---|---|---|
| F0 | 运行内核 | M13/M14/M16 | 配置/状态枚举/SQLite migration/CLI doctor/日志 | 每件有对应 CI Test 与故障用例 |
| F1 | 读取端 | M01/M02/M03/M15 | gh 身份、权限、Project Schema、分页、Issue/PR Resolver、同步轮询 | Fake GitHub 401/403/429、字段不覆盖 |
| F2 | Agent | M04/M05/M06/M07 | DAG/lease、Codex Adapter、worktree、safe commit/push、零本地测试限制 | 测试仓库真实 Draft PR |
| F3 | CI | M08/M09 | Ruleset、HEAD identity、required matrix、执行证明、日志、故障指纹、repair loop | 失败→修复→最新 SHA 成功 |
| F4 | 验收 | M10/M11/M12 | PR Review、证据控制器、合并授权、main 检验、Issue closure | 无授权合并 0 |
| F5 | 运维 | M13/M15/M16 | 中断恢复、外部漂移对账、Windows Service、受控 E2E | AC-01…AC-10 全通过 |

## 102 个子模块任务的生成规则
v1.1 每个 `Mxx-A`…`Mxx-n` 是候选 Task ID。拆 Issue 前检查其是否拥有独立可审查变更与独立 CI 验收；否则在同模块内组合相邻子任务并在 PR 标注所有覆盖 ID。**不要**仅为追求数量一次创建 102 个 GitHub Issue；采用 DAG 按阶段、按已确认范围批量创建。

## Issue 正文标准模板
```md
## 身份
Requirement ID: M08-D
Module: M08
Repo: burncloud/burncloud-code
Epic: F3 / CI
Priority: P0
Blocked by: #<真实编号或 NONE>
Expected branch: bam/<issue-number>-<slug>

## 边界
Input:
Output:
Allowed paths:
Forbidden paths:
Behavior on error:
Idempotency key / expected SHA:

## 实施
- [ ] 建立真实 Adapter / 领域逻辑
- [ ] 编写对应测试单元（仅写，禁止本地执行）
- [ ] 提交并 Push 至 GitHub
- [ ] 最新 HEAD 的 CI Required Checks 实际执行并成功
- [ ] 记录可复查 Acceptance Report

## 验收
AC-1:
AC-2:
Negative tests:
Required CI jobs:
Evidence URI:
Risk / rollback:
```

## 领取/派工门禁
必须同时满足 Ready、允许 Label、依赖已达可信 DONE、Acceptance 明确、非冻结、无有效 lease；已有 PR 优先接管，不创建重复 PR。准备好任务后先在 GitHub Project 同步状态，再启动工作树。触发 CI 后释放本地 Worker 槽。

## 子任务完成定义
代码和必要测试已提交；目标 PR 当前 HEAD 的每个 Required Check 真正执行且成功；安全/迁移验收证据齐全；Ready for Review 不代表完成；只有用户授权合并、main 复验成功后才将 Issue DONE。