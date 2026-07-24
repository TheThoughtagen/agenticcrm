use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::app::{
    CreateContactField, CreateContactState, PRIORITY_OPTIONS, RELATIONSHIP_OPTIONS, STATUS_OPTIONS,
};

/// Draw a centered modal overlay for creating a new contact, with relationship,
/// status, and priority pickers alongside the name so all four can be set up
/// front instead of requiring a follow-up `acrm edit`.
pub fn draw_create_contact_modal(frame: &mut Frame, modal: &CreateContactState) {
    let area = centered_rect(60, 55, frame.area());

    // Clear the area behind the modal
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" New Contact ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(3), // Company
            Constraint::Length(3), // Relationship
            Constraint::Length(3), // Status
            Constraint::Length(3), // Priority
            Constraint::Length(1), // Help line
        ])
        .split(inner);

    draw_text_field(
        frame,
        chunks[0],
        " Name ",
        &format!("{}_", modal.name),
        modal.active_field == CreateContactField::Name,
    );
    draw_text_field(
        frame,
        chunks[1],
        " Company ",
        &format!("{}_", modal.company),
        modal.active_field == CreateContactField::Company,
    );
    draw_picker_field(
        frame,
        chunks[2],
        " Relationship ",
        RELATIONSHIP_OPTIONS[modal.relationship_index],
        modal.active_field == CreateContactField::Relationship,
    );
    draw_picker_field(
        frame,
        chunks[3],
        " Status ",
        STATUS_OPTIONS[modal.status_index],
        modal.active_field == CreateContactField::Status,
    );
    draw_picker_field(
        frame,
        chunks[4],
        " Priority ",
        PRIORITY_OPTIONS[modal.priority_index],
        modal.active_field == CreateContactField::Priority,
    );

    let help = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Next field  "),
        Span::styled("[←/→]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Change  "),
        Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Create  "),
        Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Cancel"),
    ]));
    frame.render_widget(help, chunks[5]);
}

fn draw_text_field(frame: &mut Frame, area: Rect, title: &str, text: &str, active: bool) {
    let style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(style)
            .title(title),
    );
    frame.render_widget(paragraph, area);
}

fn draw_picker_field(frame: &mut Frame, area: Rect, title: &str, value: &str, active: bool) {
    let style = if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let left_arrow = if active { "< " } else { "  " };
    let right_arrow = if active { " >" } else { "  " };
    let text = format!("{}{}{}", left_arrow, value, right_arrow);
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
