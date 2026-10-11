# BurnCloud DeepSeek Harness Autonomous R&D System

本目录定义 DeepSeek Harness 在 BurnCloud 项目中的长期无人值守研发管理规范。

## 目标

将 DeepSeek Harness 从单次 Coding Agent 转换为持续运行的研发管理系统。

执行流程：

```
Scheduler
 -> Issue Discovery
 -> Task Planning
 -> Development
 -> PR Review
 -> CI Verification
 -> Merge Control
 -> Acceptance
```

## 文档结构

```
docs/deepseek-harness/

README.md
agent-topology.md
scheduled-tasks/
```

## Agent 模型

- master-orchestrator：总调度
- issue-scanner：Issue 扫描
- task-planner：任务规划
- coding-agent：代码开发
- pr-reviewer：PR 审查
- ci-guardian：CI 监控
- merge-controller：合并控制
- project-manager：Projects 管理
- harness-observer：运行观察
- recovery-manager：故障恢复

## 执行规则

- Scheduled Tasks 文件即 DeepSeek Harness 执行任务。
- 每个任务必须包含明确目标、输入、执行动作、输出和执行周期。
- 定时任务必须无人值守运行。
- 不使用人工确认作为普通技术决策依赖。
- 所有代码验证通过 GitHub Actions 完成。

## 安全边界

- rustburn 授权范围内任务可以自动推进。
- 非授权 PR 不自动合并。
- 不修改 CI Workflow 绕过门禁。
- 不执行本地编译测试。
- 不覆盖其他开发成果。

## 子目录

- `scheduled-tasks/`：可直接复制到 DeepSeek Harness Scheduler 的任务定义。
- `agent-topology.md`：Agent 架构说明。
