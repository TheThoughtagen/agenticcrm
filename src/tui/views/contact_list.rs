use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::Paragraph;

use crate::tui::app::App;

/// Placeholder - will be fully implemented in Task 2
pub fn draw_contact_list(frame: &mut Frame, _app: &mut App) {
    let area = frame.area();
    let placeholder = Paragraph::new("Contact list - building in task 2...");
    frame.render_widget(placeholder, area);
}
