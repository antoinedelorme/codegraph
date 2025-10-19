use anyhow::Result;
use std::path::PathBuf;
use tracing::info;
use walkdir::WalkDir;

use crate::config::Config;
use crate::indexer::Indexer;

pub async fn index_project(
    project: String,
    languages: Option<String>,
    watch: bool,
    rebuild: bool,
) -> Result<()> {
    info!("Indexing project: {}", project);

    // Load configuration
    let config = Config::from_project_dir(&project);

    println!("CodeGraph Indexer v0.1.0");
    println!("Project: {}", project);
    println!("Config: {}", if config.project.name != "unnamed-project" { "loaded" } else { "default" });

    // Determine enabled languages (CLI override or config)
    let enabled_languages = if let Some(langs) = languages {
        langs.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>()
    } else {
        config.get_enabled_languages()
    };
    println!("Languages: {}", enabled_languages.join(", "));

    // Determine watch setting (CLI override or config)
    let should_watch = watch || config.indexing.watch;
    println!("Watch: {}", should_watch);
    println!("Rebuild: {}", rebuild);

    // Initialize database
    let db_path = PathBuf::from(&project).join(".codegraph.db");
    println!("Database: {}", db_path.display());

    // Basic file scanning
    println!("\nScanning project files...");
    let indexer = Indexer::new(&db_path)?;
    let mut python_files = Vec::new();
    let mut rust_files = Vec::new();
    let mut go_files = Vec::new();
    let mut java_files = Vec::new();
    let mut intent_files = Vec::new();
    let mut other_files = Vec::new();

    for entry in WalkDir::new(&project).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let path_str = path.to_string_lossy().to_string();

            // Check if file should be indexed based on config patterns
            if config.should_index_file(&path_str) && indexer.can_index_file(&path_str) {
                // Check if language is enabled
                let is_enabled = if path_str.ends_with(".py") {
                    enabled_languages.contains(&"python".to_string())
                } else if path_str.ends_with(".rs") {
                    enabled_languages.contains(&"rust".to_string())
                } else if path_str.ends_with(".go") {
                    enabled_languages.contains(&"go".to_string())
                } else if path_str.ends_with(".java") {
                    enabled_languages.contains(&"java".to_string())
                } else if path_str.ends_with(".intent") {
                    enabled_languages.contains(&"intent".to_string())
                } else {
                    false
                };

                if is_enabled {
                    if path_str.ends_with(".py") {
                        python_files.push(path_str);
                    } else if path_str.ends_with(".rs") {
                        rust_files.push(path_str);
                    } else if path_str.ends_with(".go") {
                        go_files.push(path_str);
                    } else if path_str.ends_with(".java") {
                        java_files.push(path_str);
                    } else if path_str.ends_with(".intent") {
                        intent_files.push(path_str);
                    }
                } else {
                    other_files.push(path_str);
                }
            } else {
                other_files.push(path_str);
            }
        }
    }

    println!("Found {} Python files", python_files.len());
    for file in &python_files {
        println!("  - {}", file);
    }

    println!("Found {} Rust files", rust_files.len());
    for file in &rust_files {
        println!("  - {}", file);
    }

    println!("Found {} Go files", go_files.len());
    for file in &go_files {
        println!("  - {}", file);
    }

    println!("Found {} Java files", java_files.len());
    for file in &java_files {
        println!("  - {}", file);
    }

    println!("Found {} Intent files", intent_files.len());
    for file in &intent_files {
        println!("  - {}", file);
    }

    if !other_files.is_empty() {
        println!("Found {} other files (not indexed)", other_files.len());
    }

    // Phase 1: Index all supported files to collect symbols
    println!("\nPhase 1: Indexing {} Python files, {} Rust files, {} Go files, {} Java files, and {} Intent files...", python_files.len(), rust_files.len(), go_files.len(), java_files.len(), intent_files.len());
    let mut all_files = Vec::new();
    all_files.extend(python_files.clone());
    all_files.extend(rust_files.clone());
    all_files.extend(go_files.clone());
    all_files.extend(java_files.clone());
    all_files.extend(intent_files.clone());

    let mut all_symbols = Vec::new();
    for file_path in &all_files {
        println!("Indexing: {}", file_path);
        let content = std::fs::read_to_string(file_path)?;
        let (symbols, _) = indexer.index_file(file_path, &content).await?;
        all_symbols.extend(symbols);
        println!("  → {} total symbols", all_symbols.len());
    }

    // Phase 2: Extract relationships with global context
    println!("\nPhase 2: Extracting relationships...");
    let mut total_relationships = 0;
    for file_path in &all_files {
        println!("Extracting relationships: {}", file_path);
        let content = std::fs::read_to_string(file_path)?;
        let relationships = indexer.extract_relationships(file_path, &content, &all_symbols).await?;
        total_relationships += relationships.len();
        println!("  → {} relationships", relationships.len());
    }

    // Show stats
    let stats = indexer.get_stats()?;
    println!("\nIndexing complete!");
    println!("Total symbols: {}", stats.total_symbols);
    println!("Total files: {}", stats.total_files);
    println!("Total relationships: {}", total_relationships);

    // Start file watcher if requested
    if should_watch {
        println!("\n👀 Starting file watcher...");
        println!("Monitoring for file changes. Press Ctrl+C to stop.");

        // Start the watcher (this will block)
        crate::indexer::watcher::start_watcher(&project, should_watch).await?;
    } else {
        println!("\n✅ Initial indexing complete!");
        println!("Run with --watch to monitor for changes.");
    }

    Ok(())
}
