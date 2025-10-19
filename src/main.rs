use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber;

mod cli;
mod config;
mod index;
mod indexer;
mod mcp;
mod query;

#[derive(Parser)]
#[command(name = "codegraph")]
#[command(author = "Intent Project Team")]
#[command(version = "0.1.0")]
#[command(about = "Real-time semantic code index for AI agents via MCP", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Project directory (shorthand for 'codegraph start <project>')
    #[arg(value_name = "PROJECT")]
    project: Option<String>,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start MCP server (auto-index + watch) - default command
    Start {
        /// Project directory to index
        #[arg(default_value = ".")]
        project: String,

        /// Port for HTTP server (optional, uses stdio by default)
        #[arg(short = 'P', long)]
        port: Option<u16>,

        /// Disable file watching
        #[arg(long)]
        no_watch: bool,

        /// Force rebuild index
        #[arg(short, long)]
        rebuild: bool,
    },

    /// Start MCP server (manual mode - no auto-indexing)
    Serve {
        /// Project directory to index
        #[arg(short, long, default_value = ".")]
        project: String,

        /// Port for HTTP server (optional, uses stdio by default)
        #[arg(short = 'P', long)]
        port: Option<u16>,
    },

    /// Index a project
    Index {
        /// Project directory to index
        #[arg(short, long, default_value = ".")]
        project: String,

        /// Languages to index (comma-separated)
        #[arg(short, long)]
        languages: Option<String>,

        /// Watch for changes
        #[arg(short, long)]
        watch: bool,

        /// Rebuild entire index
        #[arg(short, long)]
        rebuild: bool,
    },

    /// Query the index
    Query {
        /// Query type: callers, callees, references, deps
        query_type: String,

        /// Target symbol
        target: String,

        /// Project directory
        #[arg(short, long, default_value = ".")]
        project: String,

        /// Output format: json, text
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Analyze impact of changes
    Impact {
        /// Change type: rename, delete, change_type
        change_type: String,

        /// Target symbol
        target: String,

        /// New value (for rename, change_type)
        #[arg(short, long)]
        to: Option<String>,

        /// Project directory
        #[arg(short, long, default_value = ".")]
        project: String,
    },

    /// Show index statistics
    Stats {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        project: String,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// List supported languages
    Languages,
}

fn init_logging(debug: bool, verbose: bool) {
    let level = if debug {
        Level::DEBUG
    } else if verbose {
        Level::INFO
    } else {
        Level::WARN
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    init_logging(cli.debug, cli.verbose);

    info!("CodeGraph v0.1.0 starting...");

    // Handle shorthand: codegraph <project>
    let command = if let Some(cmd) = cli.command {
        cmd
    } else if let Some(project) = cli.project {
        // No subcommand provided, use shorthand Start
        Commands::Start {
            project,
            port: None,
            no_watch: false,
            rebuild: false,
        }
    } else {
        // No subcommand and no project path - use current directory
        Commands::Start {
            project: ".".to_string(),
            port: None,
            no_watch: false,
            rebuild: false,
        }
    };

    match command {
        Commands::Start {
            project,
            port,
            no_watch,
            rebuild,
        } => {
            info!("Starting CodeGraph for project: {}", project);
            cli::start::start_server(project, port, !no_watch, rebuild).await?;
        }

        Commands::Serve { project, port } => {
            info!("Starting MCP server for project: {}", project);
            if let Some(port) = port {
                info!("HTTP server on port {}", port);
                cli::serve::serve_http(project, port).await?;
            } else {
                info!("Using stdio transport");
                cli::serve::serve_stdio(project).await?;
            }
        }

        Commands::Index {
            project,
            languages,
            watch,
            rebuild,
        } => {
            info!("Indexing project: {}", project);
            cli::index::index_project(project, languages, watch, rebuild).await?;
        }

        Commands::Query {
            query_type,
            target,
            project,
            format,
        } => {
            cli::query::query_index(query_type, target, project, format).await?;
        }

        Commands::Impact {
            change_type,
            target,
            to,
            project,
        } => {
            cli::impact::analyze_impact(change_type, target, to, project).await?;
        }

        Commands::Stats { project, verbose } => {
            cli::stats::show_stats(project, verbose).await?;
        }

        Commands::Languages => {
            cli::languages::list_languages();
        }
    }

    Ok(())
}
