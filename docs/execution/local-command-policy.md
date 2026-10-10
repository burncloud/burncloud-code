# 本地执行硬边界（不可豁免 P0）

本规范实现 [BAM 本地零编译零测试强制政策](../bam-local-execution-policy.md)。本地 Agent **绝不能**编译代码、执行单元/集成/E2E/基准测试；所有对应测试源码提交 GitHub，由 Actions 运行。

## 权限按效果控制
允许：文件读写、生成 `#[cfg(test)]` 测试源码、静态查找、git status/diff/diff --check、受审计 commit/push、读取 GitHub CI 结果、运行已构建 BAM。禁止：cargo check/build/test/run/clippy/bench/nextest/llvm-cov、rustc、其他语言编译器及测试执行器，以及任何 shell/IDE/hook/脚本封装的间接执行。

`cargo fmt` / `rustfmt` 属于非编译格式化；若执行环境无法保证只格式化不启动编译/测试，则禁止执行并交由 GitHub CI 检查。禁止把格式检查当作 CI PASS。

## 不能只做字符串过滤
Agent 不可拥有任意执行所有进程的能力；必须通过受控执行器和 OS 权限/隔离保证：路径不越界、命令不可任意扩权、子进程继承受限权限、无额外逃逸通道。Windows 实施需验证 cmd、PowerShell、batch、任务计划程序、子进程组；Linux 需验证 shell、fork/exec、动态解释器及权限模型。若无法技术性证明某环境隔离有效，停止自动任务，记 `LOCAL_ENFORCEMENT_UNVERIFIED`。

## Commit Hook
`git config core.hooksPath` / 自定义 pre-commit/commit-msg 可能触发编译测试。BAM 必须在自己的受控提交会话中采用独立 hook 运行策略，显式记录跳过原因为“远端 GitHub CI 为权威质量门禁”，不得修改开发者全局 hook。所跳过的全部质量门禁在 GitHub Actions 必须实际运行。

## 验收反例
- LOCAL-01 直接 cargo test；LOCAL-02 bash -c 间接 cargo；LOCAL-03 PowerShell 启动编译器；LOCAL-04 Hook 自动触发 test；LOCAL-05 子进程调用其他测试框架；LOCAL-06 Agent 更改本策略或普通 TOML；LOCAL-07 GitHub CI 不可用时禁止宣告通过。
- **以上反例测试必须在隔离 CI 环境完成；不得为了验证禁令在用户机器上真正触发编译/测试。**

## 本地 BAM 发布
BAM 自身的可执行程序也由 GitHub Actions 编译、测试、打包；用户 Windows/Linux 机器只安装并运行已发布二进制，不依赖本地 cargo/rustc。