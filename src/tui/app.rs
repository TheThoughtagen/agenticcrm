use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use tui_textarea::{CursorMove, TextArea};

use crate::models::contact::{Priority, Relationship, Status};
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
    CreateContact,
    EditContact,
    ConfirmDelete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogField {
    Type,
    Summary,
}

/// Vim-lite editing mode for the multi-line Summary field. Starts in `Insert` so
/// plain typing works with no vim knowledge required; `Esc` drops to `Normal` for
/// cursor motions, `i`/`a`/`I`/`A`/`o`/`O` return to `Insert`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VimMode {
    Insert,
    Normal,
}

/// Lines moved by `Ctrl+u`/`Ctrl+d` in the Summary textarea's vim-lite Normal
/// mode. The modal doesn't know its own rendered viewport height at the state
/// layer (that's only known at draw time), so this is a fixed approximation
/// of "half a page" rather than computed from the actual visible area.
const HALF_PAGE_LINES: usize = 10;

pub struct LogModalState {
    pub contact_idx: usize,
    pub interaction_type: String,
    pub summary: TextArea<'static>,
    pub summary_vim_mode: VimMode,
    /// Holds a pending first key of a two-key Normal-mode command (`d`/`dd` delete
    /// line, `y`/`yy` yank line, `g`/`gg` go to top, `Z`/`ZZ` submit). Cleared after
    /// the next keystroke either completes or cancels the command.
    pub summary_pending_op: Option<char>,
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
            summary: TextArea::default(),
            summary_vim_mode: VimMode::Insert,
            summary_pending_op: None,
            active_field: LogField::Type,
            type_options,
            type_index: 0,
        }
    }

    /// Interpret one keystroke against the Summary textarea, honoring the current
    /// vim-lite mode. Called for every key routed to the Summary field. Returns
    /// `true` when the keystroke completed the `ZZ` submit gesture — the caller
    /// is responsible for actually submitting (this method only edits text).
    pub fn handle_summary_key(&mut self, key: KeyEvent) -> bool {
        match self.summary_vim_mode {
            VimMode::Insert => {
                self.handle_summary_insert_key(key);
                false
            }
            VimMode::Normal => self.handle_summary_normal_key(key),
        }
    }

    fn handle_summary_insert_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.summary_vim_mode = VimMode::Normal,
            KeyCode::Enter => self.summary.insert_newline(),
            KeyCode::Backspace => {
                self.summary.delete_char();
            }
            KeyCode::Delete => {
                self.summary.delete_next_char();
            }
            KeyCode::Left => self.summary.move_cursor(CursorMove::Back),
            KeyCode::Right => self.summary.move_cursor(CursorMove::Forward),
            KeyCode::Up => self.summary.move_cursor(CursorMove::Up),
            KeyCode::Down => self.summary.move_cursor(CursorMove::Down),
            KeyCode::Home => self.summary.move_cursor(CursorMove::Head),
            KeyCode::End => self.summary.move_cursor(CursorMove::End),
            KeyCode::Char(c) => self.summary.insert_char(c),
            _ => {}
        }
    }

    /// Returns `true` only when `ZZ` completes — that's the one Normal-mode
    /// command the caller must act on (submit the log); everything else here
    /// only mutates the textarea.
    fn handle_summary_normal_key(&mut self, key: KeyEvent) -> bool {
        if let Some(pending) = self.summary_pending_op.take() {
            match (pending, key.code) {
                ('d', KeyCode::Char('d')) => {
                    // dd: delete the current line, including its trailing newline
                    // (if there is a following line to merge with).
                    self.summary.move_cursor(CursorMove::Head);
                    self.summary.delete_line_by_end();
                    self.summary.delete_next_char();
                }
                ('y', KeyCode::Char('y')) => self.yank_current_line(),
                ('g', KeyCode::Char('g')) => self.summary.move_cursor(CursorMove::Top),
                ('Z', KeyCode::Char('Z')) => return true,
                _ => {} // unrecognized second key: drop the pending command
            }
            return false;
        }

        match key.code {
            KeyCode::Char('i') => self.summary_vim_mode = VimMode::Insert,
            KeyCode::Char('a') => {
                self.summary.move_cursor(CursorMove::Forward);
                self.summary_vim_mode = VimMode::Insert;
            }
            KeyCode::Char('I') => {
                self.summary.move_cursor(CursorMove::Head);
                self.summary_vim_mode = VimMode::Insert;
            }
            KeyCode::Char('A') => {
                self.summary.move_cursor(CursorMove::End);
                self.summary_vim_mode = VimMode::Insert;
            }
            KeyCode::Char('o') => {
                self.summary.move_cursor(CursorMove::End);
                self.summary.insert_newline();
                self.summary_vim_mode = VimMode::Insert;
            }
            KeyCode::Char('O') => {
                self.summary.move_cursor(CursorMove::Head);
                self.summary.insert_newline();
                self.summary.move_cursor(CursorMove::Up);
                self.summary_vim_mode = VimMode::Insert;
            }
            KeyCode::Char('h') | KeyCode::Left => self.summary.move_cursor(CursorMove::Back),
            KeyCode::Char('l') | KeyCode::Right => self.summary.move_cursor(CursorMove::Forward),
            KeyCode::Char('j') | KeyCode::Down => self.summary.move_cursor(CursorMove::Down),
            KeyCode::Char('k') | KeyCode::Up => self.summary.move_cursor(CursorMove::Up),
            KeyCode::Char('0') | KeyCode::Char('^') => self.summary.move_cursor(CursorMove::Head),
            KeyCode::Char('$') => self.summary.move_cursor(CursorMove::End),
            KeyCode::Char('w') => self.summary.move_cursor(CursorMove::WordForward),
            KeyCode::Char('b') => self.summary.move_cursor(CursorMove::WordBack),
            KeyCode::Char('e') => self.summary.move_cursor(CursorMove::WordEnd),
            KeyCode::Char('G') => self.summary.move_cursor(CursorMove::Bottom),
            KeyCode::Char('x') => {
                self.summary.delete_next_char();
            }
            KeyCode::Char('D') => {
                self.summary.delete_line_by_end();
            }
            KeyCode::Char('p') => {
                // Paste-after: vim pastes a charwise yank just past the cursor.
                self.summary.move_cursor(CursorMove::Forward);
                self.summary.paste();
            }
            KeyCode::Char('P') => {
                self.summary.paste();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                for _ in 0..HALF_PAGE_LINES {
                    self.summary.move_cursor(CursorMove::Up);
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                for _ in 0..HALF_PAGE_LINES {
                    self.summary.move_cursor(CursorMove::Down);
                }
            }
            KeyCode::Char('u') => {
                self.summary.undo();
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.summary.redo();
            }
            KeyCode::Char('d') => self.summary_pending_op = Some('d'),
            KeyCode::Char('y') => self.summary_pending_op = Some('y'),
            KeyCode::Char('g') => self.summary_pending_op = Some('g'),
            KeyCode::Char('Z') => self.summary_pending_op = Some('Z'),
            _ => {}
        }
        false
    }

    /// Yank the current line into the textarea's yank register, for `p`/`P` to
    /// paste back. Includes the trailing newline when there's a following line
    /// to select through; on the last line, yanks just that line's text.
    fn yank_current_line(&mut self) {
        let (row, _) = self.summary.cursor();
        let is_last_line = row + 1 >= self.summary.lines().len();

        self.summary.move_cursor(CursorMove::Head);
        self.summary.start_selection();
        if is_last_line {
            self.summary.move_cursor(CursorMove::End);
        } else {
            self.summary.move_cursor(CursorMove::Down);
            self.summary.move_cursor(CursorMove::Head);
        }
        self.summary.copy();
        self.summary.cancel_selection();
    }
}

// ── Create Contact modal ────────────────────────────────────────────────────

/// Option labels for the Relationship/Status/Priority pickers, both in the
/// create-contact modal (where index 0 means "leave unset") and as the
/// contact-list quick filters (where index 0 means "show all"). Values after
/// index 0 are the exact serde wire values `ops::contact::edit`'s `key=value`
/// sets expect, so they can be passed straight through.
pub const RELATIONSHIP_OPTIONS: [&str; 9] = [
    "(unset)",
    "friend",
    "colleague",
    "client",
    "mentor",
    "mentee",
    "acquaintance",
    "family",
    "other",
];
pub const STATUS_OPTIONS: [&str; 5] = ["(unset)", "active", "dormant", "lost-touch", "archived"];
pub const PRIORITY_OPTIONS: [&str; 4] = ["(unset)", "high", "medium", "low"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CreateContactField {
    Name,
    Company,
    Relationship,
    Status,
    Priority,
}

pub struct CreateContactState {
    pub name: String,
    pub company: String,
    pub relationship_index: usize,
    pub status_index: usize,
    pub priority_index: usize,
    pub active_field: CreateContactField,
}

impl CreateContactState {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            company: String::new(),
            relationship_index: 0,
            status_index: 0,
            priority_index: 0,
            active_field: CreateContactField::Name,
        }
    }

    /// The `--set key=value` strings to apply after `ops::contact::add`, for
    /// whichever fields were moved off "(unset)" (or, for Company, non-empty).
    fn edit_sets(&self) -> Vec<String> {
        let mut sets = Vec::new();
        let company = self.company.trim();
        if !company.is_empty() {
            sets.push(format!("company={}", company));
        }
        if self.relationship_index != 0 {
            sets.push(format!(
                "relationship={}",
                RELATIONSHIP_OPTIONS[self.relationship_index]
            ));
        }
        if self.status_index != 0 {
            sets.push(format!("status={}", STATUS_OPTIONS[self.status_index]));
        }
        if self.priority_index != 0 {
            sets.push(format!("priority={}", PRIORITY_OPTIONS[self.priority_index]));
        }
        sets
    }
}

// ── Edit Contact modal (Company/Email/Phone/Birthday) ──────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditContactField {
    Company,
    Email,
    Phone,
    Birthday,
}

pub struct EditContactState {
    pub contact_idx: usize,
    pub company: String,
    /// Comma-separated, matching `acrm edit --set email=a@b.com,c@d.com`.
    pub email: String,
    /// Comma-separated, same convention as `email`.
    pub phone: String,
    /// `YYYY-MM-DD`, or empty if unset.
    pub birthday: String,
    pub active_field: EditContactField,
}

impl EditContactState {
    /// Pre-populate every field from the contact's current values so leaving a
    /// field untouched round-trips it unchanged.
    pub fn new(cf: &ContactFile, contact_idx: usize) -> Self {
        let c = &cf.contact;
        Self {
            contact_idx,
            company: c.company.clone(),
            email: c.email.join(", "),
            phone: c.phone.join(", "),
            birthday: c
                .birthday
                .map(|d| d.to_string())
                .unwrap_or_default(),
            active_field: EditContactField::Company,
        }
    }

    fn active_field_mut(&mut self) -> &mut String {
        match self.active_field {
            EditContactField::Company => &mut self.company,
            EditContactField::Email => &mut self.email,
            EditContactField::Phone => &mut self.phone,
            EditContactField::Birthday => &mut self.birthday,
        }
    }

    /// The `--set key=value` strings to apply. Only non-empty fields are
    /// included — clearing a field to blank isn't supported here, matching
    /// how the create-contact modal treats untouched optional fields (an
    /// empty `birthday` would fail to parse as a date on write anyway).
    fn edit_sets(&self) -> Vec<String> {
        let mut sets = Vec::new();
        let company = self.company.trim();
        if !company.is_empty() {
            sets.push(format!("company={}", company));
        }
        let email = self.email.trim();
        if !email.is_empty() {
            sets.push(format!("email={}", email));
        }
        let phone = self.phone.trim();
        if !phone.is_empty() {
            sets.push(format!("phone={}", phone));
        }
        let birthday = self.birthday.trim();
        if !birthday.is_empty() {
            sets.push(format!("birthday={}", birthday));
        }
        sets
    }
}

/// How the contact list is ordered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    /// Whatever order `store::load_all_contacts` returns (file order).
    Default,
    /// Alphabetical by name (case-insensitive).
    Name,
    /// Alphabetical by company (case-insensitive); contacts with no company
    /// sort last.
    Company,
    /// High, then Medium, then Low; contacts with no priority sort last.
    Priority,
    /// Active, then Dormant, then Lost Touch, then Archived; contacts with no
    /// status sort last.
    Status,
    /// By `Relationship`'s declaration order (Friend, Colleague, Client, ...);
    /// contacts with no relationship sort last.
    Relationship,
    /// Most recently contacted first; contacts never logged sort last.
    LastContacted,
}

impl SortMode {
    /// The order `t` cycles through.
    fn next(self) -> Self {
        match self {
            SortMode::Default => SortMode::Name,
            SortMode::Name => SortMode::Company,
            SortMode::Company => SortMode::Priority,
            SortMode::Priority => SortMode::Status,
            SortMode::Status => SortMode::Relationship,
            SortMode::Relationship => SortMode::LastContacted,
            SortMode::LastContacted => SortMode::Default,
        }
    }

    /// Short label shown in the contact list's column header / title.
    pub fn label(self) -> Option<&'static str> {
        match self {
            SortMode::Default => None,
            SortMode::Name => Some("Name"),
            SortMode::Company => Some("Company"),
            SortMode::Priority => Some("Pri"),
            SortMode::Status => Some("Status"),
            SortMode::Relationship => Some("Relationship"),
            SortMode::LastContacted => Some("Last Contacted"),
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
    BackToSearch,
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
    SummaryKey(KeyEvent),
    StartCreateContact,
    ToggleCreateContactField,
    CreateContactInput(char),
    CreateContactBackspace,
    CreateContactRelationshipNext,
    CreateContactRelationshipPrev,
    CreateContactStatusNext,
    CreateContactStatusPrev,
    CreateContactPriorityNext,
    CreateContactPriorityPrev,
    SubmitCreateContact,
    CycleStatusFilter,
    CyclePriorityFilter,
    CycleRelationshipFilter,
    CycleSortMode,
    CycleContactStatus,
    CycleContactPriority,
    CycleContactRelationship,
    StartDeleteConfirm,
    ConfirmDeleteYes,
    StartEditContact,
    ToggleEditContactField,
    EditContactInput(char),
    EditContactBackspace,
    SubmitEditContact,
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
    pub create_contact_modal: Option<CreateContactState>,
    pub edit_contact_modal: Option<EditContactState>,
    /// Index into `self.contacts` awaiting y/n delete confirmation.
    pub delete_confirm: Option<usize>,
    pub status_message: Option<String>,
    pub running: bool,
    pub crm_root: PathBuf,
    /// Index into `STATUS_OPTIONS`; 0 = no filter (show all).
    pub status_filter_index: usize,
    /// Index into `PRIORITY_OPTIONS`; 0 = no filter (show all).
    pub priority_filter_index: usize,
    /// Index into `RELATIONSHIP_OPTIONS`; 0 = no filter (show all).
    pub relationship_filter_index: usize,
    pub sort_mode: SortMode,
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
            create_contact_modal: None,
            edit_contact_modal: None,
            delete_confirm: None,
            status_message: None,
            running: true,
            crm_root,
            status_filter_index: 0,
            priority_filter_index: 0,
            relationship_filter_index: 0,
            sort_mode: SortMode::Default,
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
                // Preserve the existing query so re-entering search resumes where
                // you left off, instead of always starting from a blank field.
                self.input_mode = InputMode::Search;
            }
            Message::BackToSearch => {
                self.screen = Screen::ContactList;
                self.input_mode = InputMode::Search;
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
                self.create_contact_modal = None;
                self.edit_contact_modal = None;
                self.delete_confirm = None;
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
            Message::SummaryKey(key) => {
                let submit_requested = self
                    .log_modal
                    .as_mut()
                    .map(|modal| modal.handle_summary_key(key))
                    .unwrap_or(false);

                if submit_requested {
                    // `ZZ` completed: pull out what's needed, then submit exactly
                    // like SubmitLog does (can't call self.submit_log while
                    // log_modal is still borrowed).
                    if let Some(ref modal) = self.log_modal {
                        if !modal.summary.is_empty() {
                            let interaction_type = modal.interaction_type.clone();
                            let summary = modal.summary.lines().join("\n");
                            let contact_idx = modal.contact_idx;
                            let name = self.contacts[contact_idx].contact.name.clone();
                            match self.submit_log(&interaction_type, &summary, contact_idx) {
                                Ok(()) => {
                                    self.status_message = Some(format!(
                                        "Logged {} with {}",
                                        interaction_type, name
                                    ));
                                }
                                Err(e) => {
                                    self.status_message = Some(format!("Error: {}", e));
                                }
                            }
                        }
                    }
                    self.log_modal = None;
                    self.input_mode = InputMode::Normal;
                }
            }
            Message::StartCreateContact => {
                self.create_contact_modal = Some(CreateContactState::new());
                self.input_mode = InputMode::CreateContact;
            }
            Message::ToggleCreateContactField => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    modal.active_field = match modal.active_field {
                        CreateContactField::Name => CreateContactField::Company,
                        CreateContactField::Company => CreateContactField::Relationship,
                        CreateContactField::Relationship => CreateContactField::Status,
                        CreateContactField::Status => CreateContactField::Priority,
                        CreateContactField::Priority => CreateContactField::Name,
                    };
                }
            }
            Message::CreateContactInput(c) => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    match modal.active_field {
                        CreateContactField::Company => modal.company.push(c),
                        _ => modal.name.push(c),
                    }
                }
            }
            Message::CreateContactBackspace => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    match modal.active_field {
                        CreateContactField::Company => {
                            modal.company.pop();
                        }
                        _ => {
                            modal.name.pop();
                        }
                    }
                }
            }
            Message::CreateContactRelationshipNext => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    modal.relationship_index =
                        (modal.relationship_index + 1) % RELATIONSHIP_OPTIONS.len();
                }
            }
            Message::CreateContactRelationshipPrev => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    modal.relationship_index = if modal.relationship_index == 0 {
                        RELATIONSHIP_OPTIONS.len() - 1
                    } else {
                        modal.relationship_index - 1
                    };
                }
            }
            Message::CreateContactStatusNext => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    modal.status_index = (modal.status_index + 1) % STATUS_OPTIONS.len();
                }
            }
            Message::CreateContactStatusPrev => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    modal.status_index = if modal.status_index == 0 {
                        STATUS_OPTIONS.len() - 1
                    } else {
                        modal.status_index - 1
                    };
                }
            }
            Message::CreateContactPriorityNext => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    modal.priority_index = (modal.priority_index + 1) % PRIORITY_OPTIONS.len();
                }
            }
            Message::CreateContactPriorityPrev => {
                if let Some(ref mut modal) = self.create_contact_modal {
                    modal.priority_index = if modal.priority_index == 0 {
                        PRIORITY_OPTIONS.len() - 1
                    } else {
                        modal.priority_index - 1
                    };
                }
            }
            Message::SubmitCreateContact => {
                if let Some(ref modal) = self.create_contact_modal {
                    let name = modal.name.trim().to_string();
                    if !name.is_empty() {
                        let sets = modal.edit_sets();
                        match self.submit_create_contact(&name, &sets) {
                            Ok(()) => {
                                self.status_message = Some(format!("Created contact {}", name));
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Error: {}", e));
                            }
                        }
                    }
                }
                self.create_contact_modal = None;
                self.input_mode = InputMode::Normal;
            }
            Message::CycleStatusFilter => {
                self.status_filter_index = (self.status_filter_index + 1) % STATUS_OPTIONS.len();
                self.filter_contacts();
            }
            Message::CyclePriorityFilter => {
                self.priority_filter_index =
                    (self.priority_filter_index + 1) % PRIORITY_OPTIONS.len();
                self.filter_contacts();
            }
            Message::CycleRelationshipFilter => {
                self.relationship_filter_index =
                    (self.relationship_filter_index + 1) % RELATIONSHIP_OPTIONS.len();
                self.filter_contacts();
            }
            Message::CycleSortMode => {
                self.sort_mode = self.sort_mode.next();
                self.filter_contacts();
            }
            Message::CycleContactStatus => {
                if let Screen::ContactDetail(idx) = self.screen {
                    let current = self.contacts[idx].contact.status;
                    let next = next_status(current);
                    self.apply_field_edit(idx, "status", status_wire_value(next));
                }
            }
            Message::CycleContactPriority => {
                if let Screen::ContactDetail(idx) = self.screen {
                    let current = self.contacts[idx].contact.priority;
                    let next = next_priority(current);
                    self.apply_field_edit(idx, "priority", priority_wire_value(next));
                }
            }
            Message::CycleContactRelationship => {
                if let Screen::ContactDetail(idx) = self.screen {
                    let current = self.contacts[idx].contact.relationship;
                    let next = next_relationship(current);
                    self.apply_field_edit(idx, "relationship", relationship_wire_value(next));
                }
            }
            Message::StartDeleteConfirm => {
                let idx = match self.screen {
                    Screen::ContactDetail(idx) => Some(idx),
                    Screen::ContactList => self
                        .table_state
                        .selected()
                        .and_then(|i| self.filtered.get(i))
                        .copied(),
                    Screen::FollowUpDashboard => None,
                };
                if let Some(idx) = idx {
                    self.delete_confirm = Some(idx);
                    self.input_mode = InputMode::ConfirmDelete;
                }
            }
            Message::ConfirmDeleteYes => {
                if let Some(idx) = self.delete_confirm {
                    let name = self.contacts[idx].contact.name.clone();
                    match self.perform_delete(&name) {
                        Ok(()) => {
                            self.status_message = Some(format!("Deleted {}", name));
                            // The contact just vanished from self.contacts; if we were
                            // looking at its detail pane, there's nothing left to show.
                            if self.screen == Screen::ContactDetail(idx) {
                                self.screen = Screen::ContactList;
                            }
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
                self.delete_confirm = None;
                self.input_mode = InputMode::Normal;
            }
            Message::StartEditContact => {
                if let Screen::ContactDetail(idx) = self.screen {
                    if idx < self.contacts.len() {
                        self.edit_contact_modal =
                            Some(EditContactState::new(&self.contacts[idx], idx));
                        self.input_mode = InputMode::EditContact;
                    }
                }
            }
            Message::ToggleEditContactField => {
                if let Some(ref mut modal) = self.edit_contact_modal {
                    modal.active_field = match modal.active_field {
                        EditContactField::Company => EditContactField::Email,
                        EditContactField::Email => EditContactField::Phone,
                        EditContactField::Phone => EditContactField::Birthday,
                        EditContactField::Birthday => EditContactField::Company,
                    };
                }
            }
            Message::EditContactInput(c) => {
                if let Some(ref mut modal) = self.edit_contact_modal {
                    modal.active_field_mut().push(c);
                }
            }
            Message::EditContactBackspace => {
                if let Some(ref mut modal) = self.edit_contact_modal {
                    modal.active_field_mut().pop();
                }
            }
            Message::SubmitEditContact => {
                if let Some(ref modal) = self.edit_contact_modal {
                    let sets = modal.edit_sets();
                    let contact_idx = modal.contact_idx;
                    if !sets.is_empty() {
                        match self.apply_edit_sets(contact_idx, &sets) {
                            Ok(name) => {
                                self.status_message = Some(format!("Updated {}", name));
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Error: {}", e));
                            }
                        }
                    }
                }
                self.edit_contact_modal = None;
                self.input_mode = InputMode::Normal;
            }
        }
    }

    /// Reload contacts from disk after a mutation, then re-select the same
    /// contact by its stable `id` rather than its position in the reloaded
    /// list. `store::load_all_contacts` walks the contacts directory via
    /// `WalkDir` with no sort, so its order is not guaranteed stable across
    /// calls -- a raw index captured before a reload can silently point at a
    /// different contact afterward. Also re-points `Screen::ContactDetail`
    /// at the same contact's new index, so editing-then-saving in the detail
    /// pane doesn't leave you looking at (or next editing) the wrong person.
    fn reload_contacts(&mut self, keep_contact_id: Option<&str>) -> anyhow::Result<()> {
        self.contacts = store::load_all_contacts(&self.crm_root)?;
        self.filter_contacts();

        if let Some(id) = keep_contact_id {
            if let Some(new_idx) = self.contacts.iter().position(|cf| cf.contact.id == id) {
                if let Screen::ContactDetail(_) = self.screen {
                    self.screen = Screen::ContactDetail(new_idx);
                }
                if let Some(row) = self.filtered.iter().position(|&i| i == new_idx) {
                    self.table_state.select(Some(row));
                }
            }
        }

        Ok(())
    }

    /// Log an interaction without printing to stdout (TUI-safe).
    /// Delegates to ops::contact::log_interaction for all file manipulation.
    fn submit_log(
        &mut self,
        interaction_type: &str,
        summary: &str,
        contact_idx: usize,
    ) -> anyhow::Result<()> {
        let id = self.contacts[contact_idx].contact.id.clone();
        let name = &self.contacts[contact_idx].contact.name;

        ops::contact::log_interaction(&self.crm_root, name, interaction_type, summary, None)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        self.reload_contacts(Some(&id))?;

        Ok(())
    }

    /// Create a new contact, then apply any relationship/status/priority picked
    /// in the modal via a follow-up edit. Both delegate to `ops::contact` so the
    /// TUI never touches contact files directly.
    fn submit_create_contact(&mut self, name: &str, sets: &[String]) -> anyhow::Result<()> {
        ops::contact::add(&self.crm_root, name).map_err(|e| anyhow::anyhow!("{}", e))?;

        if !sets.is_empty() {
            ops::contact::edit(&self.crm_root, name, sets)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }

        self.reload_contacts(None)?;

        Ok(())
    }

    /// Apply multiple `field=value` edits at once to an existing contact (by
    /// index into `self.contacts`), reload, and return its name. Used by the
    /// Edit Contact modal (Company/Email/Phone/Birthday).
    fn apply_edit_sets(&mut self, contact_idx: usize, sets: &[String]) -> anyhow::Result<String> {
        let id = self.contacts[contact_idx].contact.id.clone();
        let name = self.contacts[contact_idx].contact.name.clone();
        ops::contact::edit(&self.crm_root, &name, sets).map_err(|e| anyhow::anyhow!("{}", e))?;

        self.reload_contacts(Some(&id))?;

        Ok(name)
    }

    /// Apply a single `field=value` edit to an existing contact (by index into
    /// `self.contacts`) and reload. Used by the Contact Detail pane's quick
    /// status/priority/relationship cycle keys.
    fn apply_field_edit(&mut self, contact_idx: usize, field: &str, value: &str) {
        let id = self.contacts[contact_idx].contact.id.clone();
        let name = self.contacts[contact_idx].contact.name.clone();
        let set = format!("{field}={value}");
        match ops::contact::edit(&self.crm_root, &name, &[set]).map_err(|e| anyhow::anyhow!("{}", e))
        {
            Ok(_) => match self.reload_contacts(Some(&id)) {
                Ok(()) => {
                    self.status_message = Some(format!("{} -> {}", field, value));
                }
                Err(e) => {
                    self.status_message = Some(format!("Error: {}", e));
                }
            },
            Err(e) => {
                self.status_message = Some(format!("Error: {}", e));
            }
        }
    }

    /// Delete a contact by name (TUI-safe: no interactive stdin prompt, unlike
    /// the CLI's `acrm delete`). Confirmation already happened via the
    /// ConfirmDelete modal before this is called.
    fn perform_delete(&mut self, name: &str) -> anyhow::Result<()> {
        ops::contact::confirm_delete(&self.crm_root, name)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // The deleted contact's id will never be found post-reload, so
        // filter_contacts()'s own fallback selection (nearest remaining row) applies.
        self.reload_contacts(None)?;

        Ok(())
    }

    pub fn filter_contacts(&mut self) {
        // Remember which actual contact was selected (not just its row number)
        // so cycling a filter/sort while browsing doesn't bounce the cursor back
        // to the top of the list if that contact still matches.
        let previously_selected = self
            .table_state
            .selected()
            .and_then(|i| self.filtered.get(i))
            .copied();

        let query = self.search_query.to_lowercase();
        let status_filter = if self.status_filter_index == 0 {
            None
        } else {
            status_from_wire(STATUS_OPTIONS[self.status_filter_index])
        };
        let priority_filter = if self.priority_filter_index == 0 {
            None
        } else {
            priority_from_wire(PRIORITY_OPTIONS[self.priority_filter_index])
        };
        let relationship_filter = if self.relationship_filter_index == 0 {
            None
        } else {
            relationship_from_wire(RELATIONSHIP_OPTIONS[self.relationship_filter_index])
        };

        self.filtered = self
            .contacts
            .iter()
            .enumerate()
            .filter(|(_, cf)| {
                query.is_empty()
                    || cf.contact.name.to_lowercase().contains(&query)
                    || cf.contact.company.to_lowercase().contains(&query)
                    || cf
                        .contact
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query))
            })
            .filter(|(_, cf)| status_filter.is_none() || cf.contact.status == status_filter)
            .filter(|(_, cf)| priority_filter.is_none() || cf.contact.priority == priority_filter)
            .filter(|(_, cf)| {
                relationship_filter.is_none() || cf.contact.relationship == relationship_filter
            })
            .map(|(i, _)| i)
            .collect();

        let contacts = &self.contacts;
        match self.sort_mode {
            SortMode::Default => {}
            SortMode::Name => self.filtered.sort_by(|&a, &b| {
                contacts[a]
                    .contact
                    .name
                    .to_lowercase()
                    .cmp(&contacts[b].contact.name.to_lowercase())
            }),
            SortMode::Company => self.filtered.sort_by(|&a, &b| {
                // Empty company sorts last regardless of case-insensitive
                // ordering, so uncategorized contacts don't clutter the top.
                let ca = &contacts[a].contact.company;
                let cb = &contacts[b].contact.company;
                match (ca.is_empty(), cb.is_empty()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Greater,
                    (false, true) => std::cmp::Ordering::Less,
                    (false, false) => ca.to_lowercase().cmp(&cb.to_lowercase()),
                }
            }),
            SortMode::Priority => self.filtered.sort_by(|&a, &b| {
                // None (no priority) sorts last, not first, unlike the
                // default Option<T>::None-sorts-first Ord behavior.
                sort_key(contacts[a].contact.priority).cmp(&sort_key(contacts[b].contact.priority))
            }),
            SortMode::Status => self.filtered.sort_by(|&a, &b| {
                sort_key(contacts[a].contact.status).cmp(&sort_key(contacts[b].contact.status))
            }),
            SortMode::Relationship => self.filtered.sort_by(|&a, &b| {
                sort_key(contacts[a].contact.relationship)
                    .cmp(&sort_key(contacts[b].contact.relationship))
            }),
            SortMode::LastContacted => self.filtered.sort_by(|&a, &b| {
                let da = contacts[a].contact.last_contacted;
                let db = contacts[b].contact.last_contacted;
                db.cmp(&da)
            }),
        }

        // Restore the same contact's selection if it still matches; only fall
        // back to the top row if it was filtered out (or nothing was selected).
        if self.filtered.is_empty() {
            self.table_state.select(None);
        } else {
            let restored = previously_selected
                .and_then(|prev_idx| self.filtered.iter().position(|&i| i == prev_idx));
            self.table_state.select(Some(restored.unwrap_or(0)));
        }
    }
}

/// Sort key that puts "no value set" last instead of first — the derived
/// `Ord` on `Option<T>` sorts `None` before `Some(_)`, which is backwards for
/// a contact list (uncategorized contacts shouldn't crowd the top).
fn sort_key<T: Ord>(value: Option<T>) -> (bool, Option<T>) {
    (value.is_none(), value)
}

// ── Status/Priority/Relationship cycling + wire-value helpers ──────────────
//
// These back the Contact Detail pane's quick-edit keys. Cycling from `None`
// always lands on a concrete value (there's no "unset" step) since the intent
// of pressing the key is "set/change this now", matching how a CRM field is
// expected to behave once you've started touching it.

fn next_status(current: Option<Status>) -> Status {
    match current {
        None | Some(Status::Archived) => Status::Active,
        Some(Status::Active) => Status::Dormant,
        Some(Status::Dormant) => Status::LostTouch,
        Some(Status::LostTouch) => Status::Archived,
    }
}

fn status_wire_value(s: Status) -> &'static str {
    match s {
        Status::Active => "active",
        Status::Dormant => "dormant",
        Status::LostTouch => "lost-touch",
        Status::Archived => "archived",
    }
}

fn status_from_wire(value: &str) -> Option<Status> {
    match value {
        "active" => Some(Status::Active),
        "dormant" => Some(Status::Dormant),
        "lost-touch" => Some(Status::LostTouch),
        "archived" => Some(Status::Archived),
        _ => None,
    }
}

fn next_priority(current: Option<Priority>) -> Priority {
    match current {
        None | Some(Priority::Low) => Priority::High,
        Some(Priority::High) => Priority::Medium,
        Some(Priority::Medium) => Priority::Low,
    }
}

fn priority_wire_value(p: Priority) -> &'static str {
    match p {
        Priority::High => "high",
        Priority::Medium => "medium",
        Priority::Low => "low",
    }
}

fn priority_from_wire(value: &str) -> Option<Priority> {
    match value {
        "high" => Some(Priority::High),
        "medium" => Some(Priority::Medium),
        "low" => Some(Priority::Low),
        _ => None,
    }
}

fn next_relationship(current: Option<Relationship>) -> Relationship {
    match current {
        None | Some(Relationship::Other) => Relationship::Friend,
        Some(Relationship::Friend) => Relationship::Colleague,
        Some(Relationship::Colleague) => Relationship::Client,
        Some(Relationship::Client) => Relationship::Mentor,
        Some(Relationship::Mentor) => Relationship::Mentee,
        Some(Relationship::Mentee) => Relationship::Acquaintance,
        Some(Relationship::Acquaintance) => Relationship::Family,
        Some(Relationship::Family) => Relationship::Other,
    }
}

fn relationship_wire_value(r: Relationship) -> &'static str {
    match r {
        Relationship::Friend => "friend",
        Relationship::Colleague => "colleague",
        Relationship::Client => "client",
        Relationship::Mentor => "mentor",
        Relationship::Mentee => "mentee",
        Relationship::Acquaintance => "acquaintance",
        Relationship::Family => "family",
        Relationship::Other => "other",
    }
}

fn relationship_from_wire(value: &str) -> Option<Relationship> {
    match value {
        "friend" => Some(Relationship::Friend),
        "colleague" => Some(Relationship::Colleague),
        "client" => Some(Relationship::Client),
        "mentor" => Some(Relationship::Mentor),
        "mentee" => Some(Relationship::Mentee),
        "acquaintance" => Some(Relationship::Acquaintance),
        "family" => Some(Relationship::Family),
        "other" => Some(Relationship::Other),
        _ => None,
    }
}

#[cfg(test)]
mod reload_contacts_tests {
    use super::*;

    /// Builds an isolated CRM root (own contacts/ + templates/) with three
    /// contacts, and an `App` pointed at it with the detail screen open on
    /// the second one.
    fn setup(tmp: &std::path::Path) -> App {
        std::fs::create_dir_all(tmp.join("contacts")).unwrap();
        std::fs::create_dir_all(tmp.join("templates")).unwrap();
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/templates/contact.md"),
            tmp.join("templates/contact.md"),
        )
        .unwrap();

        for name in ["Alice Anderson", "Bob Baker", "Carol Chen"] {
            ops::contact::add(tmp, name).unwrap();
        }

        let contacts = store::load_all_contacts(tmp).unwrap();
        let filtered: Vec<usize> = (0..contacts.len()).collect();
        let bob_idx = contacts
            .iter()
            .position(|cf| cf.contact.name == "Bob Baker")
            .unwrap();

        let mut table_state = TableState::default();
        table_state.select(Some(bob_idx));

        App {
            contacts,
            filtered,
            screen: Screen::ContactDetail(bob_idx),
            input_mode: InputMode::Normal,
            search_query: String::new(),
            table_state,
            dashboard_state: TableState::default(),
            log_modal: None,
            create_contact_modal: None,
            edit_contact_modal: None,
            delete_confirm: None,
            status_message: None,
            running: true,
            crm_root: tmp.to_path_buf(),
            status_filter_index: 0,
            priority_filter_index: 0,
            relationship_filter_index: 0,
            sort_mode: SortMode::Default,
        }
    }

    /// After editing a contact and reloading, both the detail screen and the
    /// list selection must still point at that same contact by identity --
    /// not by the position it happened to occupy before the reload, since
    /// `load_all_contacts` (`WalkDir`, unsorted) offers no ordering guarantee
    /// across calls.
    #[test]
    fn edit_keeps_same_contact_selected_after_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let mut app = setup(tmp.path());

        let bob_idx_before = match app.screen {
            Screen::ContactDetail(i) => i,
            _ => panic!("expected ContactDetail"),
        };
        let bob_id = app.contacts[bob_idx_before].contact.id.clone();

        app.apply_field_edit(bob_idx_before, "priority", "high");

        let bob_idx_after = match app.screen {
            Screen::ContactDetail(i) => i,
            _ => panic!("expected ContactDetail after edit"),
        };
        assert_eq!(
            app.contacts[bob_idx_after].contact.id, bob_id,
            "detail screen must still point at Bob after the reload, regardless of his new position"
        );
        assert_eq!(app.contacts[bob_idx_after].contact.name, "Bob Baker");
        assert_eq!(
            app.contacts[bob_idx_after].contact.priority,
            Some(Priority::High)
        );

        let selected_row = app.table_state.selected().unwrap();
        let selected_contacts_idx = app.filtered[selected_row];
        assert_eq!(
            app.contacts[selected_contacts_idx].contact.id, bob_id,
            "list selection must still highlight Bob after the reload"
        );
    }
}
