use ratatui::style::{Color, Modifier, Style};

use crate::models::contact::{Priority, Status};

pub fn status_style(status: &Option<Status>) -> Style {
    match status {
        Some(Status::Active) => Style::default().fg(Color::Green),
        Some(Status::Dormant) => Style::default().fg(Color::Yellow),
        Some(Status::LostTouch) => Style::default().fg(Color::Red),
        Some(Status::Archived) => Style::default().fg(Color::DarkGray),
        None => Style::default(),
    }
}

pub fn priority_style(priority: &Option<Priority>) -> Style {
    match priority {
        Some(Priority::High) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        Some(Priority::Medium) => Style::default().fg(Color::Yellow),
        Some(Priority::Low) => Style::default().fg(Color::DarkGray),
        None => Style::default(),
    }
}

pub fn format_status(status: &Option<Status>) -> &'static str {
    match status {
        Some(Status::Active) => "Active",
        Some(Status::Dormant) => "Dormant",
        Some(Status::LostTouch) => "Lost Touch",
        Some(Status::Archived) => "Archived",
        None => "",
    }
}

pub fn format_priority(priority: &Option<Priority>) -> &'static str {
    match priority {
        Some(Priority::High) => "!!!",
        Some(Priority::Medium) => "!!",
        Some(Priority::Low) => "!",
        None => "",
    }
}
