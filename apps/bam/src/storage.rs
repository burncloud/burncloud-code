use rusqlite::{params, Connection};
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create state directory: {e}"))?;
    }
    let mut connection = Connection::open(path).map_err(|e| format!("open database: {e}"))?;
    connection.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
        .map_err(|e| e.to_string())?;
    let tx = connection.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch(include_str!("../migrations/0001_initial.sql"))
        .map_err(|e| e.to_string())?;
    tx.execute("INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)", params![1])
        .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(connection)
}

pub fn task_count(connection: &Connection) -> Result<i64, String> {
    connection.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
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
        let versions: i64 = second.query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row.get(0)).unwrap();
        assert_eq!(versions, 1);
    }
    #[test]
    fn unique_active_issue_is_enforced() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../migrations/0001_initial.sql")).unwrap();
        let insert = "INSERT INTO tasks(id,repo_id,issue_node_id,issue_number,state) VALUES (?1,1,'I1',1,'READY')";
        conn.execute(insert, ["one"]).unwrap();
        assert!(conn.execute(insert, ["two"]).is_err());
    }
}
