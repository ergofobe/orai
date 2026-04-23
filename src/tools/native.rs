use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

const MAX_READ_SIZE: usize = 10 * 1024 * 1024;
const MAX_SHELL_OUTPUT: usize = 50 * 1024;
const MAX_WEB_FETCH_SIZE: usize = 100 * 1024;

use super::ToolResult;

pub async fn tool_read(args: &HashMap<String, Value>) -> ToolResult {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return ToolResult::Error("Missing 'path' argument".to_string()),
    };

    let p = Path::new(path);

    if !p.exists() {
        return ToolResult::Error(format!("File not found: {}", path));
    }

    let metadata = match std::fs::metadata(p) {
        Ok(m) => m,
        Err(e) => return ToolResult::Error(format!("Cannot read file metadata: {}", e)),
    };

    if metadata.len() as usize > MAX_READ_SIZE {
        return ToolResult::Error(format!(
            "File too large ({} bytes, max {} bytes)",
            metadata.len(),
            MAX_READ_SIZE
        ));
    }

    match std::fs::read(p) {
        Ok(bytes) => match String::from_utf8(bytes.clone()) {
            Ok(text) => ToolResult::Success(text),
            Err(_) => {
                use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
                let b64 = BASE64.encode(&bytes);
                ToolResult::Success(format!(
                    "[Binary file: {} bytes, base64 encoded]\n{}",
                    bytes.len(),
                    b64
                ))
            }
        },
        Err(e) => ToolResult::Error(format!("Cannot read file: {}", e)),
    }
}

pub async fn tool_write(args: &HashMap<String, Value>) -> ToolResult {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return ToolResult::Error("Missing 'path' argument".to_string()),
    };

    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return ToolResult::Error("Missing 'content' argument".to_string()),
    };

    let p = Path::new(path);

    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::Error(format!("Cannot create directory: {}", e));
            }
        }
    }

    match std::fs::write(p, content) {
        Ok(()) => ToolResult::Success(format!(
            "Successfully wrote {} bytes to {}",
            content.len(),
            path
        )),
        Err(e) => ToolResult::Error(format!("Cannot write file: {}", e)),
    }
}

pub async fn tool_shell(args: &HashMap<String, Value>, timeout_secs: u64) -> ToolResult {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return ToolResult::Error("Missing 'command' argument".to_string()),
    };

    let timeout = Duration::from_secs(if timeout_secs > 0 { timeout_secs } else { 120 });
    let result = tokio::time::timeout(timeout, async {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
    })
    .await;

    match result {
        Ok(Ok(output)) => {
            let exit_code = output.status.code().unwrap_or(-1);
            let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if stdout.len() > MAX_SHELL_OUTPUT {
                stdout = format!(
                    "{}...\n(truncated, {} bytes total)",
                    &stdout[..MAX_SHELL_OUTPUT],
                    stdout.len()
                );
            }
            if stderr.len() > MAX_SHELL_OUTPUT {
                stderr = format!(
                    "{}...\n(truncated, {} bytes total)",
                    &stderr[..MAX_SHELL_OUTPUT],
                    stderr.len()
                );
            }

            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str("[stderr]\n");
                result.push_str(&stderr);
            }
            result.push_str(&format!("\n[exit code: {}]", exit_code));

            ToolResult::Success(result)
        }
        Ok(Err(e)) => ToolResult::Error(format!("Failed to execute command: {}", e)),
        Err(_) => ToolResult::Error(format!(
            "Shell command timed out after {} seconds",
            timeout.as_secs()
        )),
    }
}

pub async fn tool_web_fetch(args: &HashMap<String, Value>) -> ToolResult {
    let url = match args.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return ToolResult::Error("Missing 'url' argument".to_string()),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => return ToolResult::Error(format!("Failed to create HTTP client: {}", e)),
    };

    match client.get(url).send().await {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                return ToolResult::Error(format!(
                    "HTTP error: {} {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("")
                ));
            }

            match response.text().await {
                Ok(text) => {
                    if text.len() > MAX_WEB_FETCH_SIZE {
                        ToolResult::Success(format!(
                            "{}...\n(truncated, {} bytes total)",
                            &text[..MAX_WEB_FETCH_SIZE],
                            text.len()
                        ))
                    } else {
                        ToolResult::Success(text)
                    }
                }
                Err(e) => ToolResult::Error(format!("Failed to read response: {}", e)),
            }
        }
        Err(e) => ToolResult::Error(format!("Failed to fetch URL: {}", e)),
    }
}
