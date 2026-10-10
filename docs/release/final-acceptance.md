# BAM v2.0 最终验收和发布判定

## 发布状态
此文档定义验收门槛，**并不表示 BAM 已开发完成**。发布负责人必须提交当前 PR/Release SHA、对应 Actions 检查、测试报告和可复现结果。

## P0 不可谈判门禁
| ID | 条件 | 不合格行为 |
|---|---|---|
| REL-01 | 原 PRD 全部 P0 真实实现并关联代码路径 | 以文档、stub 或截图代替代码 |
| REL-02 | 原 AC-01…AC-10 在受控 E2E 全部执行通过 | 任一缺失/跳过 |
| REL-03 | 102 子模块 P0/测试矩阵均有追踪状态与证据 | 没有证据却标 VERIFIED |
| REL-04 | 本地编译进程数 = 0、本地测试数 = 0 | 任何绕过/执行 |
| REL-05 | 测试源码提交 GitHub，对应 CI Job 对最新 HEAD 确实执行成功 | 仅写测试却未跑 |
| REL-06 | Required Policy、SHA、merge ref、review 均复核通过 | 旧绿灯/Required skipped |
| REL-07 | 未授权合并次数 = 0，旧授权不能跨 HEAD | 任何未经用户明确授权 |
| REL-08 | 工作树用户数据丢失/强制覆盖次数 = 0 | reset --hard/force push |
| REL-09 | SQLite lease/inbox/outbox 崩溃恢复无重复副作用 | 双 PR/重复 Merge |
| REL-10 | GitHub Projects 不破坏人工字段与现有工作流 | 未授权字段更新 |
| REL-11 | Windows 安装无需 Rust toolchain，BAM 二进制来自 CI | 本地 cargo build |
| REL-12 | 全部 23 项 PRD 交付物与已知限制清单齐全 | 缺失后仍宣称完成 |

## 验收执行步骤
1. 冻结待发布 HEAD 并生成 manifest、策略 hash。
2. 在 GitHub Actions 运行编译、Fmt/Test/Clippy/Deny 和适用的 PG/策略/E2E 任务；核对 Required Checks。
3. 对十个强制 AC 与 20 个补充测试输出 pass/fail/blocked/unknown 结果。
4. 注入失败：CI 旧 SHA、跳过测试、Agent 假报告、丢失 PR Create 响应、撤销授权、代码越界、编译绕过；所有安全反例应被阻断。
5. 对 Windows/Linux 预编译软件包执行下载/运行/恢复检查，确保本地零编译零测试。
6. 生成人工可读 Markdown、机器可读 JSON、以及 `requirements-traceability.csv` 记录。
7. 缺失任何 P0 证据则 BLOCKED，不允许发布“完成”。
8. PR Ready 后仍需用户授权才能合并；合并后验证 main 并关闭相关 Issue。

## 报告模板
```text
BAM RELEASE ACCEPTANCE
Version:
Commit SHA:
Required Policy Revision:
P0 Total / Verified / Blocked:
AC-01..AC-10:
CI Run IDs:
Actual test execution proof:
Local compile count: 0
Local test count: 0
Unauthorized merges: 0
Main verification:
Known limitations:
Verdict: PASS | FAIL | BLOCKED | UNKNOWN
Reviewed by:
```

## 验收完成的定义
所有 P0 功能有实现和远端执行证据，所有强制场景通过，所有授权/隔离门禁有效且审计可追溯，全部交付物已提交。缺一个硬门禁即非 PASS；禁止把等待外部结果算成完成。