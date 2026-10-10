# BAM 故障排查、恢复与审计手册

## 常见状态诊断
| 症状 | 首先检查 | 安全恢复 |
|---|---|---|
| GitHub 401/403 | `bam doctor` 凭证/权限报告 | 更换凭证后重新读取远端，禁止盲写 |
| GitHub API 429 | RateLimit/Retry-After | 指数退避，不人工高频重试 |
| CI Pending 很久 | Actions Runner/队列/Run ID | WAIT_CI 并释放 Agent 槽，不能本地测 |
| CI SKIPPED | Required Policy 与 job 执行证据 | BLOCKED，修复远端 CI 配置后重新验证 |
| Push non-fast-forward | 当前远端 branch HEAD | 拉取核验，禁止 force push |
| Worktree dirty | git status 与文件所有权 | 保留用户修改，阻断清理 |
| Agent Crash | session、lease、checkpoint、outbox | 验证进程与远端后恢复 |
| PR 重复候选 | Issue Development + branch refs | 标记 Ambiguous，不盲建 |
| SQLite 迁移失败 | DB backup、schema version | 只读诊断/回滚迁移 |
| Merge outcome unknown | PR GitHub merged 状态 | 只查不重复合并 |
| Main 验证失败 | merged commit、main Required Checks | 维持 VERIFY_MAIN，不关闭 Issue |

## 恢复顺序
1. `bam pause` 停止新任务领取。
2. 导出脱敏状态快照 `bam status --json`、任务审计日志、最后确认的 GitHub Run/HEAD。
3. 冻结独立 Merge Controller 的等待队列，重新读取授权状态。
4. `bam doctor` 执行非编译非测试的权限自检。
5. 从 GitHub 重建 Issue/PR/Project/Checks 当前快照并对照 SQLite outbox。
6. 仅回收已确认过期且无活动子进程的 lease。
7. 对每个未确认外部操作按 operation_key 查询远端；不重放未知写入。
8. 由管理员确认危险冲突已经隔离，再 `bam resume`。

## 观察指标
active/blocked/awaiting_merge 数、正在运行 Agent 数、等待 CI 时长、API 429 次数、同错修复次数、过期 lease、重复 PR/Issue 次数、所需检查缺失数、本地编译/测试被拒事件数、实际本地编译/测试进程数（必须为零）、无授权合并次数（必须为零）。

## 备份与灾备
定期备份 SQLite 和配置（不包含明文 token），必要时加密；保留 audit/snapshot 的 SHA 和时间。还原后必须禁用历史授权状态并重新核验 GitHub，不得从备份中恢复可用的旧授权。严格区分 GitHub 可重建状态和本地不能自动恢复的 Agent Session。

## 故障演练
DR-01 Kill 进程重启；02 GitHub 断网/限流；03 Token 轮换；04 SQLite损坏；05 CI Runner 故障；06 合并授权撤回与 HEAD 改变；07 Push 冲突；08 两进程争锁。所有演练在 GitHub CI 或隔离测试仓库执行，绝不在本地编译、跑单测或破坏生产 main。