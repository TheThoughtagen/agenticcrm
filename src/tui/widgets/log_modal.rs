use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::app::{LogField, LogModalState, VimMode};

/// Draw a centered modal overlay for logging interactions.
///
/// Takes `modal: &mut` because the Summary field's underlying `TextArea` needs its
/// block/style set immediately before each render (the standard tui-textarea
/// integration pattern) — this is a rendering-time style refresh, not a state
/// mutation that affects behavior.
pub fn draw_log_modal(frame: &mut Frame, modal: &mut LogModalState, contact_name: &str) {
    let area = centered_rect(70, 60, frame.area());

    // Clear the area behind the modal
    frame.render_widget(Clear, area);

    let title = format!(" Log Interaction -- {} ", contact_name);
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Type field
            Constraint::Min(6),    // Summary field (multi-line)
            Constraint::Length(1), // Help line
        ])
        .split(inner);

    // Type field
    let type_active = modal.active_field == LogField::Type;
    let type_border_style = if type_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let left_arrow = if type_active { "< " } else { "  " };
    let right_arrow = if type_active { " >" } else { "  " };
    let type_text = format!(
        "{}{}{}",
        left_arrow, modal.type_options[modal.type_index], right_arrow
    );
    let type_paragraph = Paragraph::new(type_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(type_border_style)
            .title(" Type "),
    );
    frame.render_widget(type_paragraph, chunks[0]);

    // Summary field: a real multi-line, cursor-navigable textarea with vim-lite
    // Normal/Insert modes (see LogModalState::handle_summary_key).
    let summary_active = modal.active_field == LogField::Summary;
    let summary_border_style = if summary_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let mode_label = match modal.summary_vim_mode {
        VimMode::Insert => "INSERT",
        VimMode::Normal => "NORMAL",
    };
    let summary_title = if summary_active {
        format!(" Summary [{}] ", mode_label)
    } else {
        " Summary ".to_string()
    };
    modal.summary.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(summary_border_style)
            .title(summary_title),
    );
    // Cursor line highlight only makes sense while this field is actually focused.
    if summary_active {
        modal
            .summary
            .set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
        modal.summary.set_cursor_style(Style::default().bg(
            if modal.summary_vim_mode == VimMode::Insert {
                Color::Yellow
            } else {
                Color::Cyan
            },
        ));
    } else {
        modal.summary.set_cursor_line_style(Style::default());
        modal.summary.set_cursor_style(Style::default());
    }
    frame.render_widget(&modal.summary, chunks[1]);

    // Help line
    let help = if summary_active && modal.summary_vim_mode == VimMode::Insert {
        Paragraph::new(Line::from(vec![
            Span::styled("[Tab]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Switch field  "),
            Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Normal mode"),
        ]))
    } else if summary_active {
        Paragraph::new(Line::from(vec![
            Span::styled("[hjkl/w/b/e/gg/G]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Move  "),
            Span::styled("[i/a/o]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Insert  "),
            Span::styled("[x/dd/yy/p/u]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Edit  "),
            Span::styled("[ZZ]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Submit"),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("[Tab]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Switch field  "),
            Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Submit  "),
            Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" Cancel"),
        ]))
    };
    frame.render_widget(help, chunks[2]);
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
