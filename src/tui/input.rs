use crossterm::event::{KeyCode, KeyEvent};
use tui_textarea::TextArea;

pub fn create_textarea() -> TextArea<'static> {
    let mut textarea = TextArea::default();
    textarea.set_block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::NONE));
    textarea
}

#[allow(dead_code)]
pub fn handle_input(key: KeyEvent, textarea: &mut TextArea) -> Option<String> {
    match key.code {
        KeyCode::Enter
            if !key
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT) =>
        {
            let text = textarea.lines().join("\n");
            if !text.trim().is_empty() {
                textarea.select_all();
                textarea.cut();
                Some(text)
            } else {
                None
            }
        }
        KeyCode::Char('j')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            textarea.insert_newline();
            None
        }
        _ => {
            textarea.input(key);
            None
        }
    }
}
