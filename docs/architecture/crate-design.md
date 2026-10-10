# BAM Rust Crate 与文件职责

## 现有项目兼容
仓库已存在 `burncloud-code` package（检查时为 Rust edition 2021，version 0.1.9）；禁止未经单独 ADR 直接重命名/替换/改坏其公开 API。推荐独立 `apps/bam` 二进制，workspace 迁移必须做兼容性审查；这仅是待实施方案。

```text
apps/bam/src/
 main.rs
 config.rs
 domain/{ids,task,policy,evidence,events,transition}.rs
 ports/{github,git,agent,storage,clock}.rs
 infra/github/{auth,projects,issues,prs,checks,actions,rulesets}.rs
 infra/git/{repo,worktree,commit,push}.rs
 infra/codex/{process,session,prompt}.rs
 infra/sqlite/{migration,repository,lease,outbox}.rs
 application/{synchronizer,scheduler,executor,ci_watcher,repair,acceptance,merge,recovery}.rs
 cli/{init,doctor,run,status,tasks,project,merge}.rs
```

## 依赖规则
`domain` 不依赖网络、存储、shell；`ports` 以 trait 定义外部调用；`infra` 仅实现 port；`application` 调度领域对象与 port；`cli` 只负责解析参数与状态可视化。不可循环依赖。Worker 不能通过任何 port 获得 Merge 权限；控制器凭证必须独立。

## 实现包分组
| 包/模块 | 负责 | 不负责 |
|---|---|---|
| domain | 状态、不变量、错误码、验收判定纯逻辑 | HTTP / GitHub 请求 |
| ports | 可 mock 的服务契约 | 具体凭证与执行 |
| github infra | API、分页、重试、证据读取 | 自行批准 merge |
| git infra | worktree、安全 push、diff | 修改共享 main |
| codex infra | 非交互会话、日志、受限执行 | 自报 CI 通过 |
| sqlite infra | migration、唯一约束、事务、lease/outbox | 自称远端权威 |
| application | 状态推进、幂等、恢复、CI 修复 | 绕过 domain guard |
| cli | 管理命令、JSON 输出 | 在本地触发编译测试 |

## 工程门禁
Rust 版本、依赖、CLI 参数以 CI 里构建证实；严禁开发机运行 cargo check/test/build/clippy/run。每个 public port 必须有 Fake 合约测试，测试写到对应 `tests` 单元，上传 GitHub Actions 执行。