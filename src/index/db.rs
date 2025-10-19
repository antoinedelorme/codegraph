use anyhow::{Context, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

use super::schema::init_schema;

/// Type alias for connection pool
pub type ConnectionPool = Pool<SqliteConnectionManager>;

/// Symbol stored in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub kind: SymbolKind,
    pub name: String,
    pub qualified_name: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub signature: Option<String>,
    pub type_: Option<String>,
    pub visibility: Visibility,
    pub language: String,
    pub metadata: Option<String>,
    pub content_hash: String,
    pub last_indexed: u64,
}

impl From<&super::Symbol> for Symbol {
    fn from(symbol: &super::Symbol) -> Self {
        Self {
            id: symbol.id.clone(),
            kind: symbol.kind.clone().into(),
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            file: symbol.location.file.clone(),
            line: symbol.location.line as usize,
            column: symbol.location.column as usize,
            end_line: symbol.location.end_line as usize,
            end_column: symbol.location.end_column as usize,
            signature: symbol.signature.clone(),
            type_: symbol.type_info.clone(),
            visibility: symbol.visibility.clone().into(),
            language: symbol.language.clone(),
            metadata: Some(symbol.metadata.to_string()),
            content_hash: symbol.content_hash.clone(),
            last_indexed: symbol.last_indexed as u64,
        }
    }
}

impl From<super::SymbolKind> for SymbolKind {
    fn from(kind: super::SymbolKind) -> Self {
        match kind {
            super::SymbolKind::Function => Self::Function,
            super::SymbolKind::Type => Self::Type,
            super::SymbolKind::Variable => Self::Variable,
            super::SymbolKind::Context => Self::Context,
            super::SymbolKind::Module => Self::Module,
            super::SymbolKind::Class => Self::Class,
            super::SymbolKind::Method => Self::Method,
            super::SymbolKind::Field => Self::Field,
            super::SymbolKind::Parameter => Self::Parameter,
            super::SymbolKind::Import => Self::Import,
        }
    }
}

/// Symbol kinds
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Type,
    Variable,
    Context,
    Module,
    Class,
    Method,
    Field,
    Parameter,
    Import,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Type => "type",
            SymbolKind::Variable => "variable",
            SymbolKind::Context => "context",
            SymbolKind::Module => "module",
            SymbolKind::Class => "class",
            SymbolKind::Method => "method",
            SymbolKind::Field => "field",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Import => "import",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "function" => Ok(SymbolKind::Function),
            "type" => Ok(SymbolKind::Type),
            "variable" => Ok(SymbolKind::Variable),
            "context" => Ok(SymbolKind::Context),
            "module" => Ok(SymbolKind::Module),
            "class" => Ok(SymbolKind::Class),
            "method" => Ok(SymbolKind::Method),
            "field" => Ok(SymbolKind::Field),
            "parameter" => Ok(SymbolKind::Parameter),
            "import" => Ok(SymbolKind::Import),
            _ => anyhow::bail!("Unknown symbol kind: {}", s),
        }
    }
}

impl From<super::Visibility> for Visibility {
    fn from(vis: super::Visibility) -> Self {
        match vis {
            super::Visibility::Public => Self::Public,
            super::Visibility::Private => Self::Private,
            super::Visibility::Internal => Self::Internal,
        }
    }
}

/// Symbol visibility
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
    Internal,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
            Visibility::Internal => "internal",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "public" => Ok(Visibility::Public),
            "private" => Ok(Visibility::Private),
            "internal" => Ok(Visibility::Internal),
            _ => anyhow::bail!("Unknown visibility: {}", s),
        }
    }
}

impl From<&super::Relationship> for Relationship {
    fn from(rel: &super::Relationship) -> Self {
        Self {
            from_id: rel.from_id.clone(),
            to_id: rel.to_id.clone(),
            type_: rel.kind.clone().into(),
            file: rel.location.file.clone(),
            line: rel.location.line as usize,
            metadata: Some(rel.metadata.to_string()),
        }
    }
}

/// Relationship between symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_id: String,
    pub to_id: String,
    pub type_: RelationshipType,
    pub file: String,
    pub line: usize,
    pub metadata: Option<String>,
}

impl From<super::RelationshipKind> for RelationshipType {
    fn from(kind: super::RelationshipKind) -> Self {
        match kind {
            super::RelationshipKind::Calls => Self::Calls,
            super::RelationshipKind::References => Self::References,
            super::RelationshipKind::DependsOn => Self::DependsOn,
            super::RelationshipKind::Defines => Self::Defines,
            super::RelationshipKind::Implements => Self::Implements,
            super::RelationshipKind::Extends => Self::Extends,
            super::RelationshipKind::Contains => Self::Contains,
            super::RelationshipKind::Imports => Self::References, // Map to references for now
        }
    }
}

/// Relationship types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    Calls,
    References,
    DependsOn,
    Defines,
    Implements,
    Extends,
    Contains,
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::Calls => "calls",
            RelationshipType::References => "references",
            RelationshipType::DependsOn => "depends_on",
            RelationshipType::Defines => "defines",
            RelationshipType::Implements => "implements",
            RelationshipType::Extends => "extends",
            RelationshipType::Contains => "contains",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "calls" => Ok(RelationshipType::Calls),
            "references" => Ok(RelationshipType::References),
            "depends_on" => Ok(RelationshipType::DependsOn),
            "defines" => Ok(RelationshipType::Defines),
            "implements" => Ok(RelationshipType::Implements),
            "extends" => Ok(RelationshipType::Extends),
            "contains" => Ok(RelationshipType::Contains),
            _ => anyhow::bail!("Unknown relationship type: {}", s),
        }
    }
}

/// Database connection manager
#[derive(Clone)]
pub struct IndexDatabase {
    pool: ConnectionPool,
    db_path: PathBuf,
}

impl IndexDatabase {
    /// Create or open a database
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        info!("Opening database at: {}", db_path.display());

        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Create connection manager
        let manager = SqliteConnectionManager::file(&db_path);

        // Create connection pool
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .context("Failed to create connection pool")?;

        // Initialize schema
        {
            let conn = pool.get().context("Failed to get connection")?;
            init_schema(&conn).context("Failed to initialize schema")?;
        }

        Ok(Self { pool, db_path })
    }

    /// Get a connection from the pool
    pub fn get_conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool.get().context("Failed to get connection from pool")
    }

    /// Insert a symbol
    pub fn insert_symbol(&self, symbol: &Symbol) -> Result<()> {
        let conn = self.get_conn()?;

        debug!("Inserting symbol: {}", symbol.qualified_name);

        conn.execute(
            "INSERT OR REPLACE INTO symbols (
                id, kind, name, qualified_name, file, line, column, end_line, end_column,
                signature, type, visibility, language, metadata, content_hash, last_indexed
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                symbol.id,
                symbol.kind.as_str(),
                symbol.name,
                symbol.qualified_name,
                symbol.file,
                symbol.line as i64,
                symbol.column as i64,
                symbol.end_line as i64,
                symbol.end_column as i64,
                symbol.signature,
                symbol.type_,
                symbol.visibility.as_str(),
                symbol.language,
                symbol.metadata,
                symbol.content_hash,
                symbol.last_indexed as i64,
            ],
        )?;

        Ok(())
    }

    /// Get symbol by ID
    pub fn get_symbol(&self, id: &str) -> Result<Option<Symbol>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "SELECT id, kind, name, qualified_name, file, line, column, end_line, end_column,
                    signature, type, visibility, language, metadata, content_hash, last_indexed
             FROM symbols WHERE id = ?1",
        )?;

        let symbol = stmt
            .query_row([id], |row| Ok(row_to_symbol(row)?))
            .optional()?;

        Ok(symbol)
    }

    /// Find symbols by qualified name
    pub fn find_symbols_by_name(&self, qualified_name: &str) -> Result<Vec<Symbol>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "SELECT id, kind, name, qualified_name, file, line, column, end_line, end_column,
                    signature, type, visibility, language, metadata, content_hash, last_indexed
             FROM symbols WHERE qualified_name = ?1",
        )?;

        let symbols = stmt
            .query_map([qualified_name], |row| Ok(row_to_symbol(row)?))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(symbols)
    }

    /// Find symbols by file
    pub fn find_symbols_by_file(&self, file: &str) -> Result<Vec<Symbol>> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "SELECT id, kind, name, qualified_name, file, line, column, end_line, end_column,
                    signature, type, visibility, language, metadata, content_hash, last_indexed
             FROM symbols WHERE file = ?1 ORDER BY line",
        )?;

        let symbols = stmt
            .query_map([file], |row| Ok(row_to_symbol(row)?))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(symbols)
    }

    /// Delete symbols by file
    pub fn delete_symbols_by_file(&self, file: &str) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute("DELETE FROM symbols WHERE file = ?1", [file])?;
        Ok(())
    }

    /// Insert a relationship
    pub fn insert_relationship(&self, rel: &Relationship) -> Result<()> {
        let conn = self.get_conn()?;

        debug!("Inserting relationship: {} -> {}", rel.from_id, rel.to_id);

        conn.execute(
            "INSERT INTO relationships (from_id, to_id, type, file, line, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                rel.from_id,
                rel.to_id,
                rel.type_.as_str(),
                rel.file,
                rel.line as i64,
                rel.metadata,
            ],
        )?;

        Ok(())
    }

    /// Find relationships from a symbol
    pub fn find_relationships_from(&self, from_id: &str, type_: Option<RelationshipType>) -> Result<Vec<Relationship>> {
        let conn = self.get_conn()?;

        let relationships = if let Some(type_) = type_ {
            let mut stmt = conn.prepare(
                "SELECT from_id, to_id, type, file, line, metadata
                 FROM relationships WHERE from_id = ?1 AND type = ?2",
            )?;

            let result = stmt.query_map(params![from_id, type_.as_str()], |row| Ok(row_to_relationship(row)?))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            result
        } else {
            let mut stmt = conn.prepare(
                "SELECT from_id, to_id, type, file, line, metadata
                 FROM relationships WHERE from_id = ?1",
            )?;

            let result = stmt.query_map([from_id], |row| Ok(row_to_relationship(row)?))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            result
        };

        Ok(relationships)
    }

    /// Find relationships to a symbol
    pub fn find_relationships_to(&self, to_id: &str, type_: Option<RelationshipType>) -> Result<Vec<Relationship>> {
        let conn = self.get_conn()?;

        let relationships = if let Some(type_) = type_ {
            let mut stmt = conn.prepare(
                "SELECT from_id, to_id, type, file, line, metadata
                 FROM relationships WHERE to_id = ?1 AND type = ?2",
            )?;

            let result = stmt.query_map(params![to_id, type_.as_str()], |row| Ok(row_to_relationship(row)?))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            result
        } else {
            let mut stmt = conn.prepare(
                "SELECT from_id, to_id, type, file, line, metadata
                 FROM relationships WHERE to_id = ?1",
            )?;

            let result = stmt.query_map([to_id], |row| Ok(row_to_relationship(row)?))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            result
        };

        Ok(relationships)
    }

    /// Get index statistics
    pub fn get_stats(&self) -> Result<IndexStats> {
        let conn = self.get_conn()?;

        let total_symbols: i64 = conn.query_row(
            "SELECT COUNT(*) FROM symbols",
            [],
            |row| row.get(0),
        )?;

        let total_files: i64 = conn.query_row(
            "SELECT COUNT(*) FROM files",
            [],
            |row| row.get(0),
        )?;

        let total_relationships: i64 = conn.query_row(
            "SELECT COUNT(*) FROM relationships",
            [],
            |row| row.get(0),
        )?;

        Ok(IndexStats {
            total_symbols: total_symbols as usize,
            total_files: total_files as usize,
            total_relationships: total_relationships as usize,
        })
    }

    /// Update file indexing metadata
    pub fn update_file_indexed(&self, file_path: &str, language: &str, content_hash: String, symbol_count: i64) -> Result<()> {
        let conn = self.get_conn()?;
        let now = now();

        conn.execute(
            "INSERT OR REPLACE INTO files (path, language, content_hash, last_indexed, symbol_count, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![file_path, language, content_hash, now, symbol_count, now],
        )?;

        Ok(())
    }

    /// Clear all data (for testing)
    pub fn clear(&self) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute("DELETE FROM relationships", [])?;
        conn.execute("DELETE FROM symbols", [])?;
        conn.execute("DELETE FROM files", [])?;
        Ok(())
    }
}

/// Index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_symbols: usize,
    pub total_files: usize,
    pub total_relationships: usize,
}

/// Convert database row to Symbol
fn row_to_symbol(row: &Row) -> rusqlite::Result<Symbol> {
    let kind_str: String = row.get(1)?;
    let visibility_str: String = row.get(11)?;

    Ok(Symbol {
        id: row.get(0)?,
        kind: SymbolKind::from_str(&kind_str).unwrap(),
        name: row.get(2)?,
        qualified_name: row.get(3)?,
        file: row.get(4)?,
        line: row.get::<_, i64>(5)? as usize,
        column: row.get::<_, i64>(6)? as usize,
        end_line: row.get::<_, i64>(7)? as usize,
        end_column: row.get::<_, i64>(8)? as usize,
        signature: row.get(9)?,
        type_: row.get(10)?,
        visibility: Visibility::from_str(&visibility_str).unwrap(),
        language: row.get(12)?,
        metadata: row.get(13)?,
        content_hash: row.get(14)?,
        last_indexed: row.get::<_, i64>(15)? as u64,
    })
}

/// Convert database row to Relationship
fn row_to_relationship(row: &Row) -> rusqlite::Result<Relationship> {
    let type_str: String = row.get(2)?;

    Ok(Relationship {
        from_id: row.get(0)?,
        to_id: row.get(1)?,
        type_: RelationshipType::from_str(&type_str).unwrap(),
        file: row.get(3)?,
        line: row.get::<_, i64>(4)? as usize,
        metadata: row.get(5)?,
    })
}

/// Get current timestamp in seconds
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_database() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let db = IndexDatabase::new(&db_path).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn test_insert_and_get_symbol() {
        let dir = tempdir().unwrap();
        let db = IndexDatabase::new(dir.path().join("test.db")).unwrap();

        let symbol = Symbol {
            id: "test::hello".to_string(),
            kind: SymbolKind::Function,
            name: "hello".to_string(),
            qualified_name: "test::hello".to_string(),
            file: "test.intent".to_string(),
            line: 1,
            column: 4,
            end_line: 3,
            end_column: 1,
            signature: Some("fn hello()".to_string()),
            type_: None,
            visibility: Visibility::Public,
            language: "intent".to_string(),
            metadata: None,
            content_hash: "abc123".to_string(),
            last_indexed: now(),
        };

        db.insert_symbol(&symbol).unwrap();

        let retrieved = db.get_symbol("test::hello").unwrap().unwrap();
        assert_eq!(retrieved.qualified_name, "test::hello");
        assert_eq!(retrieved.kind, SymbolKind::Function);
    }

    #[test]
    fn test_insert_relationship() {
        let dir = tempdir().unwrap();
        let db = IndexDatabase::new(dir.path().join("test.db")).unwrap();

        // Insert symbols first
        let symbol1 = Symbol {
            id: "main".to_string(),
            kind: SymbolKind::Function,
            name: "main".to_string(),
            qualified_name: "main".to_string(),
            file: "main.intent".to_string(),
            line: 1,
            column: 4,
            end_line: 5,
            end_column: 1,
            signature: Some("fn main()".to_string()),
            type_: None,
            visibility: Visibility::Public,
            language: "intent".to_string(),
            metadata: None,
            content_hash: "abc".to_string(),
            last_indexed: now(),
        };

        let symbol2 = Symbol {
            id: "hello".to_string(),
            kind: SymbolKind::Function,
            name: "hello".to_string(),
            qualified_name: "hello".to_string(),
            file: "main.intent".to_string(),
            line: 7,
            column: 4,
            end_line: 9,
            end_column: 1,
            signature: Some("fn hello()".to_string()),
            type_: None,
            visibility: Visibility::Public,
            language: "intent".to_string(),
            metadata: None,
            content_hash: "def".to_string(),
            last_indexed: now(),
        };

        db.insert_symbol(&symbol1).unwrap();
        db.insert_symbol(&symbol2).unwrap();

        // Insert relationship
        let rel = Relationship {
            from_id: "main".to_string(),
            to_id: "hello".to_string(),
            type_: RelationshipType::Calls,
            file: "main.intent".to_string(),
            line: 2,
            metadata: None,
        };

        db.insert_relationship(&rel).unwrap();

        // Query relationships
        let rels = db.find_relationships_from("main", Some(RelationshipType::Calls)).unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].to_id, "hello");
    }

    #[test]
    fn test_stats() {
        let dir = tempdir().unwrap();
        let db = IndexDatabase::new(dir.path().join("test.db")).unwrap();

        let stats = db.get_stats().unwrap();
        assert_eq!(stats.total_symbols, 0);
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_relationships, 0);
    }
}
