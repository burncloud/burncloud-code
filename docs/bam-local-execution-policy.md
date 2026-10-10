# BAM-P0-LOCAL-000 — 本地零编译、零测试强制策略

**级别：P0 / 不可通过配置豁免**  
**范围：** BAM 调度器、Coding Agent、修复 Agent、worktree、提交钩子、Agent 所调用的脚本/子进程及 BAM 自身自动化运行路径。  
**上位原则：** GitHub Actions 是编译、单元测试、集成测试、静态质量门禁的唯一执行场所。本地只负责编辑、创建测试、检查代码差异、Commit、Push 和读取 CI 结果。

## 1. 规范性条款

| ID | MUST / MUST NOT | 可审计验收 |
|---|---|---|
| LOCAL-001 | **MUST NOT** 在本地执行任何代码编译、构建、链接、解释性测试运行或测试框架执行 | 监控 Agent 启动的全部子进程，没有编译/测试进程 |
| LOCAL-002 | **MUST NOT** 本地执行单元测试、集成测试、E2E 测试、测试发现阶段的实际执行、benchmark 或 doctest | 本地执行日志中测试次数为 0 |
| LOCAL-003 | **MUST** 将所有新增或修复的测试写入对应源码模块的测试单元或仓库既有 tests 目录，并提交 GitHub | 变更清单能对应测试文件和测试 ID |
| LOCAL-004 | **MUST** 通过 GitHub Actions CI 的实际 Job 执行测试，取得针对当前 PR HEAD 的成功证据 | 记录 run id / job id / check run / SHA / 测试实际执行结果 |
| LOCAL-005 | **MUST NOT** 通过 shell、PowerShell、batch、Makefile、justfile、脚本、容器、远程 shell 迂回触发本机编译或测试 | 包装调用及子进程策略拦截测试 |
| LOCAL-006 | **MUST NOT** 借跳过本地测试之名降低远端 CI 测试覆盖、删除测试或将 Required Check 标为可选 | CI 配置变更需通过独立策略审查 |
| LOCAL-007 | **MUST** 发现当前仓库的 pre-commit/commit-msg hook 若触发编译或测试，则为 BAM 使用受审计的、仅限 BAM 的安全提交路径；不能改变全局开发者 hook | 普通开发者 hook 不变，Agent commit 不触发测试 |
| LOCAL-008 | **MUST** 记录每次 CI 门禁的确切代码版本和测试结果；旧 HEAD、SKIPPED、MISSING、CANCELLED、TIMED_OUT 不得视为 PASS | 反例矩阵全部拒绝误绿 |
| LOCAL-009 | **MUST** 若 GitHub Actions CI 未配置对应测试，先补充正确测试 Job/矩阵再验证；不得以“未运行”作为完成依据 | 每项验收证据存在真实执行记录 |
| LOCAL-010 | **MUST NOT** 让 Agent、Issue 文本或普通 TOML 配置改变本策略；违反时停止相关任务并记录 POLICY_LOCAL_EXECUTION_DENIED | 试图改配置、注入命令、绕过控制均被拒绝 |
| LOCAL-011 | **MUST** 为 BAM 本身的构建与发布使用 GitHub Actions 产出二进制；本地仅安装/运行预构建产物 | BAM 安装流程不调用 cargo/rustc |
| LOCAL-012 | **MUST NOT** 在开发任务中本地运行语言运行时执行的测试，例如 pytest、npm test、go test 或自定义 test runner；不仅限制 Rust | 多语言测试命令覆盖验证 |

## 2. 明确禁止的命令与行为

包括但不限于：`cargo build`、`cargo check`、`cargo test`、`cargo clippy`、`cargo run`、`cargo bench`、`cargo nextest`、`cargo llvm-cov`、`rustc`、`rustdoc --test`、`make test`、`ctest`、`pytest`、`unittest`、`npm test`、`pnpm test`、`yarn test`、`go test`、`dotnet test`、`mvn test`、`gradle test`、编译包装脚本，以及会自动启动这些命令的 IDE 任务、Git hooks 或工具链代理。

禁止规则按**最终效果**判定，不仅做关键字匹配：`bash -c`、`powershell -Command`、`cmd /c`、`python script.py`、`just`、`make` 或自定义可执行程序只要在本机执行编译/测试，也一律禁止。单靠提示词或命令黑名单无法完整强制，必须将任务执行放在受控权限/进程隔离边界内，并通过子进程监督、策略检查和故障测试验证。

本地允许：文件编辑、静态内容读取、创建单元测试源文件、检查 `git status` / `git diff` / `git diff --check`、Git commit/push、查询 GitHub API/CI 日志、运行已经预编译的 BAM 管理程序。**不允许通过运行待开发业务程序来“顺手验证”。**

`cargo fmt` / `rustfmt` 属于不编译源码的格式化操作，可在技术上独立于编译测试；为消除误触发工具链与 hook 的风险，BAM 默认只允许受控的纯格式检查/格式修改，且不得由其衍生编译或测试。若无法保证纯格式化，不执行，交由 CI 完成验证。**不得把格式检查当作 CI 通过。**

## 3. 测试文件归属与 CI 执行规则

| 测试类型 | 代码存放位置 | 执行方 |
|---|---|---|
| Rust 模块单元测试 | 对应 `src` 模块 `#[cfg(test)] mod tests` 或现有项目规范位置 | GitHub Actions `cargo test` / 既有 `code test` |
| Rust 集成测试 | 对应 crate 的 `tests/` | GitHub Actions |
| 依赖/数据库测试 | 既有测试模块、测试 crate 或专用 PG Job | GitHub Actions 配置的真实 DB 环境 |
| 编译与类型检查 | 仓库现有 CI 工作流 | GitHub Actions |
| Clippy / Deny / Format | 仓库现有正式 CI 工作流 | GitHub Actions |
| BAM 自身的单元、集成与 E2E | `apps/bam/src`、`apps/bam/tests` 或经 ADR 采纳的独立目录 | GitHub Actions |

任何新的测试必须关联至少一个明确需求 ID / Issue Acceptance；路径过滤不得造成该需求的必需测试被静默 SKIPPED。针对工作区依赖需要追加测试或 CI 配置时，先提交到 GitHub，再凭 CI 的当前 SHA 成功结果闭环。

## 4. 强制执行边界

1. 调度器生成只读及文件写入许可，不向 Agent 分发任意本地执行权限。工作树默认最小化权限。
2. 对每个子进程执行目的、实际可执行文件、参数、父进程及执行路径进行监督；不认可只靠 Prompt 或过滤命令字符串的“安全控制”。
3. 对本机无法可靠实现操作系统级阻断的启动方式，必须标记 `LOCAL_ENFORCEMENT_UNVERIFIED` 并暂停自动化任务，不能声称满足强制策略。
4. 对 Agent 提交的 CI、workflow、脚本、hooks 更改做策略检查；若试图在 GitHub Actions 之外执行测试，拒绝。
5. Windows 首发必须覆盖 PowerShell、cmd、批处理、子进程树、计划任务与 IDE 工具调用；Linux 覆盖 shell、进程组和可执行权限。
6. CI 失败后仅允许“读 CI 日志 → 修改测试/业务源码 → Commit → Push → 等待 Actions”；禁止本地复现。
7. GitHub Actions 无法使用时标记 `BLOCKED_CI_UNAVAILABLE`，可以继续撰写/提交受控变更，但不能宣告验证通过或关闭 Issue。

## 5. 新增强制测试（由 GitHub Actions 执行）

| ID | 故障注入 | 合格条件 |
|---|---|---|
| AC-LOCAL-01 | Agent 直接要求 cargo check/build/test/clippy/run | 不启动本地编译测试进程 |
| AC-LOCAL-02 | Agent 通过 PowerShell/bash/cmd 包装运行 test | 实际子进程被拦截 |
| AC-LOCAL-03 | Git Hook 触发 cargo test | BAM 提交路径不会执行 hook 测试且保留开发者设置 |
| AC-LOCAL-04 | 新建 `#[cfg(test)]` 测试 | 测试文件随 commit push，GitHub CI 有真实执行记录 |
| AC-LOCAL-05 | CI 跳过新增测试或未配置 Required Check | 无法 READY_FOR_REVIEW |
| AC-LOCAL-06 | Rust 编译失败 | Agent 只基于远端错误修代码，不本地编译 |
| AC-LOCAL-07 | CI 被取消/延迟/断网 | 保持 WAIT_CI/BLOCKED，不用本地测试补救 |
| AC-LOCAL-08 | Agent 试图修改策略让 local_full_test=true | 被拒绝，不可通过普通配置豁免 |
| AC-LOCAL-09 | BAM 自身发布 | GitHub Actions 构建可安装二进制，本地安装不触发编译 |
| AC-LOCAL-10 | Agent 通过间接脚本启动真实编译器 | 拒绝执行并审计，不能绕过 |

这些验收必须在受控 CI Runner / 隔离测试环境中执行，**不得为了验证禁令而在用户本地真正启动编译/测试进程**。

## 6. 与旧文档冲突的替代规则

凡 v1.0 / v1.1 / v1.2 中出现“默认禁止本地全量编译或测试”“允许局部编译”“本地执行单元测试”“通过配置开放本地测试”等表达，统一以本文件的**零本地编译、零本地测试**硬约束替代。旧文档中“单元测试必须覆盖”的要求仍然有效，只是**编写由本地 Agent 完成，执行在 GitHub Actions 完成**。

**完成标准：** 任何 P0 特性若需要编译或测试验证，必须具备上传的对应测试单元、实际 GitHub Actions 执行记录、与最新 HEAD 相符的 required check 成功证据。仅提交测试源码或仅创建 PR 均不算完成。
