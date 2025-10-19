// Index storage and schema

pub mod schema;
pub mod db;

/// A code symbol (function, type, variable, etc.)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Symbol {
    pub id: String,
    pub kind: SymbolKind,
    pub name: String,
    pub qualified_name: String,
    pub location: Location,
    pub signature: Option<String>,
    pub type_info: Option<String>,
    pub visibility: Visibility,
    pub language: String,
    pub metadata: serde_json::Value,
    pub content_hash: String,
    pub last_indexed: i64,
}

/// Symbol kinds
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// Visibility levels
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Internal,
}

/// Location in source code
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

/// Relationship between symbols
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Relationship {
    pub from_id: String,
    pub to_id: String,
    pub kind: RelationshipKind,
    pub location: Location,
    pub metadata: serde_json::Value,
}

/// Relationship kinds
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RelationshipKind {
    Calls,
    References,
    DependsOn,
    Defines,
    Implements,
    Extends,
    Contains,
    Imports,
}

/// Parser trait for different languages
pub trait Parser {
    fn can_parse(&self, file_path: &str) -> bool;
    fn parse(&self, content: &str, file_path: &str) -> anyhow::Result<(Vec<Symbol>, Vec<Relationship>)>;
    fn extract_relationships_with_global_context(&self, content: &str, file_path: &str, global_symbol_map: &std::collections::HashMap<&str, &Symbol>) -> anyhow::Result<Vec<Relationship>>;
}

// TODO: Implement index storage
// - SQLite database
// - Symbol table
// - Relationship graph
// - Query cache
