// MCP tool handlers

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::indexer::Indexer;
use crate::query::engine::QueryEngine;

/// Query tool handler
pub async fn query(indexer: &Indexer, args: &HashMap<String, Value>) -> Result<Value> {
    let query_type = args.get("query_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query_type"))?;

    let target = args.get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing target"))?;

    let format = args.get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("text");

    // Execute query using the query engine
    let query_engine = QueryEngine::new(indexer.db().clone());
    let results = match query_type {
        "callers" => query_engine.find_callers(target)?,
        "callees" => query_engine.find_callees(target)?,
        "references" => query_engine.find_references(target)?,
        "dependencies" => query_engine.find_dependencies(target)?,
        _ => return Err(anyhow::anyhow!("Unknown query type: {}", query_type)),
    };

    if format == "json" {
        let json_results: Vec<Value> = results
            .into_iter()
            .map(|r| {
                json!({
                    "symbol_id": r.symbol_id,
                    "qualified_name": r.qualified_name,
                    "file": r.file,
                    "line": r.line,
                    "kind": r.kind
                })
            })
            .collect();

        Ok(json!({
            "query_type": query_type,
            "target": target,
            "results": json_results
        }))
    } else {
        let mut text_results = Vec::new();
        if results.is_empty() {
            text_results.push(format!("No {} found for '{}'", query_type, target));
        } else {
            text_results.push(format!("Found {} {} of '{}':", results.len(), query_type, target));
            for result in results {
                text_results.push(format!("  {}:{} - {} ({})",
                    result.file,
                    result.line,
                    result.qualified_name,
                    result.kind
                ));
            }
        }

        Ok(json!({
            "content": [{
                "type": "text",
                "text": text_results.join("\n")
            }]
        }))
    }
}

/// Search tool handler
pub async fn search(indexer: &Indexer, args: &HashMap<String, Value>) -> Result<Value> {
    let query = args.get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing query"))?;

    let kind = args.get("kind").and_then(|v| v.as_str());
    let limit = args.get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    // Execute search using the query engine
    let query_engine = QueryEngine::new(indexer.db().clone());
    let results = query_engine.search_symbols(query, kind, limit)?;

    let mut text_results = Vec::new();
    if results.is_empty() {
        text_results.push(format!("No symbols found matching '{}'", query));
    } else {
        text_results.push(format!("Found {} symbols matching '{}':", results.len(), query));
        for result in results {
            text_results.push(format!("  {}:{} - {} ({})",
                result.file,
                result.line,
                result.qualified_name,
                result.kind
            ));
        }
    }

    Ok(json!({
        "content": [{
            "type": "text",
            "text": text_results.join("\n")
        }]
    }))
}

/// Stats tool handler
pub async fn stats(indexer: &Indexer, _args: &HashMap<String, Value>) -> Result<Value> {
    let stats = indexer.get_stats()?;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": format!("Index Statistics:\n- Symbols: {}\n- Files: {}\n- Relationships: {}",
                          stats.total_symbols, stats.total_files, stats.total_relationships)
        }]
    }))
}
