use anyhow::Result;
use std::path::PathBuf;

use crate::config::Config;
use crate::indexer::Indexer;

pub async fn show_stats(project: String, verbose: bool) -> Result<()> {
    // Load configuration
    let config = Config::from_project_dir(&project);

    println!("CodeGraph Statistics v0.1.0");
    println!("Project: {}", project);
    println!("Config: {}", if config.project.name != "unnamed-project" { "loaded" } else { "default" });

    // Initialize indexer
    let db_path = PathBuf::from(&project).join(".codegraph.db");
    let indexer = Indexer::new(&db_path)?;

    // Get basic stats
    let stats = indexer.get_stats()?;

    println!("\n📊 Index Statistics:");
    println!("  Total files: {}", stats.total_files);
    println!("  Total symbols: {}", stats.total_symbols);
    println!("  Total relationships: {}", stats.total_relationships);

    // Calculate index size
    let db_size = get_db_size(&db_path)?;
    println!("  Index size: {:.2} MB", db_size);

    if verbose {
        println!("\n📈 Detailed Statistics:");

        // Get symbols by kind
        let symbols_by_kind = get_symbols_by_kind(&indexer)?;
        if !symbols_by_kind.is_empty() {
            println!("  Symbols by kind:");
            for (kind, count) in symbols_by_kind {
                println!("    {}: {}", kind, count);
            }
        }

        // Get languages breakdown
        let languages = get_languages_breakdown(&indexer)?;
        if !languages.is_empty() {
            println!("  Languages:");
            for (lang, count) in languages {
                println!("    {}: {} files", lang, count);
            }
        }

        // Get relationships by type
        let relationships_by_type = get_relationships_by_type(&indexer)?;
        if !relationships_by_type.is_empty() {
            println!("  Relationships by type:");
            for (rel_type, count) in relationships_by_type {
                println!("    {}: {}", rel_type, count);
            }
        }
    }

    Ok(())
}

fn get_db_size(db_path: &PathBuf) -> Result<f64> {
    let metadata = std::fs::metadata(db_path)?;
    let size_bytes = metadata.len() as f64;
    let size_mb = size_bytes / (1024.0 * 1024.0);
    Ok(size_mb)
}

fn get_symbols_by_kind(indexer: &Indexer) -> Result<Vec<(String, usize)>> {
    let db = indexer.db();
    let conn = db.get_conn()?;

    let mut stmt = conn.prepare(
        "SELECT kind, COUNT(*) as count FROM symbols GROUP BY kind ORDER BY count DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        let kind: String = row.get(0)?;
        let count: usize = row.get(1)?;
        Ok((kind, count))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }

    Ok(results)
}

fn get_languages_breakdown(indexer: &Indexer) -> Result<Vec<(String, usize)>> {
    let db = indexer.db();
    let conn = db.get_conn()?;

    let mut stmt = conn.prepare(
        "SELECT language, COUNT(*) as count FROM files GROUP BY language ORDER BY count DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        let language: String = row.get(0)?;
        let count: usize = row.get(1)?;
        Ok((language, count))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }

    Ok(results)
}

fn get_relationships_by_type(indexer: &Indexer) -> Result<Vec<(String, usize)>> {
    let db = indexer.db();
    let conn = db.get_conn()?;

    let mut stmt = conn.prepare(
        "SELECT type, COUNT(*) as count FROM relationships GROUP BY type ORDER BY count DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        let rel_type: String = row.get(0)?;
        let count: usize = row.get(1)?;
        Ok((rel_type, count))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }

    Ok(results)
}
