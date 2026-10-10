# BAM 目录与领域归属决策 (ADR-F0-001)

## 依据

- https://burncloud.github.io/architecture/base-rules/
- https://burncloud.github.io/architecture/crates/
- https://burncloud.github.io/architecture/crates/platform/
- https://github.com/burncloud/burncloud/blob/main/crates/platform/code/README.md

官方规则要求：外部按领域和功能组织，内部按实现技术组织；默认浅目录；目录不必等于 crate；代码和测试跟随 Owner。主仓库现有开发工具代码位于 `crates/platform/code`。

## F0 目录

```text
crates/
  platform/
    code/
      agent-manager/
        Cargo.toml
        src/
          lib.rs
          main.rs
          config.rs
          state.rs
          storage.rs
        migrations/
          0001_initial.sql
        tests/
          cli.rs
```

本文件夹在独立 `burncloud-code` 仓库中，表示和主仓库 `crates/platform/code` 相同的领域归属，**并不表示必须将其移动到主仓库**。BAM 是开发工具的独立执行器，不得混入业务层、通用共享层或现有 `burncloud-code` 公共 API。暂时保持一个独立 Cargo package；有实际需求时才继续拆分 crate。当前根目录 Cargo.toml 原样保留；子 package 以独立 `[workspace]` 隔离，避免根 manifest 没有 Rust target 影响 CI。

## 后续目录演进

新增功能优先在 `agent-manager/src/` 内按实际责任划分 `github`、`agent`、`scheduler`、`ci`、`acceptance`、`merge` 等模块，只有出现明确独立实现/所有权和稳定 API 才拆成新的 crate。不要机械按 M01–M16 创建十六个顶层 crate。

单元测试写在 Owner 模块 `#[cfg(test)]`；本 crate 公开行为和集成测试放在 `agent-manager/tests/`；跨系统 E2E 归属单独可控测试单元。**所有编译、格式检查、Clippy、单元测试、集成测试都只在 GitHub Actions 执行，本地只编辑并 Push。**

## 迁移验收

1. 原 `apps/bam/` 不再存在，源码、迁移 SQL 和测试无丢失。
2. Actions `--manifest-path` 指向新目录，实际执行而非跳过。
3. 不更改或替换现有根 Cargo.toml；不降低 CI 和用户 Merge 授权门禁。
4. 当前 PR HEAD 的 Fmt、Clippy、Unit/Integration Tests 由 GitHub Actions 实际执行通过。
