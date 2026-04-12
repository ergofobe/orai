pub mod input;
pub mod render;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::attachment::{load_attachment, parse_attachments_from_text};
use crate::client::{Message, OpenRouterClient};

#[allow(dead_code)]
const MAX_TOOL_ITERATIONS: u32 = 25;

pub struct App {
    pub messages: Vec<ChatMessage>,
    pub current_response: String,
    pub is_streaming: bool,
    pub model: String,
    pub auto_scroll: bool,
    pub scroll_offset: u16,
    pub should_quit: bool,
    pub popup: Option<Popup>,
    pub approve_all: bool,
}

pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Clone, Copy)]
pub enum Role {
    User,
    #[allow(dead_code)]
    Assistant,
    System,
}

#[derive(Clone)]
pub enum Popup {
    FilePicker {
        input: String,
        completions: Vec<std::path::PathBuf>,
        selected: usize,
    },
    #[allow(dead_code)]
    ToolConfirm {
        tool_name: String,
        arguments: String,
    },
}

pub async fn run_tui(cli: &crate::cli::Cli) -> Result<()> {
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(std::io::stderr());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App {
        messages: Vec::new(),
        current_response: String::new(),
        is_streaming: false,
        model: cli.model.clone(),
        auto_scroll: true,
        scroll_offset: 0,
        should_quit: false,
        popup: None,
        approve_all: cli.yes,
    };

    let client = OpenRouterClient::new(cli).await?;

    let mut initial_attachments = Vec::new();
    for path in &cli.attach {
        let att = load_attachment(path)?;
        eprintln!("Attached: {}", att.filename);
        initial_attachments.push(att);
    }

    let mut textarea = input::create_textarea();

    let mut api_messages: Vec<Message> = Vec::new();

    let (response_tx, mut response_rx): (tokio::sync::mpsc::UnboundedSender<Message>, _) =
        tokio::sync::mpsc::unbounded_channel();

    loop {
        terminal.draw(|f| render::render(f, &app, &textarea))?;

        if let Ok(response_msg) = response_rx.try_recv() {
            let content = match &response_msg.content {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            if !content.is_empty() {
                app.messages.push(ChatMessage {
                    role: Role::Assistant,
                    content,
                });
                api_messages.push(response_msg);
            }
            app.is_streaming = false;
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if handle_key_event(
                        key,
                        &mut app,
                        &mut textarea,
                        &client,
                        &mut api_messages,
                        &initial_attachments,
                        &response_tx,
                    )
                    .await
                    {
                        break;
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    terminal.show_cursor()?;

    Ok(())
}

async fn handle_key_event(
    key: KeyEvent,
    app: &mut App,
    textarea: &mut tui_textarea::TextArea<'_>,
    client: &OpenRouterClient,
    api_messages: &mut Vec<Message>,
    attachments: &[crate::attachment::Attachment],
    response_tx: &tokio::sync::mpsc::UnboundedSender<Message>,
) -> bool {
    if app.is_streaming {
        return false;
    }

    match key.code {
        KeyCode::Esc => {
            if app.popup.is_some() {
                app.popup = None;
                return false;
            }
            app.should_quit = true;
            return true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.popup.is_some() {
                app.popup = None;
                return false;
            }
            app.should_quit = true;
            return true;
        }
        KeyCode::Enter if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            if app.popup.is_some() {
                handle_popup_event(key, app);
                return false;
            }

            let input = textarea.lines().join("\n");
            if input.trim().is_empty() {
                return false;
            }

            if input.trim() == "/quit" || input.trim() == "/exit" {
                app.should_quit = true;
                return true;
            }

            if input.trim() == "/clear" {
                app.messages.clear();
                api_messages.clear();
                app.current_response.clear();
                textarea.select_all();
                textarea.cut();
                return false;
            }

            let (clean_input, attach_paths) = parse_attachments_from_text(&input);
            let mut all_attachments = attachments.to_vec();

            for path in &attach_paths {
                match load_attachment(path) {
                    Ok(att) => all_attachments.push(att),
                    Err(e) => {
                        app.messages.push(ChatMessage {
                            role: Role::System,
                            content: format!("Error loading '{}': {}", path, e),
                        });
                    }
                }
            }

            app.messages.push(ChatMessage {
                role: Role::User,
                content: clean_input.clone(),
            });

            api_messages.push(Message {
                role: "user".to_string(),
                content: Some(serde_json::Value::String(clean_input)),
                tool_calls: None,
                tool_call_id: None,
            });

            textarea.select_all();
            textarea.cut();
            app.is_streaming = true;
            app.current_response.clear();
            app.auto_scroll = true;

            let tx = response_tx.clone();
            let client_clone = client.clone();
            let msgs_clone = api_messages.clone();
            let att_clone = all_attachments.clone();
            tokio::spawn(async move {
                let mut msgs = msgs_clone;
                let result = client_clone
                    .send_with_agentic_loop(&mut msgs, &att_clone)
                    .await;
                let msg = match result {
                    Ok(response) => Message {
                        role: "assistant".to_string(),
                        content: Some(serde_json::Value::String(response)),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                    Err(e) => Message {
                        role: "assistant".to_string(),
                        content: Some(serde_json::Value::String(format!("Error: {}", e))),
                        tool_calls: None,
                        tool_call_id: None,
                    },
                };
                let _ = tx.send(msg);
            });
        }
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            textarea.insert_newline();
        }
        KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            textarea.insert_newline();
        }
        KeyCode::Char('a')
            if key.modifiers.contains(KeyModifiers::CONTROL) && app.popup.is_none() =>
        {
            app.popup = Some(Popup::FilePicker {
                input: String::new(),
                completions: Vec::new(),
                selected: 0,
            });
        }
        _ => {
            if app.popup.is_some() {
                handle_popup_event(key, app);
            } else {
                textarea.input(key);
            }
        }
    }

    false
}

fn handle_popup_event(key: KeyEvent, app: &mut App) {
    match &mut app.popup {
        Some(Popup::FilePicker {
            input,
            completions,
            selected,
        }) => match key.code {
            KeyCode::Esc => {
                app.popup = None;
            }
            KeyCode::Enter => {
                if !completions.is_empty() && *selected < completions.len() {
                    let path = completions[*selected].to_string_lossy().to_string();
                    input.insert_str(input.len(), &path);
                }
                let _filename = input.clone();
                app.popup = None;
            }
            KeyCode::Tab => {
                if !input.is_empty() {
                    let path = std::path::Path::new(&*input);
                    if let Ok(entries) =
                        std::fs::read_dir(path.parent().unwrap_or(std::path::Path::new(".")))
                    {
                        *completions = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.path())
                            .filter(|p| {
                                p.to_string_lossy().starts_with(&*input)
                                    || p.file_name()
                                        .is_some_and(|f| f.to_string_lossy().starts_with(&*input))
                            })
                            .collect();
                        *selected = 0;
                    }
                }
            }
            KeyCode::Up => {
                if *selected > 0 {
                    *selected -= 1;
                }
            }
            KeyCode::Down => {
                if *selected + 1 < completions.len() {
                    *selected += 1;
                }
            }
            KeyCode::Char(c) => {
                input.push(c);
            }
            KeyCode::Backspace => {
                input.pop();
            }
            _ => {}
        },
        Some(Popup::ToolConfirm {
            tool_name: _,
            arguments: _,
        }) => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.popup = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                app.popup = None;
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                app.approve_all = true;
                app.popup = None;
            }
            _ => {}
        },
        None => {}
    }
}
