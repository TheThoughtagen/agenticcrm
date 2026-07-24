use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};

use crate::tui::app::{App, InputMode, PRIORITY_OPTIONS, RELATIONSHIP_OPTIONS, STATUS_OPTIONS};
use crate::tui::widgets::search_bar::draw_search_bar;
use crate::tui::widgets::status_badge::{
    format_priority, format_relationship, format_status, priority_style, relationship_style,
    status_style,
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

    // Search bar widget
    if show_search {
        let active = app.input_mode == InputMode::Search;
        draw_search_bar(frame, chunks[0], &app.search_query, active);
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

            let relationship_cell = Cell::from(format_relationship(&c.relationship))
                .style(relationship_style(&c.relationship));

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
                relationship_cell,
                last_contacted_cell,
            ])
        })
        .collect();

    let sorted_column = app.sort_mode.label();
    let column_header = |base: &str| -> String {
        if sorted_column == Some(base) {
            format!("{} \u{25bc}", base)
        } else {
            base.to_string()
        }
    };
    let header = Row::new(vec![
        Cell::from(column_header("Pri")),
        Cell::from(column_header("Name")),
        Cell::from(column_header("Company")),
        Cell::from(column_header("Status")),
        Cell::from(column_header("Relationship")),
        Cell::from(column_header("Last Contacted")),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);

    let mut title = if show_search {
        format!(
            " Contacts ({}/{}) ",
            app.filtered.len(),
            app.contacts.len()
        )
    } else {
        format!(" Contacts ({}) ", app.filtered.len())
    };
    // Active quick-filter badges, so it's obvious why the list is short.
    if app.status_filter_index != 0 {
        title.push_str(&format!("[Status: {}] ", STATUS_OPTIONS[app.status_filter_index]));
    }
    if app.priority_filter_index != 0 {
        title.push_str(&format!(
            "[Priority: {}] ",
            PRIORITY_OPTIONS[app.priority_filter_index]
        ));
    }
    if app.relationship_filter_index != 0 {
        title.push_str(&format!(
            "[Relationship: {}] ",
            RELATIONSHIP_OPTIONS[app.relationship_filter_index]
        ));
    }

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(5),
            Constraint::Percentage(20),
            Constraint::Percentage(18),
            Constraint::Percentage(13),
            Constraint::Percentage(14),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(title))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");

    frame.render_stateful_widget(table, chunks[1], &mut app.table_state);

    // Status bar
    let status_bar = if app.input_mode == InputMode::Search {
        Paragraph::new(Line::from(vec![
            Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Clear  "),
            Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Confirm  "),
            Span::raw("Type to filter..."),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("[j/k]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Nav  "),
            Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Details  "),
            Span::styled("[/]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Search  "),
            Span::styled("[s/p/r]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Filter  "),
            Span::styled("[t]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Sort  "),
            Span::styled("[d]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Dash  "),
            Span::styled("[l]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Log  "),
            Span::styled("[n]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" New  "),
            Span::styled("[x]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Delete  "),
            Span::styled("[q]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Quit"),
        ]))
    };
    frame.render_widget(status_bar, chunks[2]);
}
