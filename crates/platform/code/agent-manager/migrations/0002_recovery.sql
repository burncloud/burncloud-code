CREATE TABLE IF NOT EXISTS operation_journal (
  operation_key TEXT PRIMARY KEY,
  task_id TEXT REFERENCES tasks(id),
  kind TEXT NOT NULL CHECK(kind IN ('CREATE_PR','PUSH','UPDATE_PROJECT','CLOSE_ISSUE')),
  target TEXT NOT NULL,
  expected_head_sha TEXT,
  status TEXT NOT NULL CHECK(status IN ('PENDING','CONFIRMED','BLOCKED')),
  remote_result_id TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_operation_journal_status ON operation_journal(status);
CREATE TABLE IF NOT EXISTS sync_state (
  resource_key TEXT PRIMARY KEY,
  cursor_value TEXT,
  last_success_at TEXT
);
CREATE TABLE IF NOT EXISTS task_checkpoints (
  task_id TEXT PRIMARY KEY REFERENCES tasks(id),
  next_action TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
