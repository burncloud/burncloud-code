PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  repo_id INTEGER NOT NULL,
  issue_node_id TEXT NOT NULL,
  issue_number INTEGER NOT NULL,
  state TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 2,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_active_issue ON tasks(repo_id, issue_node_id) WHERE active = 1;
CREATE TABLE IF NOT EXISTS task_events (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  from_state TEXT,
  to_state TEXT NOT NULL,
  event_type TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS leases (
  resource_key TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  fencing_epoch INTEGER NOT NULL,
  expires_at TEXT NOT NULL,
  heartbeat_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS event_inbox (
  event_key TEXT PRIMARY KEY,
  received_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  handled_at TEXT
);
CREATE TABLE IF NOT EXISTS operation_outbox (
  operation_key TEXT PRIMARY KEY,
  task_id TEXT REFERENCES tasks(id),
  operation_type TEXT NOT NULL,
  expected_head_sha TEXT,
  state TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS checkpoints (
  task_id TEXT PRIMARY KEY REFERENCES tasks(id),
  last_action TEXT NOT NULL,
  next_action TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
