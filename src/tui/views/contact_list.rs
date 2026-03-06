use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::tui::app::{App, InputMode};
use crate::tui::widgets::status_badge::{
    format_priority, format_status, priority_style, status_style,
};

pub fn draw_contact_list(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Determine if search bar is visible
    let show_search = app.input_mode == InputMode::Search || !app.search_query.is_empty();

    let chunks = if show_search {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // search bar
                Constraint::Min(5),   // table
                Constraint::Length(1), // status bar
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(0), // hidden search bar
                Constraint::Min(5),   // table
                Constraint::Length(1), // status bar
            ])
            .split(area)
    };

    // Search bar
    if show_search {
        let cursor = if app.input_mode == InputMode::Search {
            "_"
        } else {
            ""
        };
        let search_text = format!("/ {}{}", app.search_query, cursor);
        let search_bar = Paragraph::new(search_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search "),
        );
        frame.render_widget(search_bar, chunks[0]);
    }

    // Build table rows from filtered contacts
    let rows: Vec<Row> = app
        .filtered
        .iter()
        .map(|&idx| {
            let cf = &app.contacts[idx];
            let c = &cf.contact;

            let priority_cell =
                Cell::from(format_priority(&c.priority)).style(priority_style(&c.priority));

            let name_cell = Cell::from(c.name.as_str());

            let company_cell = Cell::from(c.company.as_str());

            let status_cell =
                Cell::from(format_status(&c.status)).style(status_style(&c.status));

            let last_contacted_cell = Cell::from(
                c.last_contacted
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "never".to_string()),
            );

            Row::new(vec![
                priority_cell,
                name_cell,
                company_cell,
                status_cell,
                last_contacted_cell,
            ])
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Pri"),
        Cell::from("Name"),
        Cell::from("Company"),
        Cell::from("Status"),
        Cell::from("Last Contacted"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);

    let title = format!(" Contacts ({}) ", app.filtered.len());

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(5),
            Constraint::Percentage(30),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(title))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");

    frame.render_stateful_widget(table, chunks[1], &mut app.table_state);

    // Status bar
    let status_bar = Paragraph::new(Line::from(vec![
        Span::styled("[j/k]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Navigate  "),
        Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Details  "),
        Span::styled("[/]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Search  "),
        Span::styled("[d]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Dashboard  "),
        Span::styled("[l]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Log  "),
        Span::styled("[q]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Quit"),
    ]));
    frame.render_widget(status_bar, chunks[2]);
}
