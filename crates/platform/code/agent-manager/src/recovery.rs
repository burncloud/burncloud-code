//! Durable intent log for GitHub side effects (docs/execution/crash-recovery.md).
//!
//! This module is a thin domain layer over the single canonical outbox in
//! [`crate::storage`] (`operation_outbox`). It deliberately owns no SQL of its own so
//! that there is exactly one outbox implementation and one recovery point store.
use crate::storage::{self, OperationIntent};
use rusqlite::Connection;

/// Side effects that may be persisted as an intent. Merge is intentionally absent:
/// merging is not an Agent capability.
pub const OPERATION_KINDS: [&str; 4] = ["CREATE_PR", "PUSH", "UPDATE_PROJECT", "CLOSE_ISSUE"];

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

/// Persist an operation intent before performing the remote write. Replaying the same
/// intent is a no-op; reusing the same key for a different intent is an error.
pub fn begin_operation(conn: &mut Connection, op: &Operation) -> Result<(), String> {
    if op.key.trim().is_empty() || op.target.trim().is_empty() {
        return Err("operation key and target must not be empty".into());
    }
    if op.status != "PENDING" {
        return Err("operation intent must be recorded as PENDING".into());
    }
    if !OPERATION_KINDS.contains(&op.kind.as_str()) {
        return Err("invalid operation kind".into());
    }
    storage::enqueue_operation(
        conn,
        &OperationIntent {
            operation_key: op.key.clone(),
            task_id: op.task_id.clone(),
            operation_type: op.kind.clone(),
            target: op.target.clone(),
            expected_head_sha: op.head.clone(),
        },
    )
    .map(|_| ())
}

/// Only a caller that has independently re-read GitHub may confirm an operation.
/// This never generates a remote request and cannot execute a merge.
pub fn confirm_operation(conn: &Connection, key: &str, remote_id: &str) -> Result<bool, String> {
    if remote_id.trim().is_empty() {
        return Err("remote evidence id is required".into());
    }
    storage::confirm_operation(conn, key, Some(remote_id))
}

/// Intents still awaiting remote confirmation. A caller must reconcile each of these
/// against GitHub before replaying anything.
pub fn pending_operations(conn: &Connection) -> Result<Vec<Operation>, String> {
    let rows = storage::pending_operations(conn)?;
    Ok(rows
        .into_iter()
        .map(|row| Operation {
            key: row.operation_key,
            task_id: row.task_id,
            kind: row.operation_type,
            target: row.target.unwrap_or_default(),
            head: row.expected_head_sha,
            status: row.state,
            remote_id: row.remote_ref,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage;

    fn pending_operation() -> Operation {
        Operation {
            key: "create:repo:1".into(),
            task_id: None,
            kind: "CREATE_PR".into(),
            target: "repo/branch".into(),
            head: Some("abc".into()),
            status: "PENDING".into(),
            remote_id: None,
        }
    }

    #[test]
    fn intent_deduplicates_and_recovery_requires_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = storage::open(&dir.path().join("state.db")).unwrap();
        let op = pending_operation();
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
        assert!(confirm_operation(&conn, &op.key, "  ").is_err());

        drop(conn);
        let restored = storage::open(&dir.path().join("state.db")).unwrap();
        assert!(pending_operations(&restored).unwrap().is_empty());
    }

    #[test]
    fn only_recoverable_intents_are_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = storage::open(&dir.path().join("state.db")).unwrap();
        let op = pending_operation();

        assert!(begin_operation(
            &mut conn,
            &Operation {
                kind: "MERGE".into(),
                ..op.clone()
            }
        )
        .is_err());
        assert!(begin_operation(
            &mut conn,
            &Operation {
                status: "CONFIRMED".into(),
                ..op.clone()
            }
        )
        .is_err());
        assert!(begin_operation(
            &mut conn,
            &Operation {
                target: "  ".into(),
                ..op.clone()
            }
        )
        .is_err());
        assert!(pending_operations(&conn).unwrap().is_empty());
    }
}
