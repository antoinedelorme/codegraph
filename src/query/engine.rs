// Query execution engine

use anyhow::Result;
use std::collections::HashSet;

use crate::indexer::Indexer;
use crate::index::db::{IndexDatabase, RelationshipType};

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub symbol_id: String,
    pub qualified_name: String,
    pub file: String,
    pub line: usize,
    pub kind: String,
}

/// Query engine
pub struct QueryEngine {
    db: IndexDatabase,
}

impl QueryEngine {
    pub fn new(db: IndexDatabase) -> Self {
        Self { db }
    }

    /// Find all callers of a symbol
    pub fn find_callers(&self, target_symbol: &str) -> Result<Vec<QueryResult>> {
        // Find all target symbols with this name
        let symbols = self.db.find_symbols_by_name(target_symbol)?;
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for symbol in symbols {
            let relationships = self.db.find_relationships_to(&symbol.id, Some(RelationshipType::Calls))?;

            for rel in relationships {
                if let Some(caller_symbol) = self.db.get_symbol(&rel.from_id)? {
                    results.push(QueryResult {
                        symbol_id: caller_symbol.id,
                        qualified_name: caller_symbol.qualified_name,
                        file: caller_symbol.file,
                        line: caller_symbol.line,
                        kind: caller_symbol.kind.as_str().to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Find all callees of a symbol
    pub fn find_callees(&self, target_symbol: &str) -> Result<Vec<QueryResult>> {
        // Find all target symbols with this name
        let symbols = self.db.find_symbols_by_name(target_symbol)?;
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for symbol in symbols {
            let relationships = self.db.find_relationships_from(&symbol.id, Some(RelationshipType::Calls))?;

            for rel in relationships {
                if let Some(callee_symbol) = self.db.get_symbol(&rel.to_id)? {
                    results.push(QueryResult {
                        symbol_id: callee_symbol.id,
                        qualified_name: callee_symbol.qualified_name,
                        file: callee_symbol.file,
                        line: callee_symbol.line,
                        kind: callee_symbol.kind.as_str().to_string(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Find all references to a symbol
    pub fn find_references(&self, target_symbol: &str) -> Result<Vec<QueryResult>> {
        // Find the target symbol first
        let symbols = self.db.find_symbols_by_name(target_symbol)?;
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let target_id = &symbols[0].id;
        let relationships = self.db.find_relationships_to(target_id, Some(RelationshipType::References))?;

        let mut results = Vec::new();
        for rel in relationships {
            if let Some(referrer_symbol) = self.db.get_symbol(&rel.from_id)? {
                results.push(QueryResult {
                    symbol_id: referrer_symbol.id,
                    qualified_name: referrer_symbol.qualified_name,
                    file: referrer_symbol.file,
                    line: rel.line,
                    kind: referrer_symbol.kind.as_str().to_string(),
                });
            }
        }

        Ok(results)
    }

    /// Find dependencies of a symbol
    pub fn find_dependencies(&self, target_symbol: &str) -> Result<Vec<QueryResult>> {
        // For now, dependencies are similar to references
        // TODO: Implement more sophisticated dependency analysis
        self.find_references(target_symbol)
    }

    /// Search for symbols by name
    pub fn search_symbols(&self, query: &str, kind: Option<&str>, limit: usize) -> Result<Vec<QueryResult>> {
        // Use a simple LIKE query for now
        // TODO: Implement full-text search
        let conn = self.db.get_conn()?;
        let pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT id, kind, name, qualified_name, file, line, column, end_line, end_column,
                    signature, type, visibility, language, metadata, content_hash, last_indexed
             FROM symbols
             WHERE qualified_name LIKE ?1
             ORDER BY qualified_name
             LIMIT ?2",
        )?;

        let symbols = stmt.query_map([pattern, limit.to_string()], |row| {
            Ok(crate::index::db::Symbol {
                id: row.get(0)?,
                kind: crate::index::db::SymbolKind::from_str(&row.get::<_, String>(1)?).unwrap(),
                name: row.get(2)?,
                qualified_name: row.get(3)?,
                file: row.get(4)?,
                line: row.get::<_, i64>(5)? as usize,
                column: row.get::<_, i64>(6)? as usize,
                end_line: row.get::<_, i64>(7)? as usize,
                end_column: row.get::<_, i64>(8)? as usize,
                signature: row.get(9)?,
                type_: row.get(10)?,
                visibility: crate::index::db::Visibility::from_str(&row.get::<_, String>(11)?).unwrap(),
                language: row.get(12)?,
                metadata: row.get(13)?,
                content_hash: row.get(14)?,
                last_indexed: row.get::<_, i64>(15)? as u64,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut results = Vec::new();
        for symbol in symbols {
            if let Some(kind_filter) = kind {
                if symbol.kind.as_str() != kind_filter {
                    continue;
                }
            }

            results.push(QueryResult {
                symbol_id: symbol.id,
                qualified_name: symbol.qualified_name,
                file: symbol.file,
                line: symbol.line,
                kind: symbol.kind.as_str().to_string(),
            });
        }

        Ok(results)
    }
}
