// MCP server implementation

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::indexer::Indexer;
use crate::mcp::tools;

/// JSON-RPC message
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcMessage {
    jsonrpc: String,
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// MCP tool definition
#[derive(Debug, Serialize, Deserialize)]
struct Tool {
    name: String,
    description: String,
    input_schema: Value,
}

/// MCP server capabilities
#[derive(Debug, Serialize, Deserialize)]
struct ServerCapabilities {
    tools: Option<Value>,
}

/// MCP server info
#[derive(Debug, Serialize, Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
}

/// MCP initialize result
#[derive(Debug, Serialize, Deserialize)]
struct InitializeResult {
    protocol_version: String,
    capabilities: ServerCapabilities,
    server_info: ServerInfo,
}

/// MCP server
pub struct McpServer {
    indexer: Indexer,
}

impl McpServer {
    pub fn new(indexer: Indexer) -> Self {
        Self { indexer }
    }

    /// Run the MCP server
    pub async fn run(self) -> Result<()> {
        info!("Starting MCP server");

        let (tx, mut rx) = mpsc::unbounded_channel();

        // Spawn a task to handle stdin
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let stdin = io::stdin();
            let mut lines = stdin.lines();

            while let Some(line) = lines.next() {
                match line {
                    Ok(line) => {
                        if let Err(e) = tx_clone.send(line) {
                            error!("Failed to send line to channel: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from stdin: {}", e);
                        break;
                    }
                }
            }
        });

        // Main message processing loop
        while let Some(line) = rx.recv().await {
            debug!("Received: {}", line);

            match self.handle_message(&line).await {
                Ok(response) => {
                    if let Some(response) = response {
                        println!("{}", response);
                        io::stdout().flush()?;
                    }
                }
                Err(e) => {
                    error!("Error handling message: {}", e);
                    // Send error response
                    let error_response = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": {
                            "code": -32603,
                            "message": format!("Internal error: {}", e)
                        }
                    });
                    println!("{}", error_response);
                    io::stdout().flush()?;
                }
            }
        }

        Ok(())
    }

    /// Handle a JSON-RPC message
    async fn handle_message(&self, message: &str) -> Result<Option<String>> {
        let msg: JsonRpcMessage = serde_json::from_str(message)?;

        match msg.method.as_deref() {
            Some("initialize") => {
                let result = InitializeResult {
                    protocol_version: "2024-11-05".to_string(),
                    capabilities: ServerCapabilities {
                        tools: Some(json!({})),
                    },
                    server_info: ServerInfo {
                        name: "codegraph".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                };

                let response = json!({
                    "jsonrpc": "2.0",
                    "id": msg.id,
                    "result": result
                });

                Ok(Some(serde_json::to_string(&response)?))
            }

            Some("tools/list") => {
                let tools = self.list_tools();
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": msg.id,
                    "result": { "tools": tools }
                });

                Ok(Some(serde_json::to_string(&response)?))
            }

            Some("tools/call") => {
                if let Some(params) = &msg.params {
                    let result = self.call_tool(params).await?;
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": msg.id,
                        "result": result
                    });

                    Ok(Some(serde_json::to_string(&response)?))
                } else {
                    let error = json!({
                        "jsonrpc": "2.0",
                        "id": msg.id,
                        "error": {
                            "code": -32602,
                            "message": "Invalid params"
                        }
                    });
                    Ok(Some(serde_json::to_string(&error)?))
                }
            }

            Some("shutdown") => {
                info!("Received shutdown request");
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": msg.id,
                    "result": null
                });
                Ok(Some(serde_json::to_string(&response)?))
            }

            _ => {
                let error = json!({
                    "jsonrpc": "2.0",
                    "id": msg.id,
                    "error": {
                        "code": -32601,
                        "message": "Method not found"
                    }
                });
                Ok(Some(serde_json::to_string(&error)?))
            }
        }
    }

    /// List available tools
    fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "codegraph_query".to_string(),
                description: "Query the code index for relationships and references".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query_type": {
                            "type": "string",
                            "enum": ["callers", "callees", "references", "dependencies"],
                            "description": "Type of query to perform"
                        },
                        "target": {
                            "type": "string",
                            "description": "Target symbol to query"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["text", "json"],
                            "default": "text",
                            "description": "Output format"
                        }
                    },
                    "required": ["query_type", "target"]
                }),
            },
            Tool {
                name: "codegraph_search".to_string(),
                description: "Search for symbols by name or content".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "kind": {
                            "type": "string",
                            "enum": ["function", "class", "variable", "method", "field"],
                            "description": "Filter by symbol kind"
                        },
                        "limit": {
                            "type": "integer",
                            "default": 10,
                            "description": "Maximum number of results"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "codegraph_stats".to_string(),
                description: "Get index statistics".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    /// Call a tool
    async fn call_tool(&self, params: &Value) -> Result<Value> {
        let tool_name = params["name"].as_str().ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
        let tool_args = params["arguments"].as_object().ok_or_else(|| anyhow::anyhow!("Invalid arguments"))?;

        // Convert serde_json::Map to HashMap
        let args_hashmap: std::collections::HashMap<String, Value> = tool_args.clone().into_iter().collect();

        match tool_name {
            "codegraph_query" => tools::query(&self.indexer, &args_hashmap).await,
            "codegraph_search" => tools::search(&self.indexer, &args_hashmap).await,
            "codegraph_stats" => tools::stats(&self.indexer, &args_hashmap).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        }
    }
}
