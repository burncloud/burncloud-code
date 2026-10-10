-- Required entities from docs/architecture/sqlite-schema.md that had no versioned DDL yet.
-- Applied exactly once through the guarded apply_migration path.

-- Dependency DAG: a task depends on a (repo, issue node) pair. PK prevents duplicate edges.
CREATE TABLE IF NOT EXISTS task_dependencies (
  task_id TEXT NOT NULL REFERENCES tasks(id),
  depends_on_repo_id INTEGER NOT NULL,
  depends_on_node_id TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (task_id, depends_on_repo_id, depends_on_node_id)
);

-- PR association. A task may only have one active branch writer.
CREATE TABLE IF NOT EXISTS task_prs (
  task_id TEXT NOT NULL REFERENCES tasks(id),
  pr_node_id TEXT NOT NULL,
  pr_number INTEGER NOT NULL,
  branch TEXT NOT NULL,
  head_sha TEXT,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (task_id, pr_node_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_task_active_branch
  ON task_prs(task_id, branch) WHERE active = 1;

CREATE TABLE IF NOT EXISTS agent_sessions (
  session_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  adapter TEXT NOT NULL,
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  ended_at TEXT,
  result TEXT
);
CREATE INDEX IF NOT EXISTS idx_agent_sessions_task ON agent_sessions(task_id);

CREATE TABLE IF NOT EXISTS worktrees (
  path TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  branch TEXT NOT NULL,
  lease_epoch INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_worktrees_task ON worktrees(task_id);

CREATE TABLE IF NOT EXISTS ci_runs (
  run_id INTEGER NOT NULL,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  workflow TEXT NOT NULL,
  head_sha TEXT NOT NULL,
  merge_sha TEXT,
  status TEXT NOT NULL,
  conclusion TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (run_id, task_id)
);

CREATE TABLE IF NOT EXISTS ci_checks (
  check_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  run_id INTEGER,
  context TEXT NOT NULL,
  source TEXT NOT NULL,
  sha TEXT NOT NULL,
  status TEXT NOT NULL,
  conclusion TEXT,
  actually_executed INTEGER NOT NULL DEFAULT 0 CHECK (actually_executed IN (0,1)),
  url TEXT,
  observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_ci_check_identity
  ON ci_checks(task_id, context, sha, source);

CREATE TABLE IF NOT EXISTS repair_attempts (
  attempt_id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id),
  fingerprint TEXT NOT NULL,
  attempt_number INTEGER NOT NULL CHECK (attempt_number > 0),
  outcome TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_repair_attempts_fingerprint
  ON repair_attempts(task_id, fingerprint);

CREATE TABLE IF NOT EXISTS merge_authorizations (
  authorization_id TEXT PRIMARY KEY,
  repo_id INTEGER NOT NULL,
  pr_number INTEGER NOT NULL,
  head_sha TEXT NOT NULL,
  actor TEXT NOT NULL,
  expires_at TEXT,
  revoked_at TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_merge_authorizations_pr
  ON merge_authorizations(repo_id, pr_number, head_sha);

CREATE TABLE IF NOT EXISTS policies (
  version TEXT NOT NULL,
  hash TEXT NOT NULL,
  body TEXT NOT NULL,
  captured_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (version, hash)
);
