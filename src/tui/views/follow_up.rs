use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::tui::app::App;

struct DashboardEntry {
    name: String,
    follow_up_date: String,
    days_diff: i64,
    last_contacted: String,
}

pub fn draw_follow_up_dashboard(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let today = Local::now().date_naive();

    // Compute overdue and upcoming lists from in-memory contacts
    let mut overdue: Vec<DashboardEntry> = Vec::new();
    let mut upcoming: Vec<DashboardEntry> = Vec::new();

    for cf in &app.contacts {
        if let Some(next_fu) = cf.contact.next_follow_up {
            let days = (next_fu - today).num_days();
            let entry = DashboardEntry {
                name: cf.contact.name.clone(),
                follow_up_date: next_fu.format("%Y-%m-%d").to_string(),
                days_diff: days,
                last_contacted: cf
                    .contact
                    .last_contacted
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "never".to_string()),
            };
            if days < 0 {
                overdue.push(entry);
            } else if days <= 14 {
                upcoming.push(entry);
            }
        }
    }

    // Sort overdue by date ascending (most overdue first = most negative days_diff)
    overdue.sort_by_key(|e| e.days_diff);
    // Sort upcoming by date ascending
    upcoming.sort_by_key(|e| e.days_diff);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // overdue
            Constraint::Percentage(50), // upcoming
            Constraint::Length(1),      // status bar
        ])
        .split(area);

    // -- Overdue table --
    let overdue_title = format!(" Overdue ({}) ", overdue.len());
    let overdue_rows: Vec<Row> = overdue
        .iter()
        .map(|e| {
            let days_text = format!("{} days", e.days_diff.unsigned_abs());
            Row::new(vec![
                Cell::from(e.name.as_str()),
                Cell::from(e.follow_up_date.as_str()),
                Cell::from(days_text).style(
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(e.last_contacted.as_str()),
            ])
            .style(Style::default().fg(Color::Red))
        })
        .collect();

    let overdue_header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Next Follow-Up"),
        Cell::from("Days Overdue"),
        Cell::from("Last Contacted"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);

    let overdue_table = Table::new(
        overdue_rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
        ],
    )
    .header(overdue_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(overdue_title),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(overdue_table, chunks[0], &mut app.dashboard_state);

    // -- Upcoming table --
    let upcoming_title = format!(" Upcoming ({}) ", upcoming.len());
    let upcoming_rows: Vec<Row> = upcoming
        .iter()
        .map(|e| {
            let days_text = format!("{} days", e.days_diff);
            let row_style = if e.days_diff <= 3 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(e.name.as_str()),
                Cell::from(e.follow_up_date.as_str()),
                Cell::from(days_text),
                Cell::from(e.last_contacted.as_str()),
            ])
            .style(row_style)
        })
        .collect();

    let upcoming_header = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Next Follow-Up"),
        Cell::from("Days Until"),
        Cell::from("Last Contacted"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);

    let upcoming_table = Table::new(
        upcoming_rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
        ],
    )
    .header(upcoming_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(upcoming_title),
    );

    frame.render_widget(upcoming_table, chunks[1]);

    // Status bar
    let status_bar = Paragraph::new(Line::from(vec![
        Span::styled("[j/k]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Navigate  "),
        Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Back to List  "),
        Span::styled("[q]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Quit"),
    ]));
    frame.render_widget(status_bar, chunks[2]);
}
