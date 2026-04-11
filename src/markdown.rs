use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use pulldown_cmark::TagEnd;

pub fn markdown_to_lines(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let parser = pulldown_cmark::Parser::new_ext(text, pulldown_cmark::Options::all());

    let mut in_code_block = false;
    let mut code_block_lines: Vec<String> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();

    for event in parser {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(_kind)) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                in_code_block = true;
                code_block_lines.clear();
            }
            pulldown_cmark::Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                lines.push(Line::from(""));
                for code_line in &code_block_lines {
                    lines.push(Line::from(vec![Span::styled(
                        code_line.clone(),
                        Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                    )]));
                }
                lines.push(Line::from(""));
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, .. }) => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                let style = match level {
                    pulldown_cmark::HeadingLevel::H1 => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    pulldown_cmark::HeadingLevel::H2 => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                };
                current_spans.push(Span::styled("", style));
            }
            pulldown_cmark::Event::End(TagEnd::Heading(_)) => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
                lines.push(Line::from(""));
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Emphasis) => {
                current_spans.push(Span::styled(
                    "",
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
            }
            pulldown_cmark::Event::End(TagEnd::Emphasis) => {}
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Strong) => {
                current_spans.push(Span::styled(
                    "",
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
            pulldown_cmark::Event::End(TagEnd::Strong) => {}
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::BlockQuote(_)) => {
                lines.push(Line::from(""));
            }
            pulldown_cmark::Event::End(TagEnd::BlockQuote(_)) => {
                lines.push(Line::from(""));
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Paragraph) => {}
            pulldown_cmark::Event::End(TagEnd::Paragraph) => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
                lines.push(Line::from(""));
            }
            pulldown_cmark::Event::Code(code) => {
                current_spans.push(Span::styled(
                    code.to_string(),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
            }
            pulldown_cmark::Event::Text(text) => {
                if in_code_block {
                    code_block_lines.extend(text.lines().map(|l| l.to_string()));
                } else {
                    let last_style = current_spans.last().map(|s| s.style).unwrap_or_default();
                    current_spans.push(Span::styled(text.to_string(), last_style));
                }
            }
            pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => {
                if !in_code_block {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { .. }) => {
                current_spans.push(Span::styled(
                    "",
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED),
                ));
            }
            pulldown_cmark::Event::End(TagEnd::Link) => {}
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::List(_)) => {}
            pulldown_cmark::Event::End(TagEnd::List(_)) => {}
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Item) => {
                current_spans.push(Span::raw("  • "));
            }
            pulldown_cmark::Event::End(TagEnd::Item) => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
            }
            _ => {}
        }
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    lines
}

#[allow(dead_code)]
pub fn plain_text_to_lines(text: &str) -> Vec<Line<'static>> {
    text.lines().map(|l| Line::from(l.to_string())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_bold() {
        let lines = markdown_to_lines("**hello**");
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_markdown_code() {
        let lines = markdown_to_lines("`foo`");
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_plain_text() {
        let lines = plain_text_to_lines("hello\nworld");
        assert_eq!(lines.len(), 2);
    }
}
