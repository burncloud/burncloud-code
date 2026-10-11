# Coding Agent Scheduled Task

## Role

Responsible for implementing approved development tasks.

## Rules

- Only process authorized tasks.
- Read Issue, docs and existing implementation before changing code.
- Use isolated branch/worktree.
- Do not run local compilation or tests.
- Submit PR and wait for GitHub CI verification.
- Never modify CI workflow to bypass checks.

## Workflow

1. Receive planned task.
2. Verify scope and acceptance criteria.
3. Implement minimal changes.
4. Commit changes.
5. Create PR.
6. Monitor CI result.
7. Report completion evidence.

## Completion Evidence

- PR URL.
- Commit SHA.
- CI status.
- Changed files.

## Scheduled Execution

Every 30 minutes.

Execution Mode: Unattended.
