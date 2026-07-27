use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap};

use crate::models::contact::{Priority, Relationship};
use crate::tui::app::App;
use crate::tui::widgets::status_badge::{
    format_status, priority_style, status_style,
};

pub fn draw_contact_detail(frame: &mut Frame, app: &mut App, contact_idx: usize) {
    let area = frame.area();

    // Vertical split: main area + status bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(1)])
        .split(area);

    // Horizontal split: left 40% (list), right 60% (detail)
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[0]);

    // Left pane: narrow contact list (Name + Status only)
    draw_narrow_list(frame, app, panes[0]);

    // Right pane: contact detail
    if contact_idx < app.contacts.len() {
        draw_detail_pane(frame, app, contact_idx, panes[1]);
    }

    // Status bar
    let status_bar = Paragraph::new(Line::from(vec![
        Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Back  "),
        Span::styled("[l]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Log  "),
        Span::styled("[/]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Search  "),
        Span::styled("[s]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Status  "),
        Span::styled("[p]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Priority  "),
        Span::styled("[r]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Relationship  "),
        Span::styled("[e]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Edit  "),
        Span::styled("[x]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Delete  "),
        Span::styled("[q]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" Quit"),
    ]));
    frame.render_widget(status_bar, outer[1]);
}

fn draw_narrow_list(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let rows: Vec<Row> = app
        .filtered
        .iter()
        .map(|&idx| {
            let cf = &app.contacts[idx];
            let c = &cf.contact;
            let name_cell = Cell::from(c.name.as_str());
            let status_cell =
                Cell::from(format_status(&c.status)).style(status_style(&c.status));
            Row::new(vec![name_cell, status_cell])
        })
        .collect();

    let header = Row::new(vec![Cell::from("Name"), Cell::from("Status")])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let title = format!(" Contacts ({}) ", app.filtered.len());

    let table = Table::new(
        rows,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(title))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_symbol(">> ");

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_detail_pane(
    frame: &mut Frame,
    app: &App,
    contact_idx: usize,
    area: ratatui::layout::Rect,
) {
    let cf = &app.contacts[contact_idx];
    let c = &cf.contact;

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let mut lines: Vec<Line> = Vec::new();

    // Section: Contact Info
    let mut has_contact_info = false;
    if !c.email.is_empty() || !c.phone.is_empty() || !c.address.is_empty() {
        has_contact_info = true;
        lines.push(Line::from(Span::styled("-- Contact Info --", bold)));
    }
    if !c.email.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Email: ", bold),
            Span::raw(c.email.join(", ")),
        ]));
    }
    if !c.phone.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Phone: ", bold),
            Span::raw(c.phone.join(", ")),
        ]));
    }
    if !c.address.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Address: ", bold),
            Span::raw(c.address.join("; ")),
        ]));
    }
    if has_contact_info {
        lines.push(Line::from(""));
    }

    // Section: Professional
    let has_prof = !c.company.is_empty() || !c.role.is_empty() || !c.industry.is_empty() || !c.linkedin.is_empty();
    if has_prof {
        lines.push(Line::from(Span::styled("-- Professional --", bold)));
    }
    if !c.company.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Company: ", bold),
            Span::raw(c.company.as_str()),
        ]));
    }
    if !c.role.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Role: ", bold),
            Span::raw(c.role.as_str()),
        ]));
    }
    if !c.industry.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Industry: ", bold),
            Span::raw(c.industry.as_str()),
        ]));
    }
    if !c.linkedin.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("LinkedIn: ", bold),
            Span::raw(c.linkedin.as_str()),
        ]));
    }
    if has_prof {
        lines.push(Line::from(""));
    }

    // Section: Relationship
    let has_rel = c.status.is_some()
        || c.priority.is_some()
        || c.relationship.is_some()
        || !c.how_we_met.is_empty()
        || c.met_date.is_some()
        || !c.introduced_by.is_empty();
    if has_rel {
        lines.push(Line::from(Span::styled("-- Relationship --", bold)));
    }
    if c.status.is_some() {
        lines.push(Line::from(vec![
            Span::styled("Status: ", bold),
            Span::styled(format_status(&c.status), status_style(&c.status)),
        ]));
    }
    if c.priority.is_some() {
        lines.push(Line::from(vec![
            Span::styled("Priority: ", bold),
            Span::styled(format_priority_name(&c.priority), priority_style(&c.priority)),
        ]));
    }
    if let Some(ref rel) = c.relationship {
        lines.push(Line::from(vec![
            Span::styled("Relationship: ", bold),
            Span::raw(format_relationship(rel)),
        ]));
    }
    if !c.how_we_met.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("How we met: ", bold),
            Span::raw(c.how_we_met.as_str()),
        ]));
    }
    if let Some(d) = c.met_date {
        lines.push(Line::from(vec![
            Span::styled("Met date: ", bold),
            Span::raw(d.format("%Y-%m-%d").to_string()),
        ]));
    }
    if !c.introduced_by.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Introduced by: ", bold),
            Span::raw(c.introduced_by.as_str()),
        ]));
    }
    if has_rel {
        lines.push(Line::from(""));
    }

    // Section: CRM
    let has_crm = !c.follow_up_cadence.is_empty() || c.last_contacted.is_some() || c.next_follow_up.is_some();
    if has_crm {
        lines.push(Line::from(Span::styled("-- CRM --", bold)));
    }
    if !c.follow_up_cadence.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Follow-up cadence: ", bold),
            Span::raw(c.follow_up_cadence.as_str()),
        ]));
    }
    if let Some(d) = c.last_contacted {
        lines.push(Line::from(vec![
            Span::styled("Last contacted: ", bold),
            Span::raw(d.format("%Y-%m-%d").to_string()),
        ]));
    }
    if let Some(d) = c.next_follow_up {
        lines.push(Line::from(vec![
            Span::styled("Next follow-up: ", bold),
            Span::raw(d.format("%Y-%m-%d").to_string()),
        ]));
    }
    if has_crm {
        lines.push(Line::from(""));
    }

    // Section: Tags
    if !c.tags.is_empty() {
        lines.push(Line::from(Span::styled("-- Tags --", bold)));
        lines.push(Line::from(c.tags.join(", ")));
        lines.push(Line::from(""));
    }

    // Section: Social
    let has_social = !c.twitter.is_empty() || !c.github.is_empty() || !c.website.is_empty();
    if has_social {
        lines.push(Line::from(Span::styled("-- Social --", bold)));
    }
    if !c.twitter.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Twitter: ", bold),
            Span::raw(c.twitter.as_str()),
        ]));
    }
    if !c.github.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("GitHub: ", bold),
            Span::raw(c.github.as_str()),
        ]));
    }
    if !c.website.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Website: ", bold),
            Span::raw(c.website.as_str()),
        ]));
    }
    if has_social {
        lines.push(Line::from(""));
    }

    // Section: Interests
    if !c.interests.is_empty() {
        lines.push(Line::from(Span::styled("-- Interests --", bold)));
        lines.push(Line::from(c.interests.join(", ")));
        lines.push(Line::from(""));
    }

    // Sections: Notes and Interaction Log, parsed out of the raw body so the
    // body's own `## Notes` / `## Interaction Log` markdown headers aren't
    // shown redundantly next to this pane's own styled section headers.
    let (notes_text, log_text) = split_body_sections(&cf.body);
    if !notes_text.is_empty() {
        lines.push(Line::from(Span::styled("-- Notes --", bold)));
        for body_line in notes_text.lines() {
            lines.push(Line::from(body_line.to_string()));
        }
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled("-- Interaction Log --", bold)));
    if log_text.is_empty() {
        lines.push(Line::from("(no interactions logged yet)"));
    } else {
        for body_line in log_text.lines() {
            lines.push(Line::from(body_line.to_string()));
        }
    }

    let detail_title = format!(" {} ", c.name);
    let detail = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(detail_title))
        .wrap(Wrap { trim: false });

    frame.render_widget(detail, area);
}

/// Split a contact's raw markdown body into (notes, interaction log) text,
/// stripping the `## Notes` / `## Interaction Log` headers themselves — those
/// headers are what `ops::contact::add`/`log_interaction` always write, so
/// this always finds them on a normally-created contact. Falls back to
/// treating the whole body as the interaction log if the headers are missing
/// (e.g. a hand-edited or externally-imported contact file).
fn split_body_sections(body: &str) -> (String, String) {
    const NOTES_HEADER: &str = "## Notes";
    const LOG_HEADER: &str = "## Interaction Log";

    let notes_start = body.find(NOTES_HEADER);
    let log_start = body.find(LOG_HEADER);

    match (notes_start, log_start) {
        (Some(n), Some(l)) if l > n => {
            let notes = body[n + NOTES_HEADER.len()..l].trim().to_string();
            let log = body[l + LOG_HEADER.len()..].trim().to_string();
            (notes, log)
        }
        _ => (String::new(), body.trim().to_string()),
    }
}

fn format_relationship(rel: &Relationship) -> &'static str {
    match rel {
        Relationship::Friend => "Friend",
        Relationship::Colleague => "Colleague",
        Relationship::Client => "Client",
        Relationship::Mentor => "Mentor",
        Relationship::Mentee => "Mentee",
        Relationship::Acquaintance => "Acquaintance",
        Relationship::Family => "Family",
        Relationship::Neighbor => "Neighbor",
        Relationship::FamilyFriend => "Family Friend",
        Relationship::Other => "Other",
    }
}

fn format_priority_name(priority: &Option<Priority>) -> &'static str {
    match priority {
        Some(Priority::High) => "High",
        Some(Priority::Medium) => "Medium",
        Some(Priority::Low) => "Low",
        None => "",
    }
}
