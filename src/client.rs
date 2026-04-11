use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::cli::Cli;
use crate::stream::{AccumulatedToolCalls, ToolCall, parse_sse_line};
use crate::tools::{ToolCallInfo, ToolConfig, ToolResult, execute_native_tool};
use crate::attachment::Attachment;

const API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum StreamEvent {
    Delta(String),
    ToolCalls(Vec<ToolCallInfo>),
    Done(String),
    Error(String),
}

#[derive(Clone)]
pub struct OpenRouterClient {
    client: Client,
    api_key: String,
    model: String,
    tools: Vec<serde_json::Value>,
    tool_config: ToolConfig,
}

impl OpenRouterClient {
    pub fn new(cli: &Cli) -> Result<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .context("OPENROUTER_API_KEY environment variable is required")?;

        let include_native = !cli.no_native_tools;
        let include_web_search = !cli.no_web_search;
        let include_datetime = !cli.no_datetime;

        let tools = crate::tools::build_tools_array(
            include_native,
            include_web_search,
            include_datetime,
            &cli.search_engine,
            cli.max_search_results,
        );

        let tool_config = ToolConfig {
            auto_approve: cli.yes,
            shell_timeout: cli.shell_timeout,
            mode: match &cli.command {
                crate::cli::Commands::Prompt { .. } => crate::tools::ConfirmMode::Prompt,
                crate::cli::Commands::Chat => crate::tools::ConfirmMode::Chat,
                crate::cli::Commands::Tui => crate::tools::ConfirmMode::Tui,
            },
        };

        Ok(Self {
            client: Client::new(),
            api_key,
            model: cli.model.clone(),
            tools,
            tool_config,
        })
    }

    pub async fn send_with_agentic_loop(
        &self,
        messages: &mut Vec<Message>,
        attachments: &[Attachment],
    ) -> Result<String> {
        let mut iteration = 0u32;

        loop {
            if iteration >= crate::tools::MAX_TOOL_ITERATIONS {
                bail!("Max tool iterations (25) reached");
            }
            iteration += 1;

            let response = self.send_request(messages, attachments, false).await?;

            let assistant_content = response.content.clone().unwrap_or_default();
            if let Some(tool_calls) = response.tool_calls.clone() {
                messages.push(response);

                for tool_call in &tool_calls {
                    let info = ToolCallInfo {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: tool_call.function.arguments.clone(),
                    };

                    eprintln!("⚙ {}({})", info.name, truncate_display(&info.arguments, 80));

                    let result = execute_native_tool(&info.name, &info.arguments, &self.tool_config).await;

                    match &result {
                        ToolResult::Success(msg) => {
                            eprintln!("→ {}", truncate_display(msg, 80));
                        }
                        ToolResult::Error(msg) => {
                            eprintln!("✗ {}", msg);
                        }
                    }

                    messages.push(Message {
                        role: "tool".to_string(),
                        content: Some(serde_json::Value::String(result.to_content())),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }

                continue;
            }

            return Ok(assistant_content.to_string());
        }
    }

    #[allow(dead_code, clippy::type_complexity)]
    pub fn send_streaming_with_agentic_loop<'a>(
        &'a self,
        messages: &'a mut Vec<Message>,
        attachments: &'a [Attachment],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Pin<Box<dyn futures_util::Stream<Item = StreamEvent> + Send>>>> + Send + 'a>> {
        Box::pin(async move {
            let mut response = self.send_request(messages, attachments, true).await?;

            if response.tool_calls.is_some() {
                let tool_calls = response.tool_calls.take().unwrap();
                messages.push(response);

                for tool_call in &tool_calls {
                    let info = ToolCallInfo {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: tool_call.function.arguments.clone(),
                    };

                    eprintln!("⚙ {}({})", info.name, truncate_display(&info.arguments, 80));

                    let result = execute_native_tool(&info.name, &info.arguments, &self.tool_config).await;

                    match &result {
                        ToolResult::Success(msg) => {
                            eprintln!("→ {}", truncate_display(msg, 80));
                        }
                        ToolResult::Error(msg) => {
                            eprintln!("✗ {}", msg);
                        }
                    }

                    messages.push(Message {
                        role: "tool".to_string(),
                        content: Some(serde_json::Value::String(result.to_content())),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }

                return self.send_streaming_with_agentic_loop(messages, attachments).await;
            }

            let stream = self.stream_response().await?;
            Ok(stream)
        })
    }

    pub async fn send_request(
        &self,
        messages: &[Message],
        attachments: &[Attachment],
        stream: bool,
    ) -> Result<Message> {
        let mut messages_json = Vec::new();

        for msg in messages {
            let mut msg_json = serde_json::json!({
                "role": msg.role,
            });

            if let Some(content) = &msg.content {
                msg_json["content"] = content.clone();
            }

            if let Some(tool_calls) = &msg.tool_calls {
                msg_json["tool_calls"] = serde_json::to_value(tool_calls)?;
            }

            if let Some(tool_call_id) = &msg.tool_call_id {
                msg_json["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
            }

            messages_json.push(msg_json);
        }

        if !attachments.is_empty() && !messages_json.is_empty() {
            if let Some(content) = messages_json.last_mut().and_then(|m| m.get_mut("content")) {
                if let Some(existing_text) = content.as_str() {
                    let mut parts: Vec<serde_json::Value> = vec![serde_json::json!({
                        "type": "text",
                        "text": existing_text
                    })];
                    for att in attachments {
                        for part in &att.parts {
                            parts.push(part.to_openrouter());
                        }
                    }
                    *content = serde_json::Value::Array(parts);
                }
            }
        }

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages_json,
            "stream": stream,
        });

        if !self.tools.is_empty() {
            body["tools"] = serde_json::Value::Array(self.tools.clone());
        }

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/ergofobe/orai")
            .header("X-Title", "orai")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to OpenRouter")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("OpenRouter API error: {} - {}", status, body);
        }

        let response_json: serde_json::Value = response.json().await?;
        let choice = response_json
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let message = choice.get("message").ok_or_else(|| anyhow::anyhow!("No message in response"))?;

        let mut msg = Message {
            role: message.get("role").and_then(|r| r.as_str()).unwrap_or("assistant").to_string(),
            content: message.get("content").cloned(),
            tool_calls: None,
            tool_call_id: None,
        };

        if let Some(tool_calls) = message.get("tool_calls") {
            let calls: Vec<ToolCall> = serde_json::from_value(tool_calls.clone())?;
            msg.tool_calls = Some(calls);
        }

        Ok(msg)
    }

    #[allow(dead_code)]
    async fn stream_response(&self) -> Result<Pin<Box<dyn futures_util::Stream<Item = StreamEvent> + Send>>> {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": [],
            "stream": true,
        });

        if !self.tools.is_empty() {
            body["tools"] = serde_json::Value::Array(self.tools.clone());
        }

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/ergofobe/orai")
            .header("X-Title", "orai")
            .json(&body)
            .send()
            .await
            .context("Failed to connect to OpenRouter stream")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("OpenRouter API error: {} - {}", status, body);
        }

        let stream = response.bytes_stream();

        let mapped = stream.map(move |chunk| {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => return StreamEvent::Error(e.to_string()),
            };

            let text = String::from_utf8_lossy(&chunk);
            let mut acc = AccumulatedToolCalls::new();
            let mut content = String::new();
            let mut done = false;

            for line in text.lines() {
                if let Some(data) = parse_sse_line(line) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data) {
                        if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
                            if let Some(choice) = choices.first() {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(c) = delta.get("content").and_then(|v| v.as_str()) {
                                        content.push_str(c);
                                    }
                                    if let Some(tcs) = delta.get("tool_calls") {
                                        let deltas: Vec<crate::stream::ToolCallDelta> =
                                            serde_json::from_value(tcs.clone()).unwrap_or_default();
                                        acc.apply_delta(deltas);
                                    }
                                }
                                if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                                    if reason == "stop" || reason == "end_turn" || reason == "tool_calls" {
                                        done = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !acc.is_empty() {
                let calls = acc.into_tool_calls();
                let infos = calls
                    .into_iter()
                    .map(|c| ToolCallInfo {
                        id: c.id,
                        name: c.function.name,
                        arguments: c.function.arguments,
                    })
                    .collect();
                StreamEvent::ToolCalls(infos)
            } else if done {
                StreamEvent::Done(content)
            } else if !content.is_empty() {
                StreamEvent::Delta(content)
            } else {
                StreamEvent::Delta(String::new())
            }
        });

        Ok(Box::pin(mapped))
    }

    #[allow(dead_code)]
    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn tool_config(&self) -> &ToolConfig {
        &self.tool_config
    }
}

fn truncate_display(s: &str, max_len: usize) -> String {
    let single_line: String = s.lines().collect::<Vec<_>>().join(" ");
    if single_line.len() > max_len {
        format!("{}...", &single_line[..max_len])
    } else {
        single_line
    }
}