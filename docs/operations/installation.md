# BAM Windows / Linux 安装与更新 v1.9

## 核心原则
用户机器 **不得本地编译、不得运行单元测试**。安装的是 GitHub Actions 已编译、测试并发布的 BAM 二进制文件。此项目的 Release 工作流目前用于现有 `burncloud-code` crate，不能在未实施 BAM release job 前声称已有 BAM 下载产物。

## Windows 11 首发
1. 在 GitHub CI/CD 发布 BAM 的 Windows 预编译签名/校验和安装包。
2. 从可信 Release 地址获取目标版本与 SHA256；校验签名/校验和和来源。
3. 安装到用户目录/受控目录，不要求 Rust toolchain、C/C++ 编译工具或 cargo。
4. 用系统凭证管理器/gh auth 提供 GitHub 凭证；Codex CLI 安装与授权单独在 `bam doctor` 检查。
5. `bam init` 创建配置与 SQLite state，默认 2 Agent、60 秒 Poll、auto_merge=false。
6. `bam doctor` 逐项展示仓库/Projects/Codex/运行时隔离/CI/权限能力；不触发任何编译或测试。
7. `bam run` 以后台进程或 Windows 计划任务长期运行；配置单实例锁、退出恢复与日志轮转。

## Linux 可选
在 CI 构建 Linux 二进制和包，校验产物后安装。运行用 systemd 受限 service user、只开放必需网络访问和 GitHub 凭证，工作树与日志独立目录。禁止安装脚本执行 cargo/rustc/test。

## 安全配置
凭证不写 TOML，不传给 Worker prompt；Merge Controller 独立授权；生产 main 工作目录不可编辑；Agent 工作树目录隔离。配置中不存在通过 `local_full_test=true` 等允许本地编译的有效开关；即使历史格式有此字段也应直接报配置错误。

## 升级和回滚
备份 SQLite、配置、非秘密状态索引；停止接受新任务、等待安全检查点；安装已验证新二进制；校验 migration 与 GitHub 状态；失败则按兼容性策略回滚。回滚不能恢复已撤销的 Merge 授权，旧 Token 不应恢复。

## 安装验收
OPS-01 无 Rust 工具链可启动；02 安装无 cargo/rustc/test 子进程；03 断网重启后安全 Blocked；04 Windows 计划任务重启恢复；05 安装错误身份不修改仓库；06 升级保留工作树与未提交数据。以上测试通过 Actions 和受控主机验证。