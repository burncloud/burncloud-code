# DeepSeek Harness 验收用例

## 目的

记录 DeepSeek Harness 自动研发流程的验收标准。

## Issue 验收

- Issue 来源明确。
- 任务权限符合规则。
- 完成条件可验证。
- 有对应 PR 或结果记录。

## PR 验收

- PR 关联正确 Issue。
- CI 使用当前 HEAD 验证。
- 不绕过分支保护。
- 自动合并必须满足作者权限规则。

## CI 验收

- 必需 Workflow 全部通过。
- 失败原因有记录。
- 不允许通过修改 CI 绕过失败。

## 定时任务验收

- 定时任务能够独立执行。
- 不依赖人工询问。
- 执行结果可追踪。
- 异常写入 issue-log.md。

## Scheduled Execution

任务名称：DeepSeek Harness 验收用例维护

执行周期：每天一次

执行模式：无人值守
