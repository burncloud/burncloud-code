# DeepSeek Harness 测试与对照记录

本目录专门保存 DeepSeek Harness 用于 BurnCloud GitHub 研发管理的**真实测试记录、行为基线、缺陷、证据与复测结论**，便于未来版本横向对照与回归验收。

> 这是观察和测试档案，不是放宽权限的授权书，也不替代仓库根目录 `AGENTS.md`、`docs/README.md`、CI 冻结和零本地编译/测试规则。

## 范围

- 定时巡检的无人值守行为、无变化时结束单轮。
- GitHub Issue 作者白名单：**仅自动处理 `burncloud/burncloud` 仓库中由 `rustburn` 创建的 Issue**。
- PR 合并白名单：仅允许自动合并经核验由 `rustburn` 创建、全部实际代码贡献属于 `rustburn`、关联任务获得授权且满足所有门禁的 PR；其他一律人工决定。
- 多 Agent 分支 / worktree 隔离、重复任务检测与故障恢复。
- PR 最新 HEAD 的 CI、Review、合并和关闭 Issue 的证据一致性。
- 部分实现 PR 与大型父 Issue 的独立验收、保留父 Issue 未关闭。
- 遵守现有 CI 冻结和禁止本地编译/测试的规定。

## 文件导航

- [baseline.md](baseline.md)：业务规则、首次已知事实及待验证项。
- [test-cases.md](test-cases.md)：可重复执行的测试步骤与预期结果。
- [issue-log.md](issue-log.md)：观察到的问题、状态及复测跟踪。
- [runs/README.md](runs/README.md)：每次测试记录模板与证据格式。

## 测试约定

1. 记录日期、Harness 版本/模型、任务 ID、调度类型、配置或指令版本、仓库、GitHub HEAD SHA。
2. 区分 `OBSERVED`（有证据的观察）、`REPORTED`（Harness/截图自述）、`EXPECTED`（预期）和 `UNVERIFIED`（尚未核验）。
3. 不能仅因截图显示“4/4 CI green”就写 PASS；必须验证对应 PR **当前 HEAD** 的实际必需检查和 run URL。
4. 禁止为测试而本地编译/运行测试；所有真实执行验证以 GitHub Actions 为准。
5. 严禁修改、新增或绕过任何 CI workflow / required checks。
6. 测试文档和记录可以通过独立文档 PR 提交；不要声称这代表 Harness 功能验收完成。
7. 回归结论要求附 GitHub PR、Issue、CI run 和必要日志链接；如只有截图则明确标记来源局限。
8. 避免记录 Access Token、Secrets、个人隐私、敏感日志。

## 使用方式

每次运行创建 `runs/YYYY-MM-DD-<topic>.md`，引用测试用例 ID（如 `DH-001`），填入实际步骤、证据和结果。问题编号引用 `issue-log.md`（如 `BUG-DH-001`）。新规则或行为变化先更新 `baseline.md`，再增加回归用例。评估时比较相同用例的前后两次运行记录。

## 与正式任务的关系

本目录只管理 **DeepSeek Harness 的测试记录**，不自动认领任何 Issue，也不更改作者/合并权限。发生规则冲突时，以仓库现行 P0 约束和用户明确授权为准。
