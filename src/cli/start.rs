use anyhow::Result;
use std::path::PathBuf;
use tracing::info;
use tokio::task;
use walkdir::WalkDir;

use crate::config::Config;
use crate::indexer::Indexer;
use crate::mcp::server::McpServer;

/// Start MCP server with auto-indexing and optional watch mode
pub async fn start_server(
    project: String,
    port: Option<u16>,
    watch: bool,
    rebuild: bool,
) -> Result<()> {
    info!("Starting CodeGraph for project: {}", project);

    // Load configuration
    let config = Config::from_project_dir(&project);

    println!("CodeGraph MCP Server v0.1.0");
    println!("Project: {}", project);
    println!("Config: {}", if config.project.name != "unnamed-project" { "loaded" } else { "default" });
    println!("Watch mode: {}", if watch { "enabled" } else { "disabled" });

    // Initialize database
    let db_path = PathBuf::from(&project).join(".codegraph.db");
    let db_exists = db_path.exists();

    // Determine if we need to index
    let should_index = rebuild || !db_exists || {
        // Check if index is empty
        if db_exists {
            let indexer = Indexer::new(&db_path)?;
            let stats = indexer.get_stats()?;
            stats.total_symbols == 0
        } else {
            true
        }
    };

    if should_index {
        println!("\n📊 Indexing project...");

        // Get enabled languages from config
        let enabled_languages = config.get_enabled_languages();
        println!("Languages: {}", enabled_languages.join(", "));

        // Scan and index files
        let indexer = Indexer::new(&db_path)?;
        let mut python_files = Vec::new();
        let mut rust_files = Vec::new();
        let mut go_files = Vec::new();
        let mut java_files = Vec::new();
        let mut intent_files = Vec::new();

        println!("Scanning files...");
        for entry in WalkDir::new(&project).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                let path_str = path.to_string_lossy().to_string();

                if config.should_index_file(&path_str) && indexer.can_index_file(&path_str) {
                    if path_str.ends_with(".py") && enabled_languages.contains(&"python".to_string()) {
                        python_files.push(path_str);
                    } else if path_str.ends_with(".rs") && enabled_languages.contains(&"rust".to_string()) {
                        rust_files.push(path_str);
                    } else if path_str.ends_with(".go") && enabled_languages.contains(&"go".to_string()) {
                        go_files.push(path_str);
                    } else if path_str.ends_with(".java") && enabled_languages.contains(&"java".to_string()) {
                        java_files.push(path_str);
                    } else if path_str.ends_with(".intent") && enabled_languages.contains(&"intent".to_string()) {
                        intent_files.push(path_str);
                    }
                }
            }
        }

        let total_files = python_files.len() + rust_files.len() + go_files.len() + java_files.len() + intent_files.len();
        println!("Found {} files to index", total_files);

        // Collect all files
        let mut all_files = Vec::new();
        all_files.extend(python_files);
        all_files.extend(rust_files);
        all_files.extend(go_files);
        all_files.extend(java_files);
        all_files.extend(intent_files);

        // Phase 1: Index files
        let mut all_symbols = Vec::new();
        for (i, file_path) in all_files.iter().enumerate() {
            if i % 10 == 0 || i == all_files.len() - 1 {
                print!("\rIndexing: {}/{} files", i + 1, all_files.len());
                use std::io::Write;
                std::io::stdout().flush()?;
            }
            let content = std::fs::read_to_string(file_path)?;
            let (symbols, _) = indexer.index_file(file_path, &content).await?;
            all_symbols.extend(symbols);
        }
        println!("\rIndexed {} files, {} symbols", all_files.len(), all_symbols.len());

        // Phase 2: Extract relationships
        print!("Extracting relationships...");
        use std::io::Write;
        std::io::stdout().flush()?;
        for file_path in &all_files {
            let content = std::fs::read_to_string(file_path)?;
            indexer.extract_relationships(file_path, &content, &all_symbols).await?;
        }
        println!(" done!");

        let stats = indexer.get_stats()?;
        println!("✅ Index ready: {} symbols, {} files", stats.total_symbols, stats.total_files);
    } else {
        let indexer = Indexer::new(&db_path)?;
        let stats = indexer.get_stats()?;
        println!("✅ Using existing index: {} symbols, {} files", stats.total_symbols, stats.total_files);
    }

    // Start MCP server
    println!("\n🚀 Starting MCP server...");

    if watch {
        // Start file watcher in background
        let project_clone = project.clone();
        let _watcher_handle = task::spawn(async move {
            if let Err(e) = crate::indexer::watcher::start_watcher(&project_clone, true).await {
                eprintln!("File watcher error: {}", e);
            }
        });

        println!("👀 File watching enabled");
    }

    // Start MCP server based on transport
    let indexer = Indexer::new(&db_path)?;

    if let Some(port) = port {
        println!("Transport: HTTP on port {}", port);
        println!("\nHTTP transport not yet implemented - use stdio transport instead");
        println!("Run: codegraph {}", project);
    } else {
        println!("Transport: stdio");
        println!("\n✅ CodeGraph is ready! Listening for MCP requests...\n");

        let server = McpServer::new(indexer);
        server.run().await?;
    }

    Ok(())
}
