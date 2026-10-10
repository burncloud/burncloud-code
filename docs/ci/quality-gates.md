# BAM GitHub Actions 质量门禁 v1.4

## 当前仓库实际工作流（2026-10-11 读取 main）
| 文件 | 触发条件 | 工作流调用 | 当前真实执行/风险 |
|---|---|---|---|
| `ci-github-hosted-fmt.yml` | pull_request opened/synchronize/reopened/ready_for_review | reusable base check=fmt | `cargo fmt --all -- --check` |
| `ci-github-hosted-test.yml` | 同上 | check=test | `cargo test --all-targets --all-features` |
| `ci-github-hosted-clippy.yml` | 同上 | check=clippy | `cargo clippy --all-targets --all-features -- -D warnings` |
| `ci-github-hosted-deny.yml` | 同上 | check=deny | deny 在依赖策略文件变化时执行 |
| `ci-github-hosted-base.yml` | workflow_call | 统一 Ubuntu runner | 默认 90 分钟 timeout，启用 cancel-in-progress |
| `cd-release.yml` | tag `v*` / workflow_dispatch | Release | 不能代替 PR Required Checks |

**已观察到的设计风险（不是声称 CI 已失败）：** base 的 `Detect Rust target` 只识别根目录 `src/main.rs` 或 `src/lib.rs`；如果 BAM 位于 `apps/bam` 或 workspace 的不同布局，可能错过目标。Deny 在 Cargo.toml/Cargo.lock/deny.toml/.cargo/* 未改变时有意 skip。**“成功的 Workflow”不能自动证明测试或 Deny 实际运行**；必须根据实际 Required Policy、Job/Step 和覆盖规则判定。

## 必需门禁矩阵
| 门禁 | 什么时候必须实际执行 | 最低通过证据 |
|---|---|---|
| Rust Fmt | 修改 Rust 或相关格式配置 | 对目标 workspace/crate 实际运行 fmt check |
| Rust Compile/Test | 修改 Rust 代码、测试、manifest、toolchain、CI | 目标 crate 单测/集成/必要数据库测试有真实执行记录 |
| Clippy | Rust 代码或 lint/依赖变更 | 当前 HEAD 的严格 Clippy 成功 |
| Deny | 依赖/策略变更必须执行；其他情况按批准的 path applicability 判定 | 执行成功或明确可靠的“NOT_APPLICABLE”，不能伪装成执行成功 |
| PostgreSQL | 触及真实 PG 代码路径或 Acceptance 明确要求 | 真实 PG service/job 与测试执行证明 |
| BAM Policy Guard | Agent/CLI/执行器/CI/测试策略变更 | 在 CI 中运行本地零编译零测试禁令反例测试 |
| Release Build | 发布/分发 BAM 二进制时 | 远端生成 Windows/Linux 安装产物、可追溯版本 SHA |

## 准入与可观测性
Required Checks 的官方名称和来源 **必须通过 GitHub 分支保护与 Ruleset API 动态读取**，不能把此表硬编码为 GitHub 当前保护规则。根目录已有 Workflow ≠ 一定配置为 Required；如果权限不足无法确认则 UNKNOWN。对于 GitHub merge ref，保存 `head SHA, merge SHA, base SHA`，证明 merge ref 包含最新 HEAD；每次新 push 全部重评。

## 推荐修正项（后续实现 PR，不在此文档 PR 改 CI）
CI-ADR-01 workspace/子 crate 探测改为实际 Cargo manifest discovery；CI-ADR-02 路径过滤与 Deny 策略证据采用 `RUN/POLICY_EXEMPT/UNKNOWN` 三态，豁免必须由正式策略明确授权且有证据；CI-ADR-03 真实 PG job 必须覆盖所需数据库用例；CI-ADR-04 branch protection 将所需 Checks 配成 Required 并自动审计；CI-ADR-05 BAM 二进制 CI/CD 增加可追溯发布产物。

## 自动验收判定
```text
head = fetch_current_pr_head()
required = resolve_applicable_branch_protection_and_rulesets()
if required unavailable: UNKNOWN
for each required context:
  check source/app, sha (or justified merge ref), conclusion, actual execution
  missing/skipped/cancelled/timed_out => fail/unknown, never PASS
validate tests required by touched paths/Acceptance were run
head2 = fetch_current_pr_head()
if head2 != head: RETRY verification
else if all required and applicable checks with proof: PASS
```
不能在本地补跑 CI；CI 不可用时 BLOCKED_CI_UNAVAILABLE。