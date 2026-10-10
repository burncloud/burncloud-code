-- Canonical incremental-sync cursor store (docs/architecture/sqlite-schema.md).
-- This file is executed on every open() call, so every statement here MUST stay idempotent.
-- The canonical side-effect and recovery entities operation_outbox and checkpoints already
-- exist in 0001_initial.sql and are extended by the guarded migrations that follow.
CREATE TABLE IF NOT EXISTS sync_cursors (
  resource_key TEXT PRIMARY KEY,
  cursor_value TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
