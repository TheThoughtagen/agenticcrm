use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::widgets::Paragraph;

use super::app::{App, Screen};
use super::views;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match &app.screen {
        Screen::ContactList => views::contact_list::draw_contact_list(frame, app),
        Screen::ContactDetail(_) => {
            let placeholder = Paragraph::new("Contact Detail - Coming soon")
                .alignment(Alignment::Center);
            frame.render_widget(placeholder, frame.area());
        }
        Screen::FollowUpDashboard => {
            let placeholder =
                Paragraph::new("Follow-Up Dashboard - Coming soon").alignment(Alignment::Center);
            frame.render_widget(placeholder, frame.area());
        }
    }
}
