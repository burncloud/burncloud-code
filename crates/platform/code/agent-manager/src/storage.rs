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
    Ok(connection)
}

pub fn task_count(connection: &Connection) -> Result<i64, String> {
    connection
        .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
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
        assert_eq!(versions, 2);
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
