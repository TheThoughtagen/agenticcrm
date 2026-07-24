use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

use super::app::{App, Screen};
use super::views;
use super::widgets;

pub fn draw(frame: &mut Frame, app: &mut App) {
    // Draw the current screen
    match app.screen {
        Screen::ContactList => views::contact_list::draw_contact_list(frame, app),
        Screen::ContactDetail(idx) => views::contact_detail::draw_contact_detail(frame, app, idx),
        Screen::FollowUpDashboard => views::follow_up::draw_follow_up_dashboard(frame, app),
    }

    // Overlay log modal if active
    if let Some(ref mut modal) = app.log_modal {
        let contact_name = app.contacts[modal.contact_idx].contact.name.clone();
        widgets::log_modal::draw_log_modal(frame, modal, &contact_name);
    }

    // Overlay create-contact modal if active
    if let Some(ref modal) = app.create_contact_modal {
        widgets::create_contact_modal::draw_create_contact_modal(frame, modal);
    }

    // Overlay edit-contact modal if active
    if let Some(ref modal) = app.edit_contact_modal {
        let contact_name = app.contacts[modal.contact_idx].contact.name.clone();
        widgets::edit_contact_modal::draw_edit_contact_modal(frame, modal, &contact_name);
    }

    // Overlay delete-confirmation modal if active
    if let Some(idx) = app.delete_confirm {
        let contact_name = app.contacts[idx].contact.name.clone();
        widgets::confirm_delete_modal::draw_confirm_delete_modal(frame, &contact_name);
    }

    // Show status message if present
    if let Some(ref msg) = app.status_message {
        let area = frame.area();
        let status_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area)[1];
        let status = Paragraph::new(msg.as_str())
            .style(Style::default().fg(Color::Green))
            .alignment(Alignment::Center);
        frame.render_widget(status, status_area);
    }
}
