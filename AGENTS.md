# AGENTS.md — 仓库级 Agent 强制规则

本文件是 burncloud/burncloud-code 对 AI Agent 的**入口级强制规则**。适用对象包括：DeepSeek Harness 等自主开发 Harness、BAM 调度的 Coding Agent / 修复 Agent / 验收 Agent，以及任何以自动化方式修改本仓库的进程。

本文件**不取代** `docs/` 下已有的领域规范，也**不放宽**任何既有 P0 约束。它引用、强化并统一这些约束。冲突时按 §5 裁决。

---

## 1. 优先级

```
用户针对具体文件、具体改动的单独明确授权
  > 本文件 AGENTS.md
  > docs/bam-local-execution-policy.md（本地执行与 CI 冻结，P0）
  > docs/ci/quality-gates.md、docs/ci/evidence-schema.md（门禁与证据）
  > docs/architecture/*、docs/execution/*、docs/operations/*（领域规范）
  > docs/bam-prd.md > docs/bam-module-design-v1.1.md > docs/bam-implementation-v1.2.md
```

`docs/README.md` 的「冲突优先级」与「CI 冻结规则（P0）」继续有效；本文件与之一致。

---

## 2. 强制规则（P0，不可由 Agent 自行豁免）

### R1 — 自主决策模式

1. 遇到需要选择时，优先选择**最安全、可恢复、不破坏既有工作**的方案，然后继续执行，不停下来等待确认。
2. 标注为「推荐」且不违反本仓库规则的选项，直接采用。
3. 普通技术决策（命名、文件划分、测试组织、commit 拆分、模块选择、分支命名）不提问、不暂停。
4. 只有 §8 列出的情形才允许向用户提问。

### R2 — 禁止本地编译与本地测试；GitHub Actions 是唯一验证场所

**MUST NOT** 在本地执行任何编译、构建、链接、类型检查或测试执行，包括但不限于：
`cargo build`、`cargo check`、`cargo test`、`cargo clippy`、`cargo run`、`cargo bench`、`cargo nextest`、`rustc`、`rustdoc --test`、`make test`、`ctest`、`pytest`、`npm/pnpm/yarn test`、`go test`、`dotnet test`、`mvn/gradle test`，以及任何会间接启动上述命令的脚本、包装器、IDE 任务、Git hook 或子进程。

- 判定标准是**最终效果**，不是关键字匹配：`bash -c`、`powershell -Command`、`cmd /c`、`python x.py`、`make`、`just`、自定义可执行文件只要在本机执行编译或测试，一律禁止。
- 允许的本地动作：文件编辑/读取、创建测试源文件、`git status` / `git diff` / `git diff --check`、Git commit/push、查询 GitHub API 与 CI 日志、运行**已预先编译好**的产物。
- `cargo fmt` / `rustfmt` 属于不编译源码的纯格式化操作，按 `docs/bam-local-execution-policy.md` §2 允许受控使用；**不得**把格式检查当作 CI 通过，也不得由其衍生编译或测试。无法保证纯格式化时不执行。
- 验证路径只有一条：提交 → GitHub Actions 在**当前 HEAD** 上真实执行 → 读取 run/job/check 结果。CI 失败时只允许「读日志 → 改源码或测试 → commit → push → 重新等待 CI」，**禁止本地复现**。
- 对应条款：LOCAL-001、LOCAL-002、LOCAL-005、LOCAL-008、LOCAL-012。

### R3 — CI 配置冻结

**MUST NOT** 创建、修改、删除、移动、重命名或绕过任何 CI/CD 配置，包括 `.github/workflows/**`、`ci-github-hosted-base.yml`、现有 Fmt/Clippy/Test/Deny 工作流、Required Checks、触发条件、权限、路径过滤与测试命令。

- 不得以"CI 失败、测试太慢、环境不兼容、想加快合并、F0/F1 分阶段"为理由改动 CI。
- 不得为 BAM 或任意子模块新建独立 CI。
- 既有 CI 未覆盖的必要验证：记录 `BLOCKED_CI_COVERAGE`，保留缺失说明与当前 HEAD，**不得**伪装成通过。
- 需要改 CI 时，必须取得用户对**具体文件与具体改动**的单独明确授权。
- 对应条款：CI-FREEZE-001 ~ CI-FREEZE-005；LOCAL-006、LOCAL-009。

### R4 — 合并门禁（最新 HEAD 必需检查全绿才可合并）

自动合并 PR 必须**同时**满足：

1. 需求实现与验收标准已完成（不是"建了文件/建了表"）。
2. 必要测试已在 GitHub Actions 真实执行成功。
3. 所有**适用**的必需检查通过。
4. 证据绑定 PR 当前最新 HEAD；push 过新提交后旧的成功证据一律失效。
5. 无未解决的阻塞性 Review。
6. 无未解决的合并冲突。
7. 满足仓库 Ruleset / 分支保护；**不得**用管理员权限绕过。
8. PR 不含未授权的 CI 变更。
9. 不含未授权的破坏性操作。
10. 合并后核实 main 已包含对应代码。

`FAILURE`、`CANCELLED`、`TIMED_OUT`、`ACTION_REQUIRED`、`QUEUED`、`IN_PROGRESS`、`MISSING`、过期 SHA 的成功记录、未真正执行的必需测试、未核实原因的 `SKIPPED` 一律**不是**通过。

### R5 — 禁止破坏性操作与越权推送

**MUST NOT**（未经用户单独授权）：删除生产数据库或重要业务数据；删除仓库或主分支；用 force push 覆盖共享分支或他人的提交；关闭/绕过安全防护；修改组织与仓库权限；泄露 Secrets、Token、密钥或用户数据。

- **禁止 force push 覆盖其他 Agent 或开发者的成果。** 仅当分支完全由本 Agent 独占、且使用 `--force-with-lease` 作为保护时，才允许重写自己的功能分支；push 被拒绝说明远端已被他人推进，此时必须停止并重新同步，不得改用强推。
- 对应条款：M06-14；`docs/execution/crash-recovery.md`（Push 后崩溃必须 fetch 远端真实 head，严禁 `reset --hard` / force push）。

### R6 — 多 Agent 协调

1. 同一文件同一时间只允许一个写入者；发现其他 Agent 正在修改同一分支/同一文件时，**自动让出该模块**，切换到独立分支或独立 worktree，继续其他**不重叠**的任务，不提问、不争夺。
2. 禁止对同一 PR 或同一分支执行竞争性写入。
3. 优先并行无依赖冲突的任务；公共接口与数据结构优先合并。
4. 一个 Agent 失败不得阻塞其他无关模块。
5. 避免同时启动过多编译任务；CI 资源优先让给阻塞其他模块的基础 PR。
6. BAM 自身运行时仍为**单实例**（`docs/execution/crash-recovery.md`）：检测到有效 instance lock 的第二进程必须安全退出，不得宣称 HA。

### R7 — 证据与状态真实

1. 每条需求的状态只能是：`NOT_STARTED`、`IN_PROGRESS`、`LOCAL_VERIFIED`、`WAIT_CI`、`CI_FAILED`、`READY_TO_MERGE`、`MERGED`、`VERIFIED`、`BLOCKED`。
2. 只有代码已合并进 main、且满足该需求验收条件，才可标记 `MERGED`；只有通过最终验收（`docs/release/final-acceptance.md`）才可标记 `VERIFIED`。
3. **不得**把"建了表/建了文件/建了 PR"当作功能完成；不得把模式（schema）落地写成能力实现。
4. 需求追踪以 `docs/requirements/requirements-traceability.csv` 为准，更新时必须填入实现文件、测试 ID、CI job、被验证的 HEAD SHA 与 run URL。
5. `docs/ci/evidence-schema.md` 的 `PASS` 规则必须遵守：无 run/check ID、无 SHA、无实际执行证明时不得给出 `PASS`。

### R8 — 升级提问边界

只有下列情形允许向用户提问，其余一律自行决策并继续：

1. 需要**不可恢复地删除**数据、分支、仓库或生产资源。
2. 会产生**额外费用**或需要采购/开通外部服务。
3. 涉及**真实密钥**的泄露风险、轮换或需要用户提供凭据。
4. 出现**无法自主解决的重大冲突**：例如与他人的改造成不可自动调和的分歧、保护规则阻止合并且无合法路径、CI 冻结需要用户授权才能推进。

提问时必须给出：已核实事实、可选方案、推荐项与其风险、以及不决策会阻塞什么。

---

## 3. 每个功能的交付流程

1. 读取需求文档与验收标准，确认前置依赖已 DONE。
2. 检查现有代码，避免重复实现。
3. 实现（禁止占位代码、空函数、永久 TODO 冒充实现）。
4. 编写测试：正常路径、异常路径、边界、回归；测试随 PR 提交。
5. 提交并 push（规范 commit message，关联 Requirement ID / Issue）。
6. 创建或更新 PR：说明实现内容、测试结果、剩余限制、关联需求、变更文件清单。
7. 等待 GitHub Actions 在**当前 HEAD** 上真实执行。
8. 失败则读日志 → 改代码 → push → 重新等待；**禁止本地复现**。
9. 满足 R4 全部门禁后合并，并核实 main 包含该代码。
10. 更新需求追踪表、Issue 与完成证据，然后继续下一项。

---

## 4. 允许 / 禁止的本地动作速查

| 动作 | 允许 | 备注 |
|---|---|---|
| 文件编辑、读取、创建测试源文件 | ✅ | |
| `git status` / `git diff` / `git diff --check` / commit / push | ✅ | 禁止 force push 覆盖他人 |
| `gh` 查询 API、CI run、日志 | ✅ | 只读优先 |
| 运行已预编译的产物 | ✅ | 不得用待开发业务程序"顺手验证" |
| `cargo fmt` / `rustfmt` 纯格式化 | ⚠️ 受限允许 | 不得当作 CI 通过；无法保证纯格式化则不执行 |
| `cargo build/check/test/clippy/run/bench`、`rustc` | ❌ | LOCAL-001/002 |
| 任何语言的本机测试运行器 | ❌ | LOCAL-012 |
| 通过 shell/脚本/容器迂回触发编译或测试 | ❌ | LOCAL-005 |
| 修改 `.github/workflows/**` 或任何 CI 门禁配置 | ❌ | CI-FREEZE-001/002 |

---

## 5. 与既有规则的冲突裁决

| 冲突点 | 既有表述 | 本文件裁决 |
|---|---|---|
| 本地编译/测试 | 早期任务书曾对**本 Harness 会话**授予「允许本地编译、运行单元测试、Clippy、Cargo Fmt、E2E」的临时权限 | **该会话级授权已被 R2 取代并作废**：仓库 P0 策略优先，任何会话级授权都不得覆盖 `docs/bam-local-execution-policy.md`，除非用户对具体文件与具体改动重新作出单独明确授权。此前已完成的本地验证结果一律仅作开发反馈，**不得**作为 CI 证据或完成依据。 |
| 旧文档中"默认禁止本地全量测试""允许局部编译""本地执行单元测试""通过配置开放本地测试"等措辞 | 见 v1.0/v1.1/v1.2 | 沿用 `bam-local-execution-policy.md` §6 的替代规则：统一为**零本地编译、零本地测试**；"测试必须覆盖"的要求仍然有效，只是编写在本地、执行在 Actions。 |
| 自动合并 | 既有授权允许自动合并合格 PR | 与 R4 一致；R4 只把门禁写得更具体，不扩大权限。 |
| 多 Agent 并行 | `docs/execution/crash-recovery.md`：BAM 首版单实例，第二进程安全退出 | 不冲突：R6 约束的是**开发侧 Git 协作**；BAM **运行时**仍单实例。 |
| 既有 CI 未覆盖 BAM crate | `docs/ci/quality-gates.md` 已记录 `Detect Rust target` 只识别根目录 `src/main.rs`/`src/lib.rs`（CI-ADR-01） | 沿用：记录 `BLOCKED_CI_COVERAGE`，不修改 CI。 |
| `.github/workflows/bam-actual-validation.yml` 已存在 | CI-FREEZE-003 禁止为 BAM 创建独立 CI | **历史遗留不合规项**（早于本规则，由 PR #7 引入）。Agent **不得**自行删除或修改它；需用户单独裁决。当前它是该 crate 唯一真实远端验证来源，其绿色结果仍须按 R7 与真实执行日志核对后才可作为证据。 |
| 需求追踪表状态语义 | 旧表全部为 `NOT_IMPLEMENTED` | 按 R7 更新为可审计状态；仅在有合并代码 + 当前 HEAD 的真实 CI 证据时使用 `MERGED`。 |

---

## 6. 权威文档索引

- 本地执行与 CI 冻结：`docs/bam-local-execution-policy.md`
- 需求与追踪：`docs/requirements/{task-breakdown,dependency-graph,requirements-traceability.csv}`
- 架构契约：`docs/architecture/{crate-design,api-contracts,sqlite-schema,state-machine,bam-directory-ownership}.md`
- 执行与恢复：`docs/execution/{agent-protocol,local-command-policy,crash-recovery}.md`
- CI 与证据：`docs/ci/{quality-gates,test-matrix,evidence-schema}.md`
- 运维与发布：`docs/operations/{installation,recovery}.md`、`docs/release/final-acceptance.md`
- 需求基线：`docs/bam-prd.md`、`docs/bam-module-design-v1.1.md`、`docs/bam-implementation-v1.2.md`

---

**违反本文件的处理：** 立即停止相关任务、记录拒绝码（`POLICY_LOCAL_EXECUTION_DENIED` / `POLICY_CI_MODIFICATION_DENIED` / `POLICY_FORCE_PUSH_DENIED` / `POLICY_UNVERIFIED_EVIDENCE`），撤销未授权改动，并在 PR 或 Issue 中上报。
