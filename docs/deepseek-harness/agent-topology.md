# DeepSeek Harness Agent Topology

## 总体模型

```
Scheduler
    |
    v
Master Orchestrator
    |
    +-- Issue Scanner
    +-- Task Planner
    +-- Coding Agent
    +-- PR Reviewer
    +-- CI Guardian
    +-- Merge Controller
    +-- Project Manager
    +-- Harness Observer
    +-- Recovery Manager
```

## 设计原则

不要让一个 Agent 同时承担产品、开发、测试、发布和运维职责。

每个 Agent 必须：

- 有明确输入。
- 有明确输出。
- 有独立权限。
- 有可验证结果。

## 生命周期

```
Issue Discovery
 -> Planning
 -> Development
 -> PR
 -> CI
 -> Review
 -> Merge
 -> Acceptance
```

所有阶段必须保存状态。
