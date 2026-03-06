use ratatui::Frame;

use super::app::{App, Screen};
use super::views;

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::ContactList => views::contact_list::draw_contact_list(frame, app),
        Screen::ContactDetail(idx) => views::contact_detail::draw_contact_detail(frame, app, idx),
        Screen::FollowUpDashboard => views::follow_up::draw_follow_up_dashboard(frame, app),
    }
}
