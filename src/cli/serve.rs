use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

use crate::config::Config;
use crate::indexer::Indexer;
use crate::mcp::server::McpServer;

/// Start MCP server with stdio transport
pub async fn serve_stdio(project: String) -> Result<()> {
    // Load configuration
    let config = Config::from_project_dir(&project);

    info!("MCP server (stdio) for project: {}", project);
    println!("Config: {}", if config.project.name != "unnamed-project" { "loaded" } else { "default" });

    // Initialize indexer
    let db_path = PathBuf::from(&project).join(".codegraph.db");
    let indexer = Indexer::new(&db_path)?;

    // Check if index exists
    let stats = indexer.get_stats()?;
    if stats.total_symbols == 0 {
        eprintln!("Warning: No symbols indexed. Run 'codegraph index --project {}' first.", project);
    }

    // Start MCP server
    let server = McpServer::new(indexer);
    server.run().await?;

    Ok(())
}

/// Start MCP server with HTTP transport
pub async fn serve_http(project: String, port: u16) -> Result<()> {
    // Load configuration
    let config = Config::from_project_dir(&project);

    info!("MCP server (HTTP) for project: {} on port {}", project, port);
    println!("CodeGraph MCP Server v0.1.0");
    println!("Project: {}", project);
    println!("Transport: HTTP on port {}", port);
    println!("Config: {}", if config.project.name != "unnamed-project" { "loaded" } else { "default" });
    println!("\nHTTP transport not yet implemented - use stdio transport instead");
    println!("Run: codegraph serve --project {}", project);
    Ok(())
}
