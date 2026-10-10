//! Durable intent log for GitHub side effects. Recovery is deliberately read-only.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub key: String,
    pub task_id: Option<String>,
    pub kind: String,
    pub target: String,
    pub head: Option<String>,
    pub status: String,
    pub remote_id: Option<String>,
}

pub fn begin_operation(conn: &mut Connection, op: &Operation) -> Result<(), String> {
    if op.key.is_empty() || op.target.is_empty() || op.status != "PENDING" {
        return Err("invalid operation intent".into());
    }
    if !["CREATE_PR", "PUSH", "UPDATE_PROJECT", "CLOSE_ISSUE"].contains(&op.kind.as_str()) {
        return Err("invalid operation kind".into());
    }
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    let existing: Option<(String, String, Option<String>)> = tx
        .query_row(
            "SELECT kind,target,expected_head_sha FROM operation_journal WHERE operation_key=?1",
            [&op.key],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some((kind, target, head)) = existing {
        if kind == op.kind && target == op.target && head == op.head {
            return Ok(());
        }
        return Err("operation key reused with different intent".into());
    }
    tx.execute(
        "INSERT INTO operation_journal(operation_key,task_id,kind,target,expected_head_sha,status)
         VALUES (?1,?2,?3,?4,?5,'PENDING')",
        params![op.key, op.task_id, op.kind, op.target, op.head],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

/// Only the caller that has independently re-read GitHub may confirm an operation.
/// Does not generate an external request and cannot execute a merge.
pub fn confirm_operation(conn: &Connection, key: &str, remote_id: &str) -> Result<bool, String> {
    if remote_id.trim().is_empty() {
        return Err("remote evidence id is required".into());
    }
    let changed = conn.execute(
        "UPDATE operation_journal SET status='CONFIRMED',remote_result_id=?2,updated_at=CURRENT_TIMESTAMP
         WHERE operation_key=?1 AND status='PENDING'",
        params![key, remote_id],
    ).map_err(|e| e.to_string())?;
    Ok(changed == 1)
}

pub fn pending_operations(conn: &Connection) -> Result<Vec<Operation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT operation_key,task_id,kind,target,expected_head_sha,status,remote_result_id
         FROM operation_journal WHERE status='PENDING' ORDER BY created_at,operation_key",
        )
        .map_err(|e| e.to_string())?;
    let iter = stmt
        .query_map([], |r| {
            Ok(Operation {
                key: r.get(0)?,
                task_id: r.get(1)?,
                kind: r.get(2)?,
                target: r.get(3)?,
                head: r.get(4)?,
                status: r.get(5)?,
                remote_id: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    iter.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn save_checkpoint(
    conn: &Connection,
    task_id: &str,
    action: &str,
    payload: &str,
) -> Result<(), String> {
    if task_id.is_empty() || action.is_empty() {
        return Err("invalid checkpoint".into());
    }
    conn.execute(
        "INSERT INTO task_checkpoints(task_id,next_action,payload_json) VALUES(?1,?2,?3)
         ON CONFLICT(task_id) DO UPDATE SET next_action=excluded.next_action,
         payload_json=excluded.payload_json,updated_at=CURRENT_TIMESTAMP",
        params![task_id, action, payload],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage;
    #[test]
    fn intent_deduplicates_and_recovery_requires_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = storage::open(&dir.path().join("state.db")).unwrap();
        let op = Operation {
            key: "create:repo:1".into(),
            task_id: None,
            kind: "CREATE_PR".into(),
            target: "repo/branch".into(),
            head: Some("abc".into()),
            status: "PENDING".into(),
            remote_id: None,
        };
        begin_operation(&mut conn, &op).unwrap();
        begin_operation(&mut conn, &op).unwrap();
        assert_eq!(pending_operations(&conn).unwrap().len(), 1);
        let mut changed = op.clone();
        changed.target = "other".into();
        assert!(begin_operation(&mut conn, &changed).is_err());
        assert!(!confirm_operation(&conn, "missing", "PR_1").unwrap());
        assert!(confirm_operation(&conn, &op.key, "PR_1").unwrap());
        assert!(!confirm_operation(&conn, &op.key, "PR_1").unwrap());
        assert!(pending_operations(&conn).unwrap().is_empty());
        drop(conn);
        let restored = storage::open(&dir.path().join("state.db")).unwrap();
        assert!(pending_operations(&restored).unwrap().is_empty());
    }
    #[test]
    fn checkpoint_is_durable() {
        let dir = tempfile::tempdir().unwrap();
        let conn = storage::open(&dir.path().join("state.db")).unwrap();
        storage::create_task(&conn, "one", 1, "I_1", 1).unwrap();
        save_checkpoint(&conn, "one", "WAIT_CI", r#"{"sha":"abc"}"#).unwrap();
        save_checkpoint(&conn, "one", "RECHECK", r#"{"sha":"def"}"#).unwrap();
        let result: String = conn
            .query_row(
                "SELECT next_action FROM task_checkpoints WHERE task_id='one'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(result, "RECHECK");
    }
}
