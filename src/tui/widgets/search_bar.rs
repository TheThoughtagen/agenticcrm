use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn draw_search_bar(frame: &mut Frame, area: Rect, query: &str, active: bool) {
    let (text, border_style) = if active {
        let display = format!("/ {}_", query);
        (display, Style::default().fg(Color::Yellow))
    } else if !query.is_empty() {
        let display = format!("/ {}", query);
        (
            display,
            Style::default().fg(Color::DarkGray),
        )
    } else {
        return; // Don't render if inactive and empty
    };

    let text_style = if active {
        Style::default()
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };

    let search_bar = Paragraph::new(text)
        .style(text_style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Search "),
        );
    frame.render_widget(search_bar, area);
}
