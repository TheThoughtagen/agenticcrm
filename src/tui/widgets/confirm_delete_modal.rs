use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Draw a centered y/n delete-confirmation modal for the named contact.
pub fn draw_confirm_delete_modal(frame: &mut Frame, contact_name: &str) {
    let area = centered_rect(46, 20, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Delete Contact ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(1)])
        .split(inner);

    let message = Paragraph::new(Line::from(format!(
        "Delete \"{}\"? This permanently removes the contact file and cannot be undone.",
        contact_name
    )))
    .wrap(Wrap { trim: true });
    frame.render_widget(message, chunks[0]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "[y]",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Delete  "),
        Span::styled("[n/Esc]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Cancel"),
    ]));
    frame.render_widget(help, chunks[1]);
}

/// Create a centered rectangle using percentage of parent area.
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
