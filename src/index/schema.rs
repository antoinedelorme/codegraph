use rusqlite::{Connection, Result};
use tracing::{info, debug};

/// SQLite schema version
pub const SCHEMA_VERSION: i32 = 1;

/// Initialize the database schema
pub fn init_schema(conn: &Connection) -> Result<()> {
    info!("Initializing CodeGraph schema v{}", SCHEMA_VERSION);

    // Create schema version table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Check current version
    let current_version: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    debug!("Current schema version: {}", current_version);

    if current_version < SCHEMA_VERSION {
        info!("Upgrading schema from v{} to v{}", current_version, SCHEMA_VERSION);
        apply_migrations(conn, current_version)?;
    }

    Ok(())
}

/// Apply migrations from current version to latest
fn apply_migrations(conn: &Connection, from_version: i32) -> Result<()> {
    for version in (from_version + 1)..=SCHEMA_VERSION {
        info!("Applying migration v{}", version);
        match version {
            1 => create_v1_schema(conn)?,
            _ => unreachable!("Unknown schema version: {}", version),
        }

        // Record migration
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            [version],
        )?;
    }

    Ok(())
}

/// Create v1 schema (initial schema)
fn create_v1_schema(conn: &Connection) -> Result<()> {
    info!("Creating v1 schema tables");

    // Symbols table - stores all code symbols (functions, types, variables, contexts)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS symbols (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            qualified_name TEXT NOT NULL,
            file TEXT NOT NULL,
            line INTEGER NOT NULL,
            column INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            end_column INTEGER NOT NULL,
            signature TEXT,
            type TEXT,
            visibility TEXT,
            language TEXT NOT NULL,
            metadata TEXT,
            content_hash TEXT NOT NULL,
            last_indexed INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Indexes for symbols table
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_symbols_qualified_name
         ON symbols(qualified_name)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_symbols_file
         ON symbols(file)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_symbols_kind
         ON symbols(kind)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_symbols_name
         ON symbols(name)",
        [],
    )?;

    // Relationships table - stores connections between symbols
    conn.execute(
        "CREATE TABLE IF NOT EXISTS relationships (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_id TEXT NOT NULL,
            to_id TEXT NOT NULL,
            type TEXT NOT NULL,
            file TEXT NOT NULL,
            line INTEGER NOT NULL,
            metadata TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (from_id) REFERENCES symbols(id) ON DELETE CASCADE,
            FOREIGN KEY (to_id) REFERENCES symbols(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Indexes for relationships table
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_relationships_from
         ON relationships(from_id, type)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_relationships_to
         ON relationships(to_id, type)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_relationships_type
         ON relationships(type)",
        [],
    )?;

    // Full-text search table for symbols
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
            qualified_name,
            signature,
            content='symbols',
            content_rowid='rowid'
        )",
        [],
    )?;

    // Trigger to keep FTS table in sync
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS symbols_fts_insert AFTER INSERT ON symbols
         BEGIN
             INSERT INTO symbols_fts(rowid, qualified_name, signature)
             VALUES (new.rowid, new.qualified_name, new.signature);
         END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS symbols_fts_delete AFTER DELETE ON symbols
         BEGIN
             DELETE FROM symbols_fts WHERE rowid = old.rowid;
         END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS symbols_fts_update AFTER UPDATE ON symbols
         BEGIN
             UPDATE symbols_fts
             SET qualified_name = new.qualified_name,
                 signature = new.signature
             WHERE rowid = new.rowid;
         END",
        [],
    )?;

    // Files table - track which files are indexed
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            language TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            last_indexed INTEGER NOT NULL,
            symbol_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_files_language
         ON files(language)",
        [],
    )?;

    // Index statistics table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS index_stats (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Insert initial stats
    conn.execute(
        "INSERT OR IGNORE INTO index_stats (key, value)
         VALUES ('total_symbols', '0')",
        [],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO index_stats (key, value)
         VALUES ('total_files', '0')",
        [],
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO index_stats (key, value)
         VALUES ('last_full_index', '0')",
        [],
    )?;

    info!("v1 schema created successfully");

    Ok(())
}

/// Drop all tables (for testing/rebuilding)
pub fn drop_schema(conn: &Connection) -> Result<()> {
    info!("Dropping all schema tables");

    conn.execute("DROP TABLE IF EXISTS schema_version", [])?;
    conn.execute("DROP TABLE IF EXISTS index_stats", [])?;
    conn.execute("DROP TABLE IF EXISTS files", [])?;
    conn.execute("DROP TRIGGER IF EXISTS symbols_fts_update", [])?;
    conn.execute("DROP TRIGGER IF EXISTS symbols_fts_delete", [])?;
    conn.execute("DROP TRIGGER IF EXISTS symbols_fts_insert", [])?;
    conn.execute("DROP TABLE IF EXISTS symbols_fts", [])?;
    conn.execute("DROP TABLE IF EXISTS relationships", [])?;
    conn.execute("DROP TABLE IF EXISTS symbols", [])?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_init_schema() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert!(tables.contains(&"symbols".to_string()));
        assert!(tables.contains(&"relationships".to_string()));
        assert!(tables.contains(&"files".to_string()));
        assert!(tables.contains(&"schema_version".to_string()));
    }

    #[test]
    fn test_schema_version() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        let version: i32 = conn
            .query_row(
                "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_drop_schema() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        drop_schema(&conn).unwrap();

        // Verify tables are gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_idempotent_init() {
        let conn = Connection::open_in_memory().unwrap();

        // Init twice should not error
        init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();

        let version: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Should only have one version record
        assert_eq!(version, 1);
    }
}
