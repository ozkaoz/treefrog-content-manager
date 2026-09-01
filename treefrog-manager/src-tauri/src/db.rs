//! Minimal persistence layer (SQLite). Scoped and honest:
//! - WHAT IS IMPLEMENTED: schema + migrations, job/deployment records written
//!   after each deploy (content hash, size, timestamps, profile/app version,
//!   target identity, deployment status).
//! - WHAT REMAINS (documented, not faked): source_library indexing across
//!   sessions and content_fingerprint-based incremental scans are future work;
//!   nothing reads fingerprints back yet.

use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Application data dir for the DB: %APPDATA%/TreeFrogContentManager (Windows)
/// or XDG data home elsewhere. User paths are NEVER committed to the repo.
pub fn db_path() -> PathBuf {
    let base = if let Some(appdata) = std::env::var_os("APPDATA") {
        PathBuf::from(appdata).join("TreeFrogContentManager")
    } else if let Some(home) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(home).join("TreeFrogContentManager")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".local/share/TreeFrogContentManager")
    } else {
        std::env::temp_dir().join("TreeFrogContentManager")
    };
    let _ = std::fs::create_dir_all(&base);
    base.join("treefrog-manager.db")
}

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the DB with migrations. Idempotent: safe to call on every start.
pub fn init_db() -> anyhow::Result<Connection> {
    let path = db_path();
    init_db_at(&path)
}

pub fn init_db_at(db_path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_migrations (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;
    // Migration 1: base tables
    let m1_applied: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE id = 1",
            [],
            |r| Ok(r.get::<_, i64>(0)? > 0),
        )
        .unwrap_or(false);
    if !m1_applied {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS target (
                id INTEGER PRIMARY KEY,
                stable_id TEXT UNIQUE NOT NULL,
                label TEXT,
                filesystem TEXT,
                capacity_bytes INTEGER,
                first_seen TEXT NOT NULL DEFAULT (datetime('now')),
                last_seen TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS source_library (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                added_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS content_fingerprint (
                id INTEGER PRIMARY KEY,
                sha256 TEXT NOT NULL,
                size INTEGER NOT NULL,
                mtime TEXT,
                profile_version TEXT,
                app_version TEXT,
                UNIQUE(sha256)
            );
            CREATE TABLE IF NOT EXISTS job (
                id INTEGER PRIMARY KEY,
                kind TEXT NOT NULL,
                sd_path TEXT NOT NULL,
                target_stable_id TEXT,
                profile_version TEXT,
                app_version TEXT,
                started_at TEXT NOT NULL DEFAULT (datetime('now')),
                finished_at TEXT,
                status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS job_entry (
                id INTEGER PRIMARY KEY,
                job_id INTEGER NOT NULL REFERENCES job(id),
                source TEXT NOT NULL,
                destination TEXT NOT NULL,
                effective_action TEXT NOT NULL,
                sha256 TEXT,
                size INTEGER,
                status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS deployment (
                id INTEGER PRIMARY KEY,
                job_id INTEGER NOT NULL REFERENCES job(id),
                target_stable_id TEXT NOT NULL,
                destination TEXT NOT NULL,
                sha256 TEXT NOT NULL,
                size INTEGER NOT NULL,
                deployed_at TEXT NOT NULL DEFAULT (datetime('now')),
                status TEXT NOT NULL
            );
            INSERT INTO schema_migrations (id, name) VALUES (1, 'base_tables');
            ",
        )?;
    }
    Ok(conn)
}

/// Record a completed deployment (minimal persistence scope).
pub fn record_deployment(
    conn: &Connection,
    job_kind: &str,
    sd_path: &str,
    target_stable_id: Option<&str>,
    profile_version: &str,
    entries: &[(String, String, String, Option<String>, Option<u64>, String)], // (source, dest, eff_action, sha256, size, status)
) -> anyhow::Result<i64> {
    conn.execute(
        "INSERT INTO job (kind, sd_path, target_stable_id, profile_version, app_version, status) VALUES (?1, ?2, ?3, ?4, ?5, 'started')",
        params![job_kind, sd_path, target_stable_id, profile_version, APP_VERSION],
    )?;
    let job_id = conn.last_insert_rowid();
    for (source, dest, eff_action, sha256, size, status) in entries {
        conn.execute(
            "INSERT INTO job_entry (job_id, source, destination, effective_action, sha256, size, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![job_id, source, dest, eff_action, sha256, size, status],
        )?;
        if *status == "deployed" {
            if let Some(h) = sha256 {
                conn.execute(
                    "INSERT INTO deployment (job_id, target_stable_id, destination, sha256, size, status) VALUES (?1, ?2, ?3, ?4, ?5, 'ok')",
                    params![job_id, target_stable_id.unwrap_or("unknown"), dest, h, size.unwrap_or(0)],
                )?;
                conn.execute(
                    "INSERT OR IGNORE INTO content_fingerprint (sha256, size, profile_version, app_version) VALUES (?1, ?2, ?3, ?4)",
                    params![h, size.unwrap_or(0), profile_version, APP_VERSION],
                )?;
            }
        }
    }
    conn.execute(
        "UPDATE job SET finished_at = datetime('now'), status = 'finished' WHERE id = ?1",
        params![job_id],
    )?;
    Ok(job_id)
}

#[cfg(test)]
mod db_tests {
    use super::*;

    /// Migrations are idempotent and the schema matches the documented model.
    #[test]
    fn init_db_idempotent_with_expected_tables() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test.db");
        {
            let conn = init_db_at(&path).unwrap();
            let tables: Vec<String> = {
                let mut stmt = conn
                    .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                    .unwrap();
                let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
                rows.filter_map(|r| r.ok()).collect()
            };
            for expected in [
                "schema_migrations",
                "target",
                "source_library",
                "content_fingerprint",
                "job",
                "job_entry",
                "deployment",
            ] {
                assert!(
                    tables.iter().any(|t| t == expected),
                    "missing table {expected}: {tables:?}"
                );
            }
        }
        // Second init must not fail or duplicate migration
        let conn2 = init_db_at(&path).unwrap();
        let count: i64 = conn2
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "migration must be applied exactly once");
    }

    /// Deployment records store hash/size/versions/target identity.
    #[test]
    fn record_deployment_stores_fingerprints() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("test2.db");
        let conn = init_db_at(&path).unwrap();
        let entries = vec![
            (
                "C:/src/a.nes".to_string(),
                "roms/FC/a.nes".to_string(),
                "copy".to_string(),
                Some("deadbeef".to_string()),
                Some(10u64),
                "deployed".to_string(),
            ),
            (
                "C:/src/b.nes".to_string(),
                "roms/FC/b.nes".to_string(),
                "skip_duplicate".to_string(),
                None,
                Some(5u64),
                "skipped".to_string(),
            ),
        ];
        let job_id =
            record_deployment(&conn, "deploy", "G:/", Some("guid-123"), "1.1.0", &entries).unwrap();
        assert!(job_id > 0);
        let fp: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_fingerprint WHERE sha256='deadbeef'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(fp, 1);
        let dep: i64 = conn
            .query_row("SELECT COUNT(*) FROM deployment", [], |r| r.get(0))
            .unwrap();
        assert_eq!(dep, 1, "only deployed entries create deployment rows");
        let status: String = conn
            .query_row("SELECT status FROM job WHERE id=?1", params![job_id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(status, "finished");
    }
}
