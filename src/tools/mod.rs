use serde_json::Value;
use std::collections::HashMap;

pub mod confirm;
pub mod native;
pub mod server;

pub const MAX_TOOL_ITERATIONS: u32 = 25;

#[derive(Debug, Clone)]
pub enum ToolResult {
    Success(String),
    Error(String),
}

impl ToolResult {
    pub fn to_content(&self) -> String {
        match self {
            ToolResult::Success(msg) => msg.clone(),
            ToolResult::Error(msg) => format!("Error: {}", msg),
        }
    }

    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        matches!(self, ToolResult::Error(_))
    }
}

#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub auto_approve: bool,
    pub shell_timeout: u64,
    pub mode: ConfirmMode,
}

#[derive(Debug, Clone, Copy)]
pub enum ConfirmMode {
    Prompt,
    Chat,
    Tui,
}

pub fn get_native_tool_definitions() -> Vec<Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "read",
                "description": "Read the contents of a file from the local filesystem. Returns the file content as a string. Use this to examine source code, configuration files, or any text file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute or relative path to the file to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "write",
                "description": "Write content to a file on the local filesystem. Creates parent directories if needed. This will overwrite existing files. Use with caution.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute or relative path to the file to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "shell",
                "description": "Execute a shell command and return its stdout and stderr. Use for running build commands, tests, git operations, or any CLI tool. Commands run in the user's working directory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        }
                    },
                    "required": ["command"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "web_fetch",
                "description": "Fetch content from a URL. Returns the response body as text. Useful for retrieving web pages, API responses, or documentation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to fetch content from"
                        }
                    },
                    "required": ["url"]
                }
            }
        }),
    ]
}

pub fn build_tools_array(
    include_native: bool,
    include_web_search: bool,
    include_datetime: bool,
    search_engine: &str,
    max_search_results: u32,
) -> Vec<Value> {
    let mut tools = Vec::new();

    if include_native {
        tools.extend(get_native_tool_definitions());
    }

    if include_web_search {
        let mut web_search = serde_json::json!({
            "type": "openrouter:web_search",
        });

        if search_engine != "auto" || max_search_results != 5 {
            let mut params = serde_json::Map::new();
            if search_engine != "auto" {
                params.insert("engine".to_string(), Value::String(search_engine.to_string()));
            }
            if max_search_results != 5 {
                params.insert("max_results".to_string(), Value::Number(max_search_results.into()));
            }
            web_search["parameters"] = Value::Object(params);
        }

        tools.push(web_search);
    }

    if include_datetime {
        tools.push(serde_json::json!({
            "type": "openrouter:datetime"
        }));
    }

    tools
}

pub async fn execute_native_tool(
    name: &str,
    arguments: &str,
    config: &ToolConfig,
) -> ToolResult {
    let args: HashMap<String, Value> = match serde_json::from_str(arguments) {
        Ok(a) => a,
        Err(e) => return ToolResult::Error(format!("Invalid JSON arguments: {}", e)),
    };

    let needs_confirm = matches!(name, "write" | "shell");

    if needs_confirm && !config.auto_approve {
        let confirmed = match config.mode {
            ConfirmMode::Prompt => confirm::confirm_prompt(name, arguments),
            ConfirmMode::Chat => confirm::confirm_chat(name, arguments),
            ConfirmMode::Tui => confirm::confirm_tui(name, arguments),
        };

        if !confirmed {
            return ToolResult::Error("User denied this tool call.".to_string());
        }
    }

    match name {
        "read" => native::tool_read(&args).await,
        "write" => native::tool_write(&args).await,
        "shell" => native::tool_shell(&args, config.shell_timeout).await,
        "web_fetch" => native::tool_web_fetch(&args).await,
        _ => ToolResult::Error(format!("Unknown tool: {}", name)),
    }
}