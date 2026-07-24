use ratatui::crossterm::event::{KeyCode, KeyEvent};

use super::app::{App, CreateContactField, InputMode, LogField, Message, Screen, VimMode};

fn edit_contact_text_key(key: KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Backspace => Some(Message::EditContactBackspace),
        KeyCode::Char(c) => Some(Message::EditContactInput(c)),
        _ => None,
    }
}

pub fn handle_key(app: &App, key: KeyEvent) -> Option<Message> {
    match app.input_mode {
        InputMode::Search => match key.code {
            KeyCode::Esc => Some(Message::SearchClear),
            KeyCode::Enter => Some(Message::SearchConfirm),
            KeyCode::Backspace => Some(Message::SearchBackspace),
            KeyCode::Char(c) => Some(Message::SearchInput(c)),
            _ => None,
        },
        InputMode::CreateContact => {
            if let Some(ref modal) = app.create_contact_modal {
                match key.code {
                    KeyCode::Esc => Some(Message::CancelModal),
                    KeyCode::Tab | KeyCode::BackTab => Some(Message::ToggleCreateContactField),
                    KeyCode::Enter => Some(Message::SubmitCreateContact),
                    _ => match modal.active_field {
                        CreateContactField::Name | CreateContactField::Company => match key.code {
                            KeyCode::Backspace => Some(Message::CreateContactBackspace),
                            KeyCode::Char(c) => Some(Message::CreateContactInput(c)),
                            _ => None,
                        },
                        CreateContactField::Relationship => match key.code {
                            KeyCode::Right | KeyCode::Char('j') => {
                                Some(Message::CreateContactRelationshipNext)
                            }
                            KeyCode::Left | KeyCode::Char('k') => {
                                Some(Message::CreateContactRelationshipPrev)
                            }
                            _ => None,
                        },
                        CreateContactField::Status => match key.code {
                            KeyCode::Right | KeyCode::Char('j') => {
                                Some(Message::CreateContactStatusNext)
                            }
                            KeyCode::Left | KeyCode::Char('k') => {
                                Some(Message::CreateContactStatusPrev)
                            }
                            _ => None,
                        },
                        CreateContactField::Priority => match key.code {
                            KeyCode::Right | KeyCode::Char('j') => {
                                Some(Message::CreateContactPriorityNext)
                            }
                            KeyCode::Left | KeyCode::Char('k') => {
                                Some(Message::CreateContactPriorityPrev)
                            }
                            _ => None,
                        },
                    },
                }
            } else {
                Some(Message::CancelModal)
            }
        }
        InputMode::LogInteraction => {
            if let Some(ref modal) = app.log_modal {
                match modal.active_field {
                    LogField::Type => match key.code {
                        KeyCode::Esc => Some(Message::CancelModal),
                        KeyCode::Tab | KeyCode::BackTab => Some(Message::ToggleLogField),
                        KeyCode::Enter => {
                            if !modal.summary.is_empty() {
                                Some(Message::SubmitLog {
                                    interaction_type: modal.interaction_type.clone(),
                                    summary: modal.summary.lines().join("\n"),
                                })
                            } else {
                                None
                            }
                        }
                        KeyCode::Right | KeyCode::Char('j') | KeyCode::Char('l') => {
                            Some(Message::LogTypeNext)
                        }
                        KeyCode::Left | KeyCode::Char('k') | KeyCode::Char('h') => {
                            Some(Message::LogTypePrev)
                        }
                        _ => None,
                    },
                    LogField::Summary => match modal.summary_vim_mode {
                        VimMode::Insert => match key.code {
                            KeyCode::Tab | KeyCode::BackTab => Some(Message::ToggleLogField),
                            _ => Some(Message::SummaryKey(key)),
                        },
                        VimMode::Normal => match key.code {
                            KeyCode::Esc => Some(Message::CancelModal),
                            KeyCode::Tab | KeyCode::BackTab => Some(Message::ToggleLogField),
                            _ => Some(Message::SummaryKey(key)),
                        },
                    },
                }
            } else {
                Some(Message::CancelModal)
            }
        }
        InputMode::EditContact => {
            if app.edit_contact_modal.is_some() {
                match key.code {
                    KeyCode::Esc => Some(Message::CancelModal),
                    KeyCode::Tab | KeyCode::BackTab => Some(Message::ToggleEditContactField),
                    KeyCode::Enter => Some(Message::SubmitEditContact),
                    _ => edit_contact_text_key(key),
                }
            } else {
                Some(Message::CancelModal)
            }
        }
        InputMode::ConfirmDelete => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(Message::ConfirmDeleteYes),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(Message::CancelModal),
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
                KeyCode::Char('n') => Some(Message::StartCreateContact),
                KeyCode::Char('s') => Some(Message::CycleStatusFilter),
                KeyCode::Char('p') => Some(Message::CyclePriorityFilter),
                KeyCode::Char('r') => Some(Message::CycleRelationshipFilter),
                KeyCode::Char('t') => Some(Message::ToggleSortMode),
                KeyCode::Char('x') => Some(Message::StartDeleteConfirm),
                _ => None,
            },
            Screen::ContactDetail(_) => match key.code {
                KeyCode::Char('q') => Some(Message::Quit),
                KeyCode::Esc | KeyCode::Backspace => Some(Message::Back),
                KeyCode::Char('l') => Some(Message::StartLog),
                KeyCode::Char('/') => Some(Message::BackToSearch),
                KeyCode::Char('s') => Some(Message::CycleContactStatus),
                KeyCode::Char('p') => Some(Message::CycleContactPriority),
                KeyCode::Char('r') => Some(Message::CycleContactRelationship),
                KeyCode::Char('e') => Some(Message::StartEditContact),
                KeyCode::Char('x') => Some(Message::StartDeleteConfirm),
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
