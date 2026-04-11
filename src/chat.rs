use anyhow::Result;
use tokio::io::AsyncBufReadExt;

use crate::client::{Message, OpenRouterClient};
use crate::attachment::{load_attachment, parse_attachments_from_text};

pub async fn run_chat(cli: &crate::cli::Cli) -> Result<()> {
    let client = OpenRouterClient::new(cli).await?;
    let mut messages: Vec<Message> = Vec::new();
    let mut initial_attachments = Vec::new();

    for path in &cli.attach {
        let att = load_attachment(path)?;
        eprintln!("Attached: {}", att.filename);
        initial_attachments.push(att);
    }

    eprintln!("orai chat (model: {})", cli.model);
    eprintln!("Type /quit to exit, /clear to reset conversation\n");

    let stdin = tokio::io::BufReader::new(tokio::io::stdin());

    let mut lines = stdin.lines();

    loop {
        eprint!("You: ");
        let _input = String::new();
        std::io::Write::flush(&mut std::io::stderr())?;

        if lines.next_line().await?.is_none() {
            break;
        }

        let line: String = match lines.next_line().await {
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                continue;
            }
        };

        let input = line.trim().to_string();

        if input.is_empty() {
            continue;
        }

        if input == "/quit" || input == "/exit" {
            break;
        }

        if input == "/clear" {
            messages.clear();
            eprintln!("Conversation cleared.\n");
            continue;
        }

        let (clean_input, attach_paths) = parse_attachments_from_text(&input);
        let mut attachments = initial_attachments.clone();

        for path in &attach_paths {
            match load_attachment(path) {
                Ok(att) => {
                    eprintln!("Attached: {}", att.filename);
                    attachments.push(att);
                }
                Err(e) => eprintln!("Error loading attachment '{}': {}", path, e),
            }
        }

        messages.push(Message {
            role: "user".to_string(),
            content: Some(serde_json::Value::String(clean_input.clone())),
            tool_calls: None,
            tool_call_id: None,
        });

        eprint!("Assistant: ");

        match client.send_with_agentic_loop(&mut messages, &attachments).await {
            Ok(response) => {
                println!("{}", response);
                println!();
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}