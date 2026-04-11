use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct StreamDelta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallDelta>>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ToolCallDelta {
    pub index: Option<u32>,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub call_type: Option<String>,
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct AccumulatedToolCalls {
    calls: HashMap<u32, AccumulatedToolCall>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AccumulatedToolCall {
    id: String,
    call_type: String,
    name: String,
    arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

impl AccumulatedToolCalls {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_delta(&mut self, deltas: Vec<ToolCallDelta>) {
        for delta in deltas {
            let index = delta.index.unwrap_or(0);
            let entry = self
                .calls
                .entry(index)
                .or_insert_with(|| AccumulatedToolCall {
                    id: String::new(),
                    call_type: String::new(),
                    name: String::new(),
                    arguments: String::new(),
                });

            if let Some(id) = delta.id {
                entry.id = id;
            }
            if let Some(t) = delta.call_type {
                entry.call_type = t;
            }
            if let Some(func) = delta.function {
                if let Some(name) = func.name {
                    entry.name = name;
                }
                if let Some(args) = func.arguments {
                    entry.arguments.push_str(&args);
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn into_tool_calls(self) -> Vec<ToolCall> {
        let mut calls: Vec<ToolCall> = self
            .calls
            .into_values()
            .map(|acc| ToolCall {
                id: acc.id,
                call_type: acc.call_type,
                function: ToolCallFunction {
                    name: acc.name,
                    arguments: acc.arguments,
                },
            })
            .collect();
        calls.sort_by_key(|c| {
            c.id.strip_prefix("call_")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0)
        });
        calls
    }
}

#[allow(dead_code)]
pub fn parse_sse_line(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if line == "data: [DONE]" {
        return None;
    }
    line.strip_prefix("data: ").map(|data| data.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulate_tool_calls() {
        let mut acc = AccumulatedToolCalls::new();
        acc.apply_delta(vec![ToolCallDelta {
            index: Some(0),
            id: Some("call_123".to_string()),
            call_type: Some("function".to_string()),
            function: Some(FunctionDelta {
                name: Some("shell".to_string()),
                arguments: None,
            }),
        }]);
        acc.apply_delta(vec![ToolCallDelta {
            index: Some(0),
            id: None,
            call_type: None,
            function: Some(FunctionDelta {
                name: None,
                arguments: Some("{\"comma".to_string()),
            }),
        }]);
        acc.apply_delta(vec![ToolCallDelta {
            index: Some(0),
            id: None,
            call_type: None,
            function: Some(FunctionDelta {
                name: None,
                arguments: Some("nd\":\"ls\"}".to_string()),
            }),
        }]);

        let calls = acc.into_tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_123");
        assert_eq!(calls[0].function.name, "shell");
        assert_eq!(calls[0].function.arguments, "{\"command\":\"ls\"}");
    }

    #[test]
    fn test_parse_sse_line() {
        assert_eq!(
            parse_sse_line("data: {\"hello\":true}"),
            Some("{\"hello\":true}".to_string())
        );
        assert_eq!(parse_sse_line("data: [DONE]"), None);
        assert_eq!(parse_sse_line(""), None);
        assert_eq!(parse_sse_line("event: ping"), None);
    }
}
