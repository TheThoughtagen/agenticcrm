use std::path::PathBuf;

use ratatui::widgets::TableState;

use crate::models::ContactFile;
use crate::ops;
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

#[derive(Debug, Clone, PartialEq)]
pub enum LogField {
    Type,
    Summary,
}

pub struct LogModalState {
    pub contact_idx: usize,
    pub interaction_type: String,
    pub summary: String,
    pub active_field: LogField,
    pub type_options: Vec<&'static str>,
    pub type_index: usize,
}

impl LogModalState {
    pub fn new(contact_idx: usize) -> Self {
        let type_options = vec![
            "coffee", "call", "email", "message", "meeting", "lunch", "intro",
        ];
        Self {
            contact_idx,
            interaction_type: type_options[0].to_string(),
            summary: String::new(),
            active_field: LogField::Type,
            type_options,
            type_index: 0,
        }
    }
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
    ToggleLogField,
    LogTypeNext,
    LogTypePrev,
    LogSummaryInput(char),
    LogSummaryBackspace,
}

pub struct App {
    pub contacts: Vec<ContactFile>,
    pub filtered: Vec<usize>,
    pub screen: Screen,
    pub input_mode: InputMode,
    pub search_query: String,
    pub table_state: TableState,
    pub dashboard_state: TableState,
    pub log_modal: Option<LogModalState>,
    pub status_message: Option<String>,
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
            log_modal: None,
            status_message: None,
            running: true,
            crm_root,
        })
    }

    pub fn update(&mut self, msg: Message) {
        // Clear status message on any keypress
        self.status_message = None;

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
                if let Some(selected) = self.table_state.selected() {
                    if selected < self.filtered.len() {
                        let contact_idx = self.filtered[selected];
                        self.log_modal = Some(LogModalState::new(contact_idx));
                        self.input_mode = InputMode::LogInteraction;
                    }
                }
            }
            Message::SubmitLog {
                interaction_type,
                summary,
            } => {
                if let Some(ref modal) = self.log_modal {
                    let name = self.contacts[modal.contact_idx].contact.name.clone();
                    match self.submit_log(&interaction_type, &summary, modal.contact_idx) {
                        Ok(()) => {
                            self.status_message =
                                Some(format!("Logged {} with {}", interaction_type, name));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
                self.log_modal = None;
                self.input_mode = InputMode::Normal;
            }
            Message::CancelModal => {
                self.log_modal = None;
                self.input_mode = InputMode::Normal;
            }
            Message::ToggleLogField => {
                if let Some(ref mut modal) = self.log_modal {
                    modal.active_field = match modal.active_field {
                        LogField::Type => LogField::Summary,
                        LogField::Summary => LogField::Type,
                    };
                }
            }
            Message::LogTypeNext => {
                if let Some(ref mut modal) = self.log_modal {
                    modal.type_index = (modal.type_index + 1) % modal.type_options.len();
                    modal.interaction_type = modal.type_options[modal.type_index].to_string();
                }
            }
            Message::LogTypePrev => {
                if let Some(ref mut modal) = self.log_modal {
                    if modal.type_index == 0 {
                        modal.type_index = modal.type_options.len() - 1;
                    } else {
                        modal.type_index -= 1;
                    }
                    modal.interaction_type = modal.type_options[modal.type_index].to_string();
                }
            }
            Message::LogSummaryInput(c) => {
                if let Some(ref mut modal) = self.log_modal {
                    modal.summary.push(c);
                }
            }
            Message::LogSummaryBackspace => {
                if let Some(ref mut modal) = self.log_modal {
                    modal.summary.pop();
                }
            }
        }
    }

    /// Log an interaction without printing to stdout (TUI-safe).
    /// Delegates to ops::contact::log_interaction for all file manipulation.
    fn submit_log(
        &mut self,
        interaction_type: &str,
        summary: &str,
        contact_idx: usize,
    ) -> anyhow::Result<()> {
        let name = &self.contacts[contact_idx].contact.name;

        ops::contact::log_interaction(&self.crm_root, name, interaction_type, summary, None)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Reload contacts to pick up changes
        let contacts = store::load_all_contacts(&self.crm_root)?;
        self.contacts = contacts;
        self.filter_contacts();

        Ok(())
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
