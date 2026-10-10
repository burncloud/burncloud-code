use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create state directory: {e}"))?;
    }
    let mut connection = Connection::open(path).map_err(|e| format!("open database: {e}"))?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
        .map_err(|e| e.to_string())?;

    let tx = connection.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch(include_str!("../migrations/0001_initial.sql"))
        .map_err(|e| e.to_string())?;
    tx.execute_batch(include_str!("../migrations/0002_recovery.sql"))
        .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
        params![1],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version) VALUES (2)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    apply_migration(
        &mut connection,
        3,
        include_str!("../migrations/0003_recovery_storage.sql"),
    )?;
    Ok(connection)
}

fn apply_migration(connection: &mut Connection, version: i64, sql: &str) -> Result<(), String> {
    let applied: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=?1)",
            [version],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if applied {
        return Ok(());
    }

    let tx = connection
        .transaction()
        .map_err(|e| format!("start migration {version}: {e}"))?;
    tx.execute_batch(sql)
        .map_err(|e| format!("apply migration {version}: {e}"))?;
    tx.execute(
        "INSERT INTO schema_migrations(version) VALUES (?1)",
        [version],
    )
    .map_err(|e| format!("record migration {version}: {e}"))?;
    tx.commit()
        .map_err(|e| format!("commit migration {version}: {e}"))
}

pub fn task_count(connection: &Connection) -> Result<i64, String> {
    connection
        .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    pub resource_key: String,
    pub owner_id: String,
    pub epoch: i64,
    pub expires_at: i64,
}

/// Acquire a lease using the database's write transaction as a compare-and-swap.
/// `now` and `ttl_seconds` are supplied by the caller, so CI can test expiry
/// without sleeps. Lease ownership is valid only while now < expires_at.
pub fn acquire_lease(
    connection: &mut Connection,
    resource: &str,
    owner: &str,
    now: i64,
    ttl_seconds: i64,
) -> Result<Option<Lease>, String> {
    if resource.trim().is_empty() || owner.trim().is_empty() || ttl_seconds <= 0 {
        return Err("invalid lease argument".into());
    }
    let expires_at = now
        .checked_add(ttl_seconds)
        .ok_or("lease deadline overflow")?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let previous: Option<(String, i64, i64)> = tx
        .query_row(
            "SELECT owner_id, fencing_epoch, CAST(expires_at AS INTEGER) FROM leases WHERE resource_key=?1",
            [resource],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let epoch = match previous {
        None => 1,
        Some((_, _, deadline)) if deadline > now => return Ok(None),
        Some((_, previous_epoch, _)) => previous_epoch
            .checked_add(1)
            .ok_or("lease epoch overflow")?,
    };
    tx.execute(
        "INSERT INTO leases(resource_key,owner_id,fencing_epoch,expires_at,heartbeat_at)
         VALUES (?1,?2,?3,?4,?5)
         ON CONFLICT(resource_key) DO UPDATE SET owner_id=excluded.owner_id,
         fencing_epoch=excluded.fencing_epoch,expires_at=excluded.expires_at,
         heartbeat_at=excluded.heartbeat_at",
        params![
            resource,
            owner,
            epoch,
            expires_at.to_string(),
            now.to_string()
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Some(Lease {
        resource_key: resource.into(),
        owner_id: owner.into(),
        epoch,
        expires_at,
    }))
}

pub fn renew_lease(
    connection: &Connection,
    lease: &Lease,
    now: i64,
    ttl_seconds: i64,
) -> Result<Option<Lease>, String> {
    if ttl_seconds <= 0 || now >= lease.expires_at {
        return Ok(None);
    }
    let expires_at = now
        .checked_add(ttl_seconds)
        .ok_or("lease deadline overflow")?;
    let updated = connection
        .execute(
            "UPDATE leases SET expires_at=?1,heartbeat_at=?2
             WHERE resource_key=?3 AND owner_id=?4 AND fencing_epoch=?5
             AND CAST(expires_at AS INTEGER)>?6",
            params![
                expires_at.to_string(),
                now.to_string(),
                lease.resource_key,
                lease.owner_id,
                lease.epoch,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok((updated == 1).then(|| Lease {
        expires_at,
        ..lease.clone()
    }))
}

pub fn release_lease(connection: &Connection, lease: &Lease) -> Result<bool, String> {
    let updated = connection
        .execute(
            "UPDATE leases SET expires_at='0'
             WHERE resource_key=?1 AND owner_id=?2 AND fencing_epoch=?3",
            params![lease.resource_key, lease.owner_id, lease.epoch],
        )
        .map_err(|e| e.to_string())?;
    Ok(updated == 1)
}

pub fn create_task(
    connection: &Connection,
    id: &str,
    repo_id: i64,
    issue_node_id: &str,
    issue_number: i64,
) -> Result<(), String> {
    if id.is_empty() || repo_id <= 0 || issue_node_id.is_empty() || issue_number <= 0 {
        return Err("invalid task identity".into());
    }
    connection.execute(
        "INSERT INTO tasks(id,repo_id,issue_node_id,issue_number,state) VALUES (?1,?2,?3,?4,'BACKLOG')",
        params![id,repo_id,issue_node_id,issue_number],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn transition_task(
    connection: &mut Connection,
    task_id: &str,
    next: crate::state::TaskState,
    evidence: crate::state::Evidence,
    event_id: &str,
) -> Result<(), String> {
    use crate::state::TaskState;
    if event_id.is_empty() {
        return Err("event id must not be empty".into());
    }
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let state: String = tx
        .query_row("SELECT state FROM tasks WHERE id=?1", [task_id], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    let from = TaskState::parse_name(&state).ok_or("unknown persisted task state")?;
    from.transition(next, evidence)
        .map_err(|_| "state transition rejected")?;
    let inserted = tx
        .execute(
            "INSERT OR IGNORE INTO task_events(id,task_id,from_state,to_state,event_type)
         VALUES (?1,?2,?3,?4,?5)",
            params![
                event_id,
                task_id,
                state,
                next.as_name(),
                format!("{evidence:?}")
            ],
        )
        .map_err(|e| e.to_string())?;
    if inserted == 0 {
        return Err("duplicate transition event".into());
    }
    tx.execute(
        "UPDATE tasks SET state=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
        params![next.as_name(), task_id],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn record_inbox_event(connection: &Connection, event_key: &str) -> Result<bool, String> {
    if event_key.trim().is_empty() {
        return Err("event key must not be empty".into());
    }
    let inserted = connection
        .execute(
            "INSERT OR IGNORE INTO event_inbox(event_key) VALUES (?1)",
            [event_key],
        )
        .map_err(|e| e.to_string())?;
    Ok(inserted == 1)
}

pub fn mark_inbox_handled(connection: &Connection, event_key: &str) -> Result<bool, String> {
    if event_key.trim().is_empty() {
        return Err("event key must not be empty".into());
    }
    let updated = connection
        .execute(
            "UPDATE event_inbox SET handled_at=CURRENT_TIMESTAMP
             WHERE event_key=?1 AND handled_at IS NULL",
            [event_key],
        )
        .map_err(|e| e.to_string())?;
    Ok(updated == 1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationIntent {
    pub operation_key: String,
    pub task_id: Option<String>,
    pub operation_type: String,
    pub expected_head_sha: Option<String>,
}

pub fn enqueue_operation(
    connection: &mut Connection,
    intent: &OperationIntent,
) -> Result<bool, String> {
    if intent.operation_key.trim().is_empty() || intent.operation_type.trim().is_empty() {
        return Err("operation key and type must not be empty".into());
    }

    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let existing: Option<(Option<String>, String, Option<String>)> = tx
        .query_row(
            "SELECT task_id,operation_type,expected_head_sha
             FROM operation_outbox WHERE operation_key=?1",
            [&intent.operation_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some((task_id, operation_type, expected_head_sha)) = existing {
        let same = task_id == intent.task_id
            && operation_type == intent.operation_type
            && expected_head_sha == intent.expected_head_sha;
        if same {
            tx.commit().map_err(|e| e.to_string())?;
            return Ok(false);
        }
        return Err("operation key conflicts with a different intent".into());
    }

    tx.execute(
        "INSERT INTO operation_outbox(
            operation_key,task_id,operation_type,expected_head_sha,state
         ) VALUES (?1,?2,?3,?4,'PENDING')",
        params![
            intent.operation_key,
            intent.task_id,
            intent.operation_type,
            intent.expected_head_sha
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn confirm_operation(
    connection: &Connection,
    operation_key: &str,
    remote_ref: Option<&str>,
) -> Result<bool, String> {
    if operation_key.trim().is_empty() {
        return Err("operation key must not be empty".into());
    }
    let updated = connection
        .execute(
            "UPDATE operation_outbox
             SET state='CONFIRMED',confirmed_at=CURRENT_TIMESTAMP,remote_ref=?1
             WHERE operation_key=?2 AND state!='CONFIRMED'",
            params![remote_ref, operation_key],
        )
        .map_err(|e| e.to_string())?;
    Ok(updated == 1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub task_id: String,
    pub last_action: String,
    pub next_action: String,
    pub head_sha: Option<String>,
    pub worktree_path: Option<String>,
    pub ci_run_id: Option<i64>,
    pub lease_epoch: Option<i64>,
}

pub fn save_checkpoint(connection: &Connection, checkpoint: &Checkpoint) -> Result<(), String> {
    if checkpoint.task_id.trim().is_empty()
        || checkpoint.last_action.trim().is_empty()
        || checkpoint.next_action.trim().is_empty()
    {
        return Err("checkpoint identity and actions must not be empty".into());
    }

    connection
        .execute(
            "INSERT INTO checkpoints(
                task_id,last_action,next_action,head_sha,worktree_path,ci_run_id,lease_epoch,updated_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,CURRENT_TIMESTAMP)
             ON CONFLICT(task_id) DO UPDATE SET
                last_action=excluded.last_action,
                next_action=excluded.next_action,
                head_sha=excluded.head_sha,
                worktree_path=excluded.worktree_path,
                ci_run_id=excluded.ci_run_id,
                lease_epoch=excluded.lease_epoch,
                updated_at=CURRENT_TIMESTAMP",
            params![
                checkpoint.task_id,
                checkpoint.last_action,
                checkpoint.next_action,
                checkpoint.head_sha,
                checkpoint.worktree_path,
                checkpoint.ci_run_id,
                checkpoint.lease_epoch
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_checkpoint(
    connection: &Connection,
    task_id: &str,
) -> Result<Option<Checkpoint>, String> {
    if task_id.trim().is_empty() {
        return Err("task id must not be empty".into());
    }
    connection
        .query_row(
            "SELECT task_id,last_action,next_action,head_sha,worktree_path,ci_run_id,lease_epoch
             FROM checkpoints WHERE task_id=?1",
            [task_id],
            |row| {
                Ok(Checkpoint {
                    task_id: row.get(0)?,
                    last_action: row.get(1)?,
                    next_action: row.get(2)?,
                    head_sha: row.get(3)?,
                    worktree_path: row.get(4)?,
                    ci_run_id: row.get(5)?,
                    lease_epoch: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nested/state.sqlite");
        let first = open(&path).unwrap();
        assert_eq!(task_count(&first).unwrap(), 0);
        drop(first);
        let second = open(&path).unwrap();
        assert_eq!(task_count(&second).unwrap(), 0);
        let versions: i64 = second
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(versions, 3);
    }

    #[test]
    fn unique_active_issue_is_enforced() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../migrations/0001_initial.sql"))
            .unwrap();
        let insert = "INSERT INTO tasks(id,repo_id,issue_node_id,issue_number,state) VALUES (?1,1,'I1',1,'READY')";
        conn.execute(insert, ["one"]).unwrap();
        assert!(conn.execute(insert, ["two"]).is_err());
    }

    #[test]
    fn inbox_replay_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let conn = open(&temp.path().join("state.db")).unwrap();
        assert!(record_inbox_event(&conn, "delivery:1").unwrap());
        for _ in 0..100 {
            assert!(!record_inbox_event(&conn, "delivery:1").unwrap());
        }
        assert!(mark_inbox_handled(&conn, "delivery:1").unwrap());
        assert!(!mark_inbox_handled(&conn, "delivery:1").unwrap());
    }

    #[test]
    fn outbox_reuses_identical_intent_and_rejects_conflict() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = open(&temp.path().join("state.db")).unwrap();
        create_task(&conn, "t1", 1, "I_1", 1).unwrap();
        let intent = OperationIntent {
            operation_key: "push:t1:abc".into(),
            task_id: Some("t1".into()),
            operation_type: "push".into(),
            expected_head_sha: Some("abc".into()),
        };
        assert!(enqueue_operation(&mut conn, &intent).unwrap());
        assert!(!enqueue_operation(&mut conn, &intent).unwrap());

        let conflicting = OperationIntent {
            expected_head_sha: Some("def".into()),
            ..intent.clone()
        };
        assert!(enqueue_operation(&mut conn, &conflicting).is_err());

        assert!(confirm_operation(&conn, &intent.operation_key, Some("refs/heads/bam/t1")).unwrap());
        assert!(!confirm_operation(&conn, &intent.operation_key, None).unwrap());
    }

    #[test]
    fn checkpoint_round_trips_recovery_context() {
        let temp = tempfile::tempdir().unwrap();
        let conn = open(&temp.path().join("state.db")).unwrap();
        create_task(&conn, "t1", 1, "I_1", 1).unwrap();

        let checkpoint = Checkpoint {
            task_id: "t1".into(),
            last_action: "push".into(),
            next_action: "wait_ci".into(),
            head_sha: Some("0123456789abcdef".into()),
            worktree_path: Some("/managed/t1".into()),
            ci_run_id: Some(42),
            lease_epoch: Some(7),
        };
        save_checkpoint(&conn, &checkpoint).unwrap();
        assert_eq!(load_checkpoint(&conn, "t1").unwrap(), Some(checkpoint));
    }

    #[test]
    fn checkpoint_requires_existing_task() {
        let temp = tempfile::tempdir().unwrap();
        let conn = open(&temp.path().join("state.db")).unwrap();
        let checkpoint = Checkpoint {
            task_id: "missing".into(),
            last_action: "none".into(),
            next_action: "sync".into(),
            head_sha: None,
            worktree_path: None,
            ci_run_id: None,
            lease_epoch: None,
        };
        assert!(save_checkpoint(&conn, &checkpoint).is_err());
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use crate::state::{Evidence, TaskState};

    #[test]
    fn lease_exclusive_expiry_and_fencing() {
        let temp = tempfile::tempdir().unwrap();
        let mut first = open(&temp.path().join("state.db")).unwrap();
        let mut second = open(&temp.path().join("state.db")).unwrap();
        let a = acquire_lease(&mut first, "branch:1", "agent-a", 100, 10)
            .unwrap()
            .unwrap();
        assert_eq!(a.epoch, 1);
        assert!(acquire_lease(&mut second, "branch:1", "agent-b", 109, 10)
            .unwrap()
            .is_none());
        let b = acquire_lease(&mut second, "branch:1", "agent-b", 110, 10)
            .unwrap()
            .unwrap();
        assert_eq!(b.epoch, 2);
        assert!(!release_lease(&first, &a).unwrap());
        assert!(renew_lease(&first, &a, 111, 10).unwrap().is_none());
        assert!(release_lease(&second, &b).unwrap());
        let c = acquire_lease(&mut first, "branch:1", "agent-c", 112, 10)
            .unwrap()
            .unwrap();
        assert_eq!(c.epoch, 3);
    }

    #[test]
    fn renewal_and_bad_input() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = open(&temp.path().join("state.db")).unwrap();
        assert!(acquire_lease(&mut conn, "", "owner", 1, 2).is_err());
        assert!(acquire_lease(&mut conn, "a", "owner", i64::MAX, 2).is_err());
        let lease = acquire_lease(&mut conn, "a", "owner", 5, 10)
            .unwrap()
            .unwrap();
        let renewed = renew_lease(&conn, &lease, 9, 20).unwrap().unwrap();
        assert_eq!(renewed.expires_at, 29);
        assert!(renew_lease(&conn, &lease, 16, 5).unwrap().is_none());
        assert!(renew_lease(&conn, &renewed, 29, 5).unwrap().is_none());
    }

    #[test]
    fn guarded_transition_and_event_are_atomic() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = open(&temp.path().join("state.db")).unwrap();
        create_task(&conn, "t1", 1, "I_1", 10).unwrap();
        assert!(create_task(&conn, "t2", 1, "I_1", 10).is_err());
        assert!(
            transition_task(&mut conn, "t1", TaskState::Merged, Evidence::None, "ev0").is_err()
        );
        transition_task(
            &mut conn,
            "t1",
            TaskState::Ready,
            Evidence::ReadyCriteria,
            "ev1",
        )
        .unwrap();
        assert!(
            transition_task(&mut conn, "t1", TaskState::Running, Evidence::None, "ev2").is_err()
        );
        let current: String = conn
            .query_row("SELECT state FROM tasks WHERE id='t1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(current, "READY");
        let event_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(event_count, 1);
    }
}
