# 测试分类、测试单元与 CI 映射

**关键要求：所有测试只编写在对应测试单元；本地永不执行。** 测试源码随 PR 提交 GitHub，Actions 运行并产出与最新 HEAD 关联的证据。下面路径是推荐位置，实施时按现有 crate 布局确认。

| 模块 | 测试单元建议 | 必需类型 | 关键反例 |
|---|---|---|---|
| M01 | `tests/github_auth.rs` | adapter contract | 401/403/429、token 泄露 |
| M02 | `tests/projects_sync.rs` | integration mock | 字段重命名/人工字段/分页 |
| M03 | `tests/issue_resolver.rs` | integration | 父子依赖、现有 PR、注入 |
| M04 | `tests/scheduler.rs` | unit/integration | DAG 环、并发 lease |
| M05 | `tests/codex_adapter.rs` | adapter/E2E | crash、假的“完成” |
| M06 | `tests/worktree.rs` | integration | dirty 文件、非快进 push |
| M07 | `tests/local_exec_policy.rs` | security | shell/hook 间接 cargo test |
| M08 | `tests/ci_evidence.rs` | contract/security | 新 HEAD/旧通过、skipped PG |
| M09 | `tests/repair_loop.rs` | unit/integration | 3 次预算、Runner 错误 |
| M10 | `tests/pr_lifecycle.rs` | integration | Changes Requested、base 更新 |
| M11 | `tests/acceptance.rs` | unit/integration | 缺证据仍 PASS |
| M12 | `tests/merge_guard.rs` | security/E2E | 无授权、SHA 改变、撤销 |
| M13 | `tests/recovery.rs` | fault injection | outbox 响应丢失 |
| M14 | `tests/state_machine.rs` | unit | 非法转移/重放 |
| M15 | `tests/event_sync.rs` | integration | 重复、乱序、断网 |
| M16 | `tests/cli.rs` | E2E | status JSON、双实例 |

## 测试定位规则
纯函数优先 `#[cfg(test)] mod tests` 放入对应 Rust module；跨组件测试放 crate 的 `tests/`。每项 Issue Acceptance 至少指向一个 Test ID，Test ID 能追溯到编译/测试 Job、Check Run ID、HEAD。缺映射为 BLOCKED。

## 十个 PRD 强制 E2E
AC-01 Ready Issue 领取；02 现有 Draft PR 复用；03 前置依赖；04 失败 CI 自动修复；05 新 HEAD 导致旧证据失效；06 PG Required Skip 阻断；07 崩溃恢复；08 无授权不合并；09 合并授权随 HEAD 改变失效；10 合并后验证 main 并关闭 Issue。每个场景执行环境为专用测试仓库或可控 Mock；生产 main 不运行破坏性测试。

## Required vs Applicability
不是所有任务必须执行同一数据库测试，但每个测试是否适用须依据受保护、版本化规则计算，不能由 Agent 自己判免测。实际执行、正式例外（附适用性证明）、缺失要三态区分。**Required Check 自身被 SKIPPED 时不得直接当 PASS。**

## CI 验收结构
`TestEvidence {requirement_id, test_id, test_path, job_id, run_id, head_sha, result, actually_executed, evidence_url, captured_at}`。需要证据但无法访问运行日志时 UNKNOWN；“已写测试源码”不能当“测试通过”。