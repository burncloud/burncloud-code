# 执行记录归档

本目录按次归档 Harness 的真实测试运行，文件命名建议：

`YYYY-MM-DD-<short-topic>.md`

## 单次执行模板

```markdown
# Run — <日期 / 主题>

- 日期与时区：
- 测试者 / Agent：
- Harness 版本、模型、插件：
- 调度类型（交互 / 定时）与任务 ID：
- 指令版本 / AGENTS.md SHA：
- 仓库与基准分支：
- 相关 Issue / PR：
- 最新 HEAD SHA：
- GitHub CI run URL：
- 证据等级（OBSERVED / REPORTED / UNVERIFIED）：

## 用例结果

| Case ID | 预期 | 实际行为 | 证据 | 结果 |
|---|---|---|---|---|
| DH-001 | ... | ... | ... | NOT_RUN |

## 发现问题

- 缺陷 ID / 复现步骤 / 影响范围：

## 结论与下次对照

- 通过 / 未通过 / 尚未核验：
- 与前次运行的差异：
- 待复测项：
```

如只取得截图而无法核实 GitHub 状态，请使用 `REPORTED` 与 `INCONCLUSIVE`，不得标记 `PASS`。本目录不需要也不得新增或修改 GitHub Actions 工作流。
