use anyhow::Result;
use std::path::PathBuf;

use crate::config::Config;
use crate::indexer::Indexer;
use crate::query::engine::QueryEngine;

pub async fn analyze_impact(
    change_type: String,
    target: String,
    to: Option<String>,
    project: String,
) -> Result<()> {
    // Load configuration
    let config = Config::from_project_dir(&project);

    println!("CodeGraph Impact Analysis v0.1.0");
    println!("Change type: {}", change_type);
    println!("Target: {}", target);
    if let Some(ref new_value) = to {
        println!("To: {}", new_value);
    }
    println!("Project: {}", project);
    println!("Config: {}", if config.project.name != "unnamed-project" { "loaded" } else { "default" });

    // Initialize indexer and query engine
    let db_path = PathBuf::from(&project).join(".codegraph.db");
    let indexer = Indexer::new(&db_path)?;
    let query_engine = QueryEngine::new(indexer.db().clone());

    match change_type.as_str() {
        "rename" => {
            if let Some(ref new_name) = to {
                analyze_rename_impact(&query_engine, &target, new_name).await?;
            } else {
                eprintln!("Error: --to parameter required for rename");
                std::process::exit(1);
            }
        }
        "delete" => {
            analyze_delete_impact(&query_engine, &target).await?;
        }
        "change_type" => {
            if let Some(ref new_type) = to {
                analyze_type_change_impact(&query_engine, &target, new_type).await?;
            } else {
                eprintln!("Error: --to parameter required for change_type");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown change type: {}", change_type);
            eprintln!("Supported types: rename, delete, change_type");
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn analyze_rename_impact(
    query_engine: &QueryEngine,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    println!("\n🔄 Analyzing rename impact: {} → {}", old_name, new_name);

    // Find all usages of the symbol (callers)
    let callers = query_engine.find_callers(old_name)?;

    if callers.is_empty() {
        println!("✅ No usages found - safe to rename");
        return Ok(());
    }

    println!("⚠️  Found {} usages that would break:", callers.len());

    for caller in &callers {
        println!("  📍 {}:{} - {} ({})",
            caller.file,
            caller.line,
            caller.qualified_name,
            caller.kind
        );
    }

    println!("\n💡 Recommendation: Update all {} usages", callers.len());

    Ok(())
}

async fn analyze_delete_impact(query_engine: &QueryEngine, target: &str) -> Result<()> {
    println!("\n🗑️  Analyzing delete impact: {}", target);

    // Find all callers of the symbol
    let callers = query_engine.find_callers(target)?;

    // Find all callees (what this symbol calls)
    let callees = query_engine.find_callees(target)?;

    let total_impacts = callers.len() + callees.len();

    if total_impacts == 0 {
        println!("✅ No dependencies found - safe to delete");
        return Ok(());
    }

    println!("⚠️  Found {} impacts:", total_impacts);

    if !callers.is_empty() {
        println!("  📍 {} callers to update:", callers.len());
        for caller in &callers {
            println!("    {}:{} - {} ({})",
                caller.file,
                caller.line,
                caller.qualified_name,
                caller.kind
            );
        }
    }

    if !callees.is_empty() {
        println!("  🔗 {} callees that may become unreachable:", callees.len());
        for callee in &callees {
            println!("    {}:{} - {} ({})",
                callee.file,
                callee.line,
                callee.qualified_name,
                callee.kind
            );
        }
    }

    println!("\n💡 Recommendation: Update callers and consider callees");

    Ok(())
}

async fn analyze_type_change_impact(
    query_engine: &QueryEngine,
    target: &str,
    new_type: &str,
) -> Result<()> {
    println!("\n🔧 Analyzing type change impact: {} → {}", target, new_type);

    // Find all callers of the symbol
    let callers = query_engine.find_callers(target)?;

    if callers.is_empty() {
        println!("✅ No usages found - safe to change type");
        return Ok(());
    }

    println!("⚠️  Found {} usages that may need updates:", callers.len());

    for caller in &callers {
        println!("  📍 {}:{} - {} ({})",
            caller.file,
            caller.line,
            caller.qualified_name,
            caller.kind
        );
    }

    println!("\n💡 Recommendation: Check type compatibility for all {} usages", callers.len());

    Ok(())
}
