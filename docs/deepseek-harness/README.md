# BurnCloud DeepSeek Harness Autonomous R&D System

本目录定义 DeepSeek Harness 在 BurnCloud 项目中的长期运行规范。

目标：将 DeepSeek Harness 从单次 Coding Agent 转变为自主研发管理系统。

## 架构原则

- 使用多个职责明确的 Agent，而不是单一超级 Prompt。
- Scheduler 负责周期触发。
- Orchestrator 负责协调。
- Coding Agent 负责代码修改。
- Reviewer / CI / Merge Agent 负责质量门禁。
- 所有状态必须可追踪、可恢复。

## Agent 列表

- Master Orchestrator：总调度
- Issue Scanner：Issue 扫描
- Task Planner：任务规划
- Coding Agent：代码开发
- PR Reviewer：PR 审查
- CI Guardian：CI 监控
- Merge Controller：合并控制
- Project Manager：Projects 同步
- Harness Observer：Harness 行为观察
- Recovery Manager：故障恢复

## 核心规则

- 仅自动处理授权任务。
- rustburn 创建并满足条件的 PR 才允许自动合并。
- 其他贡献者 PR 必须人工决定。
- 禁止本地编译和测试。
- 禁止修改 CI Workflow 绕过门禁。
- GitHub Actions 是唯一测试执行环境。

## 文档入口

- agent-topology.md：Agent 架构
- scheduled-task-overview.md：定时任务总览
- scheduled-tasks/：每个 Agent 的独立定时任务定义
- test-cases.md：验收记录
- issue-log.md：问题记录

DeepSeek Harness 应优先读取 scheduled-tasks 下的任务文件执行自动化流程。
