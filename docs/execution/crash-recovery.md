# BAM 断点恢复与外部副作用协议 v1.6

## 不变量
GitHub 远端事实优先；SQLite 不得覆盖新 HEAD。任何外部变更使用 `operation_key`，不能因网络超时或进程重启盲重放。每个 Task 写入 lease 包含 `owner, fencing_epoch, expires_at`。

## 六阶段副作用流程
1. 获取操作前远端对象（repo/PR/HEAD/Project 字段）。
2. SQLite 事务中持久化 Intent + expected SHA + operation_key。
3. 再验证 lease epoch、策略版本、暂停/撤销与操作前条件。
4. 发起 GitHub/API/Git 写入。
5. 用远端回读确认真实副作用（object ID、HEAD、合并 SHA）。
6. 在 SQLite 同事务提交 outbox Confirmed、Checkpoint 与 Event。

| 故障位置 | 恢复动作 | 严禁 |
|---|---|---|
| 创建 PR 前崩溃 | 查同 branch、Issue 关联现有 PR | 盲新建 |
| 创建 PR 后 API 响应丢失 | 查 GitHub 原生关系/branch | 再创建第二个 PR |
| Push 后崩溃 | fetch 远端真实 branch head | reset --hard / force push |
| CI 尚未完成 | 继续轮询真实 Run | 本地启动测试 |
| 本地 lease 过期 | 检查活动 Worker 与远端，再接管 | 抢占活跃进程 |
| Merge 请求结果未知 | 查 GitHub 是否已经 merged | 再次提交 Merge API |
| 用户撤销授权 | 待合并立即停，检查原授权有效期 | 旧授权继续执行 |
| main 验证失败 | 保持 VERIFY_MAIN/BLOCKED，报告证据 | 自动关闭 Issue |

## 跨进程并发
首版单实例部署；第二进程检测有效 instance lock 后安全退出。SQLite lease 仅保证单库调度一致，不等于可安全实现分布式主控；不得宣称 HA。

## 恢复点测试
CR-01 启动前；CR-02 acquire lease 后；CR-03 git commit 后；CR-04 push 后；CR-05 Create PR 远端成功响应丢失；CR-06 CI pass 但本地未落库；CR-07 Merge 后响应丢失；CR-08 Issue Close 后重启；CR-09 数据库中断迁移；CR-10 旧 HEAD 状态文件恢复。每项至少执行重放两次，在 GitHub Actions 中跑模拟并观察状态和副作用数量。