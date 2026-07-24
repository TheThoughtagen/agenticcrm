use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::app::{EditContactField, EditContactState};

/// Draw a centered modal overlay for editing Company/Email/Phone/Birthday on
/// an existing contact. Every field is pre-populated from its current value.
pub fn draw_edit_contact_modal(frame: &mut Frame, modal: &EditContactState, contact_name: &str) {
    let area = centered_rect(60, 45, frame.area());

    frame.render_widget(Clear, area);

    let title = format!(" Edit Contact -- {} ", contact_name);
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Company
            Constraint::Length(3), // Email
            Constraint::Length(3), // Phone
            Constraint::Length(3), // Birthday
            Constraint::Length(1), // Help line
        ])
        .split(inner);

    draw_text_field(
        frame,
        chunks[0],
        " Company ",
        &modal.company,
        modal.active_field == EditContactField::Company,
    );
    draw_text_field(
        frame,
        chunks[1],
        " Email (comma-separated) ",
        &modal.email,
        modal.active_field == EditContactField::Email,
    );
    draw_text_field(
        frame,
        chunks[2],
        " Phone (comma-separated) ",
        &modal.phone,
        modal.active_field == EditContactField::Phone,
    );
    draw_text_field(
        frame,
        chunks[3],
        " Birthday (YYYY-MM-DD) ",
        &modal.birthday,
        modal.active_field == EditContactField::Birthday,
    );

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Next field  "),
        Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Save  "),
        Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Cancel"),
    ]));
    frame.render_widget(help, chunks[4]);
}

fn draw_text_field(frame: &mut Frame, area: Rect, title: &str, value: &str, active: bool) {
    let style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let text = if active {
        format!("{}_", value)
    } else {
        value.to_string()
    };
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title),
    );
    frame.render_widget(paragraph, area);
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
