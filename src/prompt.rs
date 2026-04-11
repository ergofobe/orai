use anyhow::Result;
use tokio::io::AsyncWriteExt;

use crate::client::{Message, OpenRouterClient};
use crate::attachment::load_attachment;

pub async fn run_prompt(cli: &crate::cli::Cli, prompt_args: &crate::cli::Commands) -> Result<()> {
    let prompt_text = match prompt_args {
        crate::cli::Commands::Prompt { prompt, .. } => prompt.join(" "),
        _ => unreachable!(),
    };

    let no_stream = match prompt_args {
        crate::cli::Commands::Prompt { no_stream, .. } => *no_stream,
        _ => false,
    };

    let mut attachments = Vec::new();
    for path in &cli.attach {
        let att = load_attachment(path)?;
        eprintln!("Attached: {}", att.filename);
        attachments.push(att);
    }

    let client = OpenRouterClient::new(cli).await?;
    let mut messages = vec![Message {
        role: "user".to_string(),
        content: Some(serde_json::Value::String(prompt_text.clone())),
        tool_calls: None,
        tool_call_id: None,
    }];

    if no_stream {
        let response = client.send_with_agentic_loop(&mut messages, &attachments).await?;
        println!("{}", response);
    } else {
        run_streaming_prompt(&client, &mut messages, &attachments).await?;
    }

    Ok(())
}

fn run_streaming_prompt<'a>(
    client: &'a OpenRouterClient,
    messages: &'a mut Vec<Message>,
    attachments: &'a [crate::attachment::Attachment],
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut stdout = tokio::io::BufWriter::new(tokio::io::stdout());
        let mut full_response = String::new();

        let mut response = client.send_request(messages, attachments, true).await?;

        if response.tool_calls.is_some() {
            let tool_calls = response.tool_calls.take().unwrap();
            messages.push(response);

            for tool_call in &tool_calls {
                let info = crate::tools::ToolCallInfo {
                    id: tool_call.id.clone(),
                    name: tool_call.function.name.clone(),
                    arguments: tool_call.function.arguments.clone(),
                };

                eprintln!("\n⚙ {}({})", info.name, truncate_for_display(&info.arguments, 80));

                let result = crate::tools::execute_native_tool(&info.name, &info.arguments, client.tool_config()).await;

                match &result {
                    crate::tools::ToolResult::Success(msg) => {
                        eprintln!("→ {}", truncate_for_display(msg, 80));
                    }
                    crate::tools::ToolResult::Error(msg) => {
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

            return run_streaming_prompt(client, messages, attachments).await;
        }

        let content = response.content.clone().unwrap_or_default();
        let content_str = content.to_string();
        if !content_str.is_empty() {
            print!("{}", content_str);
            stdout.flush().await?;
            full_response.push_str(&content_str);
        }

        println!();
        Ok(())
    })
}

fn truncate_for_display(s: &str, max_len: usize) -> String {
    let single_line: String = s.lines().collect::<Vec<_>>().join(" ");
    if single_line.len() > max_len {
        format!("{}...", &single_line[..max_len])
    } else {
        single_line
    }
}
