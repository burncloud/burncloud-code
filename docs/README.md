# BAM 文档导航与权威关系

BAM 是本地代码编辑、GitHub Projects 管理与 GitHub Actions 远端验证的自主开发管理器。**本地禁止任何编译和测试**，详见 [强制策略](./bam-local-execution-policy.md)。

## 阅读顺序
1. [产品需求 v1.0](./bam-prd.md)：目标、M01–M16、P0 验收、10 项 E2E。
2. [模块细化 v1.1](./bam-module-design-v1.1.md)：102 个子模块。
3. [代码级设计 v1.2](./bam-implementation-v1.2.md)：建议 Rust/SQLite/API 设计，非已实现功能。
4. [本地零编译零测试 P0](./bam-local-execution-policy.md)：高优先级不可豁免规则。
5. 本目录下的专项实施规格：任务、接口、数据、执行、CI、验收、安全、运维。

## 专项规格索引
| 领域 | 文档 | 用途 |
|---|---|---|
| 任务 | [任务拆分](./requirements/task-breakdown.md)、[依赖 DAG](./requirements/dependency-graph.md)、[需求追踪表](./requirements/requirements-traceability.csv) | 将规格变为 Issue 和验收 |
| 架构 | [Crate 设计](./architecture/crate-design.md)、[接口](./architecture/api-contracts.md)、[数据库](./architecture/sqlite-schema.md)、[状态机](./architecture/state-machine.md) | 约束实施 |
| 执行 | [Agent 协议](./execution/agent-protocol.md)、[本地命令边界](./execution/local-command-policy.md)、[崩溃恢复](./execution/crash-recovery.md) | 权限和幂等 |
| CI | [质量门禁](./ci/quality-gates.md)、[测试映射](./ci/test-matrix.md)、[证据 Schema](./ci/evidence-schema.md) | 当前 HEAD 实际测试 |
| Projects | [字段同步](./projects/projects-sync.md) | 保留既有人工流程 |
| 运维 | [安装](./operations/installation.md)、[恢复](./operations/recovery.md) | 部署、回滚、诊断 |
| 发布 | [最终验收](./release/final-acceptance.md) | 发布判定 |

## 冲突优先级
本地零编译零测试、未经明确单独授权严禁修改 CI、用户合并授权、GitHub 最新 HEAD 证据、工作区数据保护为不可削弱的全局 P0 约束；旧文件中出现“默认禁止全量测试”等宽松措辞时按此执行。其余按 PRD 上位意图→模块设计→细节设计处理，冲突时提交 ADR 而非默默选择。本文所列目录、类型和流程是**设计要求**；未经 CI 验证不可标记已实现。

## CI 冻结规则（P0）
开发 Agent 严禁擅自修改、新建、删除任何 `.github/workflows/**` 文件或相关 CI 门禁配置，包括 `ci-github-hosted-base.yml`。不得为 BAM/F0/F1 创建额外 CI。若现有 CI 缺少测试覆盖，记录 `BLOCKED_CI_COVERAGE` 并报告，不得私自修改 CI，更不得宣称测试通过。仅在用户对具体 CI 变更作出单独明确授权后才允许例外。完整条款见 [CI 配置冻结](./bam-local-execution-policy.md#7-ci-配置冻结p0--默认绝对禁止)。

## 变更管理
每个实现 PR 必须在描述中列出 Requirement IDs、Test IDs、CI Job / Run ID、当前 HEAD SHA、被修改的文档；所有新需求要同步追踪 CSV 和相应测试矩阵。文档 PR 只能宣称文档完成，不能宣称系统可用。