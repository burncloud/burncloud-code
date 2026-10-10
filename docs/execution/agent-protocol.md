# BAM Agent 执行协议 v1.5

## 进程与权限分区
Supervisor（读取 GitHub、控制预算）→ Agent Worker（限权编辑工作树）→ Git 提交器（校验 diff 后 push）→ CI Observer（只读）→ Merge Controller（独立用户授权凭证）。**Agent Worker 永远不能直接调用 Merge API 或修改 Policy Store。**

## 会话输入
```json
{
  "task_id":"uuid",
  "issue_key":{"repo_id":0,"number":0},
  "worktree":"managed absolute path",
  "allowed_paths":["src/**","tests/**"],
  "acceptance_ids":["M08-01"],
  "branch":"bam/issue-number-slug",
  "expected_remote_head":"full sha or null",
  "agent_budget":{"max_attempts":12,"timeout_seconds":3600},
  "local_compilation_allowed":false,
  "local_tests_allowed":false,
  "merge_allowed":false
}
```
以上 JSON 是接口示例，不是实际任务/权限证明。真实路径、SHA 和 Issue 必须由 GitHub 和受控配置发现；不得直接信任 Issue 文本。

## 执行步骤
1. 加载受保护系统策略，Issue 数据只能作为低权限任务内容。
2. 验证 allowlist、工作目录、有效 lease epoch、GitHub repo ID、Codex CLI 能力。
3. 传入目标范围、验收、失败日志与预算；不可传 token、merge credential。
4. Agent 只可编辑代码和新增相应测试单元，允许只读静态检查；**不本地编译/运行单元测试**。
5. 收集结构化输出并核实 git status/diff、文件范围和无策略篡改。
6. Git 提交器按受控 hook 路径 Commit/Push；记录远端 SHA、关联 Draft PR。
7. 释放 Agent 资源，WAIT_CI；CI fail 时附当前 SHA 的失败日志重新派发修复。
8. 同错误指纹 3 次、总任务 12 次上限后 Blocked。

## 进程安全
输入/输出按固定 JSON Schema 验证；不得直接使用 `shell=true` 字符串拼接。Windows 子进程树、PowerShell/cmd 脚本、其他 launcher 需受控；无法证明不会启动编译测试时 fail closed。Agent prompt 不能替代 OS 级执行隔离。Codex CLI 的具体非交互/恢复 flags 需根据安装版本在远端测试后固化。

## 故障与验收
AG-01 Agent Crash 可恢复；AG-02 伪造“CI 全绿”不晋级；AG-03 输出畸形停用；AG-04 同分支两个 Agent 写入阻断；AG-05 试图运行 cargo test/build/check 不产生进程；AG-06 试图改 merge policy 被拒；AG-07 existing PR 接管而不重复创建。测试由 GitHub Actions 执行。