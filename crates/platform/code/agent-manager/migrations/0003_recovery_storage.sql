ALTER TABLE checkpoints ADD COLUMN head_sha TEXT;
ALTER TABLE checkpoints ADD COLUMN worktree_path TEXT;
ALTER TABLE checkpoints ADD COLUMN ci_run_id INTEGER;
ALTER TABLE checkpoints ADD COLUMN lease_epoch INTEGER;
ALTER TABLE operation_outbox ADD COLUMN confirmed_at TEXT;
ALTER TABLE operation_outbox ADD COLUMN remote_ref TEXT;
