// File watcher for incremental updates

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use tracing::{debug, error, info, warn};

use crate::indexer::Indexer;

/// File watcher for automatic re-indexing
pub struct FileWatcher {
    indexer: Arc<Indexer>,
    watch_path: PathBuf,
    extensions: HashSet<String>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(indexer: Arc<Indexer>, watch_path: PathBuf) -> Self {
        let mut extensions = HashSet::new();
        extensions.insert("py".to_string());
        extensions.insert("rs".to_string());
        extensions.insert("go".to_string());
        extensions.insert("java".to_string());
        extensions.insert("intent".to_string());

        Self {
            indexer,
            watch_path,
            extensions,
        }
    }

    /// Start watching for file changes
    pub async fn watch(self) -> Result<()> {
        info!("Starting file watcher for: {}", self.watch_path.display());

        // Create a standard sync channel for file events (notify runs in its own thread)
        let (tx, rx) = std::sync::mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));

        // Create the file watcher with a sync callback
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if let Err(e) = tx.send(event) {
                            error!("Failed to send file event: {}", e);
                        }
                    }
                    Err(e) => error!("File watch error: {}", e),
                }
            },
            Config::default(),
        )?;

        // Start watching the directory recursively
        watcher.watch(&self.watch_path, RecursiveMode::Recursive)?;

        info!("File watcher started. Monitoring for changes...");

        // Process file events in async context
        // Keep watcher alive by moving it into the loop
        loop {
            // Use blocking recv in spawn_blocking to avoid blocking the async runtime
            let event = match tokio::task::spawn_blocking({
                let rx = Arc::clone(&rx);
                move || rx.lock().recv()
            })
            .await
            {
                Ok(Ok(event)) => event,
                Ok(Err(_)) => break, // Channel closed
                Err(e) => {
                    error!("Spawn blocking error: {}", e);
                    break;
                }
            };

            if let Err(e) = self.handle_event(event).await {
                error!("Error handling event: {}", e);
            }
        }

        // Keep watcher alive until loop exits
        drop(watcher);

        Ok(())
    }

    /// Handle a file system event
    async fn handle_event(&self, event: Event) -> Result<()> {
        debug!("File event: {:?}", event);

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Check if any of the changed paths are files we care about
                for path in &event.paths {
                    if self.should_index_file(path) {
                        self.handle_file_change(path, &event.kind).await?;
                    }
                }
            }
            _ => {
                // Ignore other event types
            }
        }

        Ok(())
    }

    /// Handle a file change event
    async fn handle_file_change(&self, path: &Path, kind: &EventKind) -> Result<()> {
        let path_str = path.to_string_lossy();

        match kind {
            EventKind::Create(_) => {
                info!("File created: {}", path_str);
                self.index_file(&path_str).await?;
            }
            EventKind::Modify(_) => {
                info!("File modified: {}", path_str);
                self.index_file(&path_str).await?;
            }
            EventKind::Remove(_) => {
                info!("File removed: {}", path_str);
                self.remove_file(&path_str).await?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if a file should be indexed
    fn should_index_file(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                return self.extensions.contains(ext_str);
            }
        }

        false
    }

    /// Index a single file
    async fn index_file(&self, file_path: &str) -> Result<()> {
        debug!("Indexing file: {}", file_path);

        // Read the file content
        let content = match std::fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to read file {}: {}", file_path, e);
                return Ok(()); // Don't fail the watcher for read errors
            }
        };

        // Index the file
        match self.indexer.index_file(file_path, &content).await {
            Ok((symbols, _relationships)) => {
                info!("Indexed {}: {} symbols", file_path, symbols.len());
                Ok(())
            }
            Err(e) => {
                error!("Failed to index {}: {}", file_path, e);
                Ok(()) // Don't fail the watcher for indexing errors
            }
        }
    }

    /// Remove a file from the index
    async fn remove_file(&self, file_path: &str) -> Result<()> {
        debug!("Removing file from index: {}", file_path);

        // For now, we'll just log this - full removal would require
        // deleting symbols and relationships from the database
        // TODO: Implement proper file removal
        info!("File removal not yet implemented: {}", file_path);

        Ok(())
    }
}

/// Start the file watcher for a project
pub async fn start_watcher(project_path: &str, watch: bool) -> Result<()> {
    if !watch {
        return Ok(());
    }

    info!("Initializing file watcher for project: {}", project_path);

    // Initialize indexer
    let db_path = PathBuf::from(project_path).join(".codegraph.db");
    let indexer = Arc::new(Indexer::new(&db_path)?);

    // Create and start watcher
    let watcher = FileWatcher::new(indexer, PathBuf::from(project_path));

    // Run the watcher (this will block)
    watcher.watch().await?;

    Ok(())
}
