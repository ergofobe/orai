use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tui_textarea::TextArea;

use super::App;

pub fn render(f: &mut Frame, app: &App, textarea: &TextArea) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(5),    // messages
            Constraint::Length(3), // input separator
            Constraint::Length(8), // textarea
        ])
        .split(size);

    render_title(f, chunks[0], app);
    render_messages(f, chunks[1], app);
    render_input_label(f, chunks[2]);
    f.render_widget(textarea, chunks[3]);

    if let Some(popup) = &app.popup {
        render_popup(f, popup, size);
    }
}

fn render_title(f: &mut Frame, area: Rect, app: &App) {
    let title = format!(" orai ─ {} ", app.model);
    let streaming = if app.is_streaming {
        " ● streaming"
    } else {
        ""
    };
    let full_title = format!("{}{}", title, streaming);

    let paragraph = Paragraph::new(full_title).style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(paragraph, area);
}

fn render_messages(f: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<ratatui::text::Line> = Vec::new();

    for msg in &app.messages {
        let (prefix, style) = match msg.role {
            super::Role::User => (
                "You: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            super::Role::Assistant => (
                "Assistant: ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            super::Role::System => (
                "System: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(prefix.to_string(), style),
        ]));

        let msg_lines = crate::markdown::markdown_to_lines(&msg.content);
        lines.extend(msg_lines);
        lines.push(ratatui::text::Line::from(""));
    }

    if !app.current_response.is_empty() {
        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(
                "Assistant: ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let stream_lines = crate::markdown::markdown_to_lines(&app.current_response);
        lines.extend(stream_lines);

        lines.push(ratatui::text::Line::from(vec![
            ratatui::text::Span::styled(" ● ", Style::default().fg(Color::Yellow)),
        ]));
    }

    let messages = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));

    f.render_widget(messages, area);
}

fn render_input_label(f: &mut Frame, area: Rect) {
    let label = Paragraph::new(
        " Input (Enter to send, Ctrl+J/Shift+Enter for newline, Ctrl+A attach file, Esc to quit) ",
    )
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(label, area);
}

fn render_popup(f: &mut Frame, popup: &super::Popup, area: Rect) {
    let popup_area = centered_rect(60, 20, area);

    match popup {
        super::Popup::FilePicker {
            input,
            completions,
            selected,
        } => {
            let mut lines: Vec<ratatui::text::Line> = vec![
                ratatui::text::Line::from(" Attach File"),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from(format!(" Path: {}", input)),
                ratatui::text::Line::from(""),
            ];

            for (i, path) in completions.iter().enumerate() {
                let style = if i == *selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default()
                };
                lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                    format!(" {}", path.to_string_lossy()),
                    style,
                )));
            }

            lines.push(ratatui::text::Line::from(""));
            lines.push(ratatui::text::Line::from(
                " Enter=attach  Tab=complete  Esc=cancel",
            ));

            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default()),
            );
            f.render_widget(paragraph, popup_area);
        }
        super::Popup::ToolConfirm {
            tool_name,
            arguments,
        } => {
            let display_args = if arguments.len() > 200 {
                format!("{}...", &arguments[..200])
            } else {
                arguments.clone()
            };

            let lines = vec![
                ratatui::text::Line::from(" Tool Call Confirmation"),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                    format!(" ⚙ {}({})", tool_name, display_args),
                    Style::default().fg(Color::Yellow),
                )]),
                ratatui::text::Line::from(""),
                ratatui::text::Line::from(" [Y] Approve  [N] Deny  [A] Approve all"),
            ];

            let paragraph = Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default()),
            );
            f.render_widget(paragraph, popup_area);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
