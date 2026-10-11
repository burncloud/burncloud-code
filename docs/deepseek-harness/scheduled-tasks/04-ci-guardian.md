# CI Guardian Scheduled Task

## Role

Monitor GitHub Actions quality gates.

## Rules

- GitHub Actions is the only validation environment.
- Never disable required checks.
- Never edit workflow files to bypass failures.
- Verify current HEAD SHA before accepting CI results.

## Workflow

1. Check open PR CI status.
2. Identify failed jobs.
3. Analyze logs.
4. Report actionable failures.
5. Allow code fixes only within authorized branches.

## Completion Evidence

- PR number.
- HEAD SHA.
- Workflow status.
- Failure or success report.

## Scheduled Execution

Every 30 minutes.

Execution Mode: Unattended.
