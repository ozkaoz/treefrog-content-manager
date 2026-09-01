use rusqlite::{params, Connection};
use std::path::Path;

/// SQLite schema for persistent library index
pub fn init_db(db_path: &Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS source_libraries (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            added_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS known_sd_targets (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            label TEXT,
            last_seen TEXT
        );
        CREATE TABLE IF NOT EXISTS content_fingerprints (
            id INTEGER PRIMARY KEY,
            source_path TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            size INTEGER NOT NULL,
            mtime TEXT,
            profile_version TEXT,
            UNIQUE(source_path, sha256)
        );
        CREATE TABLE IF NOT EXISTS previous_deployments (
            id INTEGER PRIMARY KEY,
            sd_path TEXT NOT NULL,
            dest_rel TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            deployed_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS profile_version (
            id INTEGER PRIMARY KEY,
            version TEXT NOT NULL,
            loaded_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tool_version (
            id INTEGER PRIMARY KEY,
            version TEXT NOT NULL,
            checked_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS job_history (
            id INTEGER PRIMARY KEY,
            kind TEXT NOT NULL,
            source_path TEXT,
            sd_path TEXT,
            summary_json TEXT,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            status TEXT
        );
        CREATE TABLE IF NOT EXISTS job_entries (
            id INTEGER PRIMARY KEY,
            job_id INTEGER NOT NULL,
            source TEXT NOT NULL,
            destination TEXT NOT NULL,
            action TEXT NOT NULL,
            resolved_action TEXT,
            status TEXT NOT NULL,
            hash TEXT,
            size INTEGER,
            content_type TEXT,
            FOREIGN KEY(job_id) REFERENCES job_history(id)
        );
        ",
    )?;
    Ok(conn)
}

pub fn record_job(
    conn: &Connection,
    kind: &str,
    source: &str,
    sd: &str,
    summary: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO job_history (kind, source_path, sd_path, summary_json, started_at, status) VALUES (?1, ?2, ?3, ?4, datetime('now'), 'planned')",
        params![kind, source, sd, summary],
    )?;
    Ok(())
}
