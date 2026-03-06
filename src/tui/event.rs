use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::app::{App, InputMode, LogField, Message, Screen};

pub fn handle_key(app: &App, key: KeyEvent) -> Option<Message> {
    match app.input_mode {
        InputMode::Search => match key.code {
            KeyCode::Esc => Some(Message::SearchClear),
            KeyCode::Enter => Some(Message::SearchConfirm),
            KeyCode::Backspace => Some(Message::SearchBackspace),
            KeyCode::Char(c) => Some(Message::SearchInput(c)),
            _ => None,
        },
        InputMode::LogInteraction => {
            if let Some(ref modal) = app.log_modal {
                match key.code {
                    KeyCode::Esc => Some(Message::CancelModal),
                    KeyCode::Tab | KeyCode::BackTab => Some(Message::ToggleLogField),
                    KeyCode::Enter => {
                        if !modal.summary.is_empty() {
                            Some(Message::SubmitLog {
                                interaction_type: modal.interaction_type.clone(),
                                summary: modal.summary.clone(),
                            })
                        } else {
                            None
                        }
                    }
                    _ => match modal.active_field {
                        LogField::Type => match key.code {
                            KeyCode::Right | KeyCode::Char('j') => Some(Message::LogTypeNext),
                            KeyCode::Left | KeyCode::Char('k') => Some(Message::LogTypePrev),
                            _ => None,
                        },
                        LogField::Summary => match key.code {
                            KeyCode::Backspace => Some(Message::LogSummaryBackspace),
                            KeyCode::Char(c) => Some(Message::LogSummaryInput(c)),
                            _ => None,
                        },
                    },
                }
            } else {
                Some(Message::CancelModal)
            }
        }
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
