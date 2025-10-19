// Configuration management for CodeGraph

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    pub languages: LanguagesConfig,
    pub indexing: IndexingConfig,
    pub query: QueryConfig,
    pub performance: PerformanceConfig,
    pub logging: LoggingConfig,
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguagesConfig {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub watch: bool,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    pub timeout: u64,
    pub max_depth: usize,
    pub cache_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub threads: usize,
    pub memory_limit: usize,
    pub profile_queries: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub transport: String,
    pub port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: "unnamed-project".to_string(),
                root: ".".to_string(),
            },
            languages: LanguagesConfig {
                enabled: vec![
                    "python".to_string(),
                    "rust".to_string(),
                    "go".to_string(),
                    "java".to_string(),
                    "intent".to_string(),
                ],
            },
            indexing: IndexingConfig {
                exclude: vec![
                    "target/".to_string(),
                    "node_modules/".to_string(),
                    "*.test.*".to_string(),
                    "**/__tests__/**".to_string(),
                    ".git/".to_string(),
                    ".codegraph.db".to_string(),
                ],
                include: vec![],
                watch: false,
                batch_size: 100,
            },
            query: QueryConfig {
                timeout: 5000,
                max_depth: 10,
                cache_size: 1000,
            },
            performance: PerformanceConfig {
                threads: 4,
                memory_limit: 500,
                profile_queries: false,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
            },
            mcp: McpConfig {
                transport: "stdio".to_string(),
                port: 3000,
            },
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from project directory
    /// Looks for .codegraph.toml in the project root
    pub fn from_project_dir<P: AsRef<Path>>(project_dir: P) -> Self {
        let config_path = project_dir.as_ref().join(".codegraph.toml");

        match Self::from_file(&config_path) {
            Ok(config) => {
                tracing::info!("Loaded configuration from {}", config_path.display());
                config
            }
            Err(e) => {
                tracing::debug!("Could not load config from {}: {}", config_path.display(), e);
                tracing::info!("Using default configuration");
                Self::default()
            }
        }
    }

    /// Check if a file path should be indexed based on include/exclude patterns
    pub fn should_index_file(&self, file_path: &str) -> bool {
        let path = Path::new(file_path);

        // Check exclude patterns first
        for pattern in &self.indexing.exclude {
            if self.matches_pattern(file_path, pattern) {
                return false;
            }
        }

        // If include patterns are specified, file must match at least one
        if !self.indexing.include.is_empty() {
            for pattern in &self.indexing.include {
                if self.matches_pattern(file_path, pattern) {
                    return true;
                }
            }
            return false; // Include patterns specified but none matched
        }

        // No include patterns, and not excluded, so index it
        true
    }

    /// Simple pattern matching (supports glob-style patterns)
    fn matches_pattern(&self, file_path: &str, pattern: &str) -> bool {
        // Simple implementation - could be enhanced with proper glob matching
        if pattern.ends_with('/') {
            // Directory pattern
            file_path.starts_with(pattern) || file_path.contains(&format!("/{}", pattern.trim_end_matches('/')))
        } else if pattern.starts_with("*.") {
            // File pattern like *.test.*
            let pattern_part = &pattern[2..]; // Remove *.
            file_path.contains(pattern_part)
        } else if pattern.contains("**") {
            // Recursive pattern - simplified for **/__tests__/**
            if pattern == "**/__tests__/**" {
                file_path.contains("/__tests__/")
            } else {
                false
            }
        } else {
            // Exact match or prefix
            file_path.contains(pattern)
        }
    }

    /// Get enabled languages, filtered by what's actually supported
    pub fn get_enabled_languages(&self) -> Vec<String> {
        let supported = vec![
            "python", "rust", "go", "java", "intent"
        ];

        self.languages.enabled.iter()
            .filter(|lang| supported.contains(&lang.as_str()))
            .cloned()
            .collect()
    }

    /// Validate configuration values
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate project settings
        if self.project.name.is_empty() {
            return Err(anyhow::anyhow!("Project name cannot be empty"));
        }

        // Validate languages
        let supported_languages = ["python", "rust", "go", "java", "intent"];
        for lang in &self.languages.enabled {
            if !supported_languages.contains(&lang.as_str()) {
                return Err(anyhow::anyhow!("Unsupported language: {}", lang));
            }
        }

        // Validate indexing settings
        if self.indexing.batch_size == 0 {
            return Err(anyhow::anyhow!("Batch size must be greater than 0"));
        }

        // Validate query settings
        if self.query.timeout == 0 {
            return Err(anyhow::anyhow!("Query timeout must be greater than 0"));
        }
        if self.query.max_depth == 0 {
            return Err(anyhow::anyhow!("Query max depth must be greater than 0"));
        }
        if self.query.cache_size == 0 {
            return Err(anyhow::anyhow!("Query cache size must be greater than 0"));
        }

        // Validate performance settings
        if self.performance.threads == 0 {
            return Err(anyhow::anyhow!("Thread count must be greater than 0"));
        }
        if self.performance.memory_limit == 0 {
            return Err(anyhow::anyhow!("Memory limit must be greater than 0"));
        }

        // Validate logging
        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(anyhow::anyhow!("Invalid log level: {}", self.logging.level));
        }
        let valid_formats = ["compact", "pretty", "json"];
        if !valid_formats.contains(&self.logging.format.as_str()) {
            return Err(anyhow::anyhow!("Invalid log format: {}", self.logging.format));
        }

        // Validate MCP settings
        let valid_transports = ["stdio", "http"];
        if !valid_transports.contains(&self.mcp.transport.as_str()) {
            return Err(anyhow::anyhow!("Invalid MCP transport: {}", self.mcp.transport));
        }
        if self.mcp.port == 0 {
            return Err(anyhow::anyhow!("MCP port must be greater than 0"));
        }

        Ok(())
    }
}

/// Load configuration for a project
pub fn load_config(project_dir: &str) -> Config {
    Config::from_project_dir(project_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.project.name, "unnamed-project");
        assert!(config.languages.enabled.contains(&"python".to_string()));
        assert!(config.indexing.exclude.contains(&"target/".to_string()));
    }

    #[test]
    fn test_should_index_file() {
        let config = Config::default();

        // Should index normal files
        assert!(config.should_index_file("src/main.rs"));
        assert!(config.should_index_file("lib/utils.py"));

        // Should exclude specified patterns
        assert!(!config.should_index_file("target/debug/binary"));
        assert!(!config.should_index_file("node_modules/package/file.js"));
        assert!(!config.should_index_file("src/__tests__/test.py"));
        assert!(!config.should_index_file(".codegraph.db"));
    }

    #[test]
    fn test_pattern_matching() {
        let config = Config::default();

        // Directory patterns
        assert!(config.matches_pattern("target/debug/file", "target/"));
        assert!(config.matches_pattern("src/target/file", "target/"));

        // Extension patterns
        assert!(config.matches_pattern("test.py", "*.py"));
        assert!(!config.matches_pattern("test.rs", "*.py"));

        // Recursive patterns
        assert!(config.matches_pattern("src/__tests__/test.py", "**/__tests__/**"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Test invalid project name
        config.project.name = "".to_string();
        assert!(config.validate().is_err());
        config.project.name = "test".to_string();

        // Test invalid language
        config.languages.enabled = vec!["invalid_lang".to_string()];
        assert!(config.validate().is_err());
        config.languages.enabled = vec!["python".to_string()];

        // Test invalid batch size
        config.indexing.batch_size = 0;
        assert!(config.validate().is_err());
        config.indexing.batch_size = 100;

        // Test invalid log level
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
        config.logging.level = "info".to_string();

        // Test invalid MCP transport
        config.mcp.transport = "invalid".to_string();
        assert!(config.validate().is_err());
        config.mcp.transport = "stdio".to_string();
    }
}