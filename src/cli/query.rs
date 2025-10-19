use anyhow::Result;
use std::path::PathBuf;

use crate::config::Config;
use crate::indexer::Indexer;
use crate::query::engine::{QueryEngine, QueryResult};

pub async fn query_index(
    query_type: String,
    target: String,
    project: String,
    format: String,
) -> Result<()> {
    // Load configuration
    let config = Config::from_project_dir(&project);

    println!("CodeGraph Query v0.1.0");
    println!("Query type: {}", query_type);
    println!("Target: {}", target);
    println!("Project: {}", project);
    println!("Format: {}", format);
    println!("Config: {}", if config.project.name != "unnamed-project" { "loaded" } else { "default" });

    // Initialize indexer
    let db_path = PathBuf::from(&project).join(".codegraph.db");
    let indexer = Indexer::new(&db_path)?;
    let query_engine = QueryEngine::new(indexer.db().clone());

    // Execute query
    let results = match query_type.as_str() {
        "callers" => query_engine.find_callers(&target)?,
        "callees" => query_engine.find_callees(&target)?,
        "references" => query_engine.find_references(&target)?,
        "dependencies" => query_engine.find_dependencies(&target)?,
        _ => {
            eprintln!("Unknown query type: {}", query_type);
            std::process::exit(1);
        }
    };

    // Format and display results
    if results.is_empty() {
        println!("\nNo results found for {} of '{}'", query_type, target);
    } else {
        println!("\nFound {} results:", results.len());

        match format.as_str() {
            "json" => {
                let json_results: Vec<serde_json::Value> = results
                    .into_iter()
                    .map(|r| {
                        serde_json::json!({
                            "symbol_id": r.symbol_id,
                            "qualified_name": r.qualified_name,
                            "file": r.file,
                            "line": r.line,
                            "kind": r.kind
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&json_results)?);
            }
            "text" => {
                for result in results {
                    println!("  {}:{} - {} ({})",
                        result.file,
                        result.line,
                        result.qualified_name,
                        result.kind
                    );
                }
            }
            _ => {
                eprintln!("Unknown format: {}", format);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
