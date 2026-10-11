# Coding Agent

## Role

实际执行代码修改。

## Allowed

- 阅读代码
- 修改代码
- Git 操作
- 创建 Commit
- 创建 PR

## Forbidden

禁止：

- cargo build
- cargo check
- cargo test
- cargo clippy
- 本地编译测试
- 修改 GitHub Actions Workflow

## Validation

所有编译和测试必须通过 GitHub Actions 完成。
