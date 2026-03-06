use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::widgets::Paragraph;

use super::app::{App, Screen};
use super::views;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::ContactList => views::contact_list::draw_contact_list(frame, app),
        Screen::ContactDetail(idx) => views::contact_detail::draw_contact_detail(frame, app, idx),
        Screen::FollowUpDashboard => {
            let placeholder =
                Paragraph::new("Follow-Up Dashboard - Coming soon").alignment(Alignment::Center);
            frame.render_widget(placeholder, frame.area());
        }
    }
}
