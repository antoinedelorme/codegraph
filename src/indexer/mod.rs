// Code indexing and file watching

pub mod watcher;
pub mod parser;

use std::collections::HashMap;
use std::path::Path;
use crate::index::{Parser, Symbol, Relationship};
use crate::index::db::IndexDatabase;

/// The main indexer that coordinates parsing and storage
pub struct Indexer {
    parsers: HashMap<String, Box<dyn Parser + Send + Sync>>,
    db: IndexDatabase,
}

impl Indexer {
    pub fn new(db_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut parsers = HashMap::new();

        // Register parsers
        parsers.insert("python".to_string(), Box::new(parser::PythonParser::new()) as Box<dyn Parser + Send + Sync>);
        parsers.insert("rust".to_string(), Box::new(parser::RustParser::new()) as Box<dyn Parser + Send + Sync>);
        parsers.insert("go".to_string(), Box::new(parser::GoParser::new()) as Box<dyn Parser + Send + Sync>);
        parsers.insert("java".to_string(), Box::new(parser::JavaParser::new()) as Box<dyn Parser + Send + Sync>);
        parsers.insert("intent".to_string(), Box::new(parser::IntentParser::new()) as Box<dyn Parser + Send + Sync>);

        let db = IndexDatabase::new(db_path)?;

        Ok(Self { parsers, db })
    }

    pub fn can_index_file(&self, file_path: &str) -> bool {
        self.parsers.values().any(|p| p.can_parse(file_path))
    }

    pub fn get_parser_for_file(&self, file_path: &str) -> Option<&(dyn Parser + Send + Sync)> {
        self.parsers.values()
            .find(|p| p.can_parse(file_path))
            .map(|p| p.as_ref())
    }

    pub async fn index_file(&self, file_path: &str, content: &str) -> anyhow::Result<(Vec<Symbol>, Vec<Relationship>)> {
        let parser = self.get_parser_for_file(file_path)
            .ok_or_else(|| anyhow::anyhow!("No parser available for file: {}", file_path))?;

        let (symbols, _) = parser.parse(content, file_path)?;

        // Store symbols in database
        for symbol in &symbols {
            let db_symbol = symbol.into();
            self.db.insert_symbol(&db_symbol)?;
        }

        // Update file metadata
        let content_hash = blake3::hash(content.as_bytes()).to_string();
        let language = if file_path.ends_with(".py") {
            "python"
        } else if file_path.ends_with(".rs") {
            "rust"
        } else if file_path.ends_with(".go") {
            "go"
        } else if file_path.ends_with(".java") {
            "java"
        } else if file_path.ends_with(".intent") {
            "intent"
        } else {
            "unknown"
        };
        self.db.update_file_indexed(file_path, language, content_hash, symbols.len() as i64)?;

        // Return symbols but no relationships yet - we'll extract them later with global context
        Ok((symbols, Vec::new()))
    }

    pub async fn extract_relationships(&self, file_path: &str, content: &str, all_symbols: &[Symbol]) -> anyhow::Result<Vec<Relationship>> {
        let parser = self.get_parser_for_file(file_path)
            .ok_or_else(|| anyhow::anyhow!("No parser available for file: {}", file_path))?;

        // Create a global symbol map
        let global_symbol_map: std::collections::HashMap<&str, &Symbol> = all_symbols.iter()
            .map(|s| (s.qualified_name.as_str(), s))
            .collect();

        // Extract relationships using the global context
        let relationships = parser.extract_relationships_with_global_context(content, file_path, &global_symbol_map)?;

        // Store relationships in database
        for relationship in &relationships {
            let db_relationship = relationship.into();
            self.db.insert_relationship(&db_relationship)?;
        }

        Ok(relationships)
    }

    pub fn get_stats(&self) -> anyhow::Result<crate::index::db::IndexStats> {
        self.db.get_stats()
    }

    pub fn db(&self) -> &crate::index::db::IndexDatabase {
        &self.db
    }
}

// TODO: Implement indexer
// - File scanner
// - Language parsers
// - Symbol extraction
// - Incremental updates
