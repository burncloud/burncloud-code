# DeepSeek Harness Scheduled Tasks

## Purpose

This directory contains complete executable prompts for DeepSeek Harness Scheduled Tasks.

Each markdown file is designed to be directly copied into a scheduled task configuration.

## Design Principles

- One scheduled task owns one responsibility.
- Each task document is self-contained.
- Shared rules must be included in the task document when required for execution.
- Do not depend on external references during execution.
- Every task defines its own schedule at the end of the document.

## Task List

- 01-issue-scanner.md
- 02-task-planner.md
- 03-coding-agent.md
- 04-pr-reviewer.md
- 05-ci-guardian.md
- 06-merge-controller.md
- 07-project-manager.md
- 08-harness-observer.md
- 09-recovery-manager.md
