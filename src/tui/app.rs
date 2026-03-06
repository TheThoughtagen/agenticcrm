use std::path::PathBuf;

use ratatui::widgets::TableState;

use crate::models::ContactFile;
use crate::store;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    ContactList,
    ContactDetail(usize),
    FollowUpDashboard,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
    LogInteraction,
}

#[derive(Debug, Clone)]
pub enum Message {
    Quit,
    SelectNext,
    SelectPrev,
    Enter,
    Back,
    StartSearch,
    SearchInput(char),
    SearchBackspace,
    SearchConfirm,
    SearchClear,
    SwitchToDashboard,
    SwitchToList,
    StartLog,
    SubmitLog {
        interaction_type: String,
        summary: String,
    },
    CancelModal,
}

pub struct App {
    pub contacts: Vec<ContactFile>,
    pub filtered: Vec<usize>,
    pub screen: Screen,
    pub input_mode: InputMode,
    pub search_query: String,
    pub table_state: TableState,
    pub dashboard_state: TableState,
    pub running: bool,
    pub crm_root: PathBuf,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let crm_root = store::find_crm_root()?;
        let contacts = store::load_all_contacts(&crm_root)?;
        let filtered: Vec<usize> = (0..contacts.len()).collect();

        let mut table_state = TableState::default();
        if !filtered.is_empty() {
            table_state.select(Some(0));
        }

        Ok(Self {
            contacts,
            filtered,
            screen: Screen::ContactList,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            table_state,
            dashboard_state: TableState::default(),
            running: true,
            crm_root,
        })
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Quit => self.running = false,
            Message::SelectNext => {
                if self.filtered.is_empty() {
                    return;
                }
                let i = match self.table_state.selected() {
                    Some(i) => {
                        if i >= self.filtered.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.table_state.select(Some(i));
            }
            Message::SelectPrev => {
                if self.filtered.is_empty() {
                    return;
                }
                let i = match self.table_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.filtered.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.table_state.select(Some(i));
            }
            Message::Enter => {
                if self.screen == Screen::ContactList {
                    if let Some(selected) = self.table_state.selected() {
                        if selected < self.filtered.len() {
                            self.screen = Screen::ContactDetail(self.filtered[selected]);
                        }
                    }
                }
            }
            Message::Back => {
                self.screen = Screen::ContactList;
            }
            Message::StartSearch => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
            }
            Message::SearchInput(c) => {
                self.search_query.push(c);
                self.filter_contacts();
            }
            Message::SearchBackspace => {
                self.search_query.pop();
                self.filter_contacts();
            }
            Message::SearchConfirm => {
                self.input_mode = InputMode::Normal;
            }
            Message::SearchClear => {
                self.search_query.clear();
                self.input_mode = InputMode::Normal;
                self.filter_contacts();
            }
            Message::SwitchToDashboard => {
                self.screen = Screen::FollowUpDashboard;
                self.dashboard_state.select(Some(0));
            }
            Message::SwitchToList => {
                self.screen = Screen::ContactList;
            }
            Message::StartLog => {
                // Stub: will be implemented in plan 03-03
            }
            Message::SubmitLog { .. } => {
                // Stub: will be implemented in plan 03-03
            }
            Message::CancelModal => {
                self.input_mode = InputMode::Normal;
            }
        }
    }

    pub fn filter_contacts(&mut self) {
        if self.search_query.is_empty() {
            self.filtered = (0..self.contacts.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered = self
                .contacts
                .iter()
                .enumerate()
                .filter(|(_, cf)| {
                    cf.contact.name.to_lowercase().contains(&query)
                        || cf.contact.company.to_lowercase().contains(&query)
                        || cf
                            .contact
                            .tags
                            .iter()
                            .any(|t| t.to_lowercase().contains(&query))
                })
                .map(|(i, _)| i)
                .collect();
        }
        // Reset selection
        if self.filtered.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }
}
