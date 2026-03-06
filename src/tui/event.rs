use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::app::{App, InputMode, Message, Screen};

pub fn handle_key(app: &App, key: KeyEvent) -> Option<Message> {
    match app.input_mode {
        InputMode::Search => match key.code {
            KeyCode::Esc => Some(Message::SearchClear),
            KeyCode::Enter => Some(Message::SearchConfirm),
            KeyCode::Backspace => Some(Message::SearchBackspace),
            KeyCode::Char(c) => Some(Message::SearchInput(c)),
            _ => None,
        },
        InputMode::LogInteraction => match key.code {
            KeyCode::Esc => Some(Message::CancelModal),
            _ => None,
        },
        InputMode::Normal => match &app.screen {
            Screen::ContactList => match key.code {
                KeyCode::Char('q') => Some(Message::Quit),
                KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNext),
                KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPrev),
                KeyCode::Enter => Some(Message::Enter),
                KeyCode::Char('/') => Some(Message::StartSearch),
                KeyCode::Char('d') => Some(Message::SwitchToDashboard),
                KeyCode::Char('l') => Some(Message::StartLog),
                _ => None,
            },
            Screen::ContactDetail(_) => match key.code {
                KeyCode::Char('q') => Some(Message::Quit),
                KeyCode::Esc | KeyCode::Backspace => Some(Message::Back),
                KeyCode::Char('l') => Some(Message::StartLog),
                _ => None,
            },
            Screen::FollowUpDashboard => match key.code {
                KeyCode::Char('q') => Some(Message::Quit),
                KeyCode::Esc => Some(Message::SwitchToList),
                KeyCode::Char('j') | KeyCode::Down => Some(Message::SelectNext),
                KeyCode::Char('k') | KeyCode::Up => Some(Message::SelectPrev),
                _ => None,
            },
        },
    }
}
