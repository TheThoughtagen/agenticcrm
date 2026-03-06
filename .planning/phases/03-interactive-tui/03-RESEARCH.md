# Phase 3: Interactive TUI - Research

**Researched:** 2026-03-06
**Domain:** Rust terminal user interface (TUI) with ratatui
**Confidence:** HIGH

## Summary

Phase 3 adds an interactive terminal UI to acrm, launched via `acrm tui`. The project is a Rust CLI application using clap, with ~985 contact markdown files parsed via existing `store::load_all_contacts()`. The TUI needs a scrollable contact table, split-pane detail view, real-time search filtering, a follow-up dashboard, and inline interaction logging.

Ratatui is the clear standard for Rust TUIs -- it is the maintained fork of tui-rs with 18.7K GitHub stars and 18.7M crate downloads. Version 0.30.0 (released 2025-12-26) introduced modularization and a simplified `ratatui::run()` API. It pairs with crossterm for terminal I/O (crossterm is the default backend, re-exported by ratatui). The Elm Architecture (TEA) pattern -- Model/Message/Update/View -- is the recommended application pattern for ratatui apps and maps perfectly to this use case.

**Primary recommendation:** Use ratatui 0.29 (stable, well-documented, avoids 0.30 breaking changes and MSRV 1.86 requirement) with crossterm (default backend). Structure the TUI using TEA pattern with separate view screens (ContactList, ContactDetail, FollowUpDashboard) and modal overlays for search and interaction logging.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TUI-01 | Scrollable table with name, company, status, last_contacted | ratatui Table widget with TableState for scrolling/selection; data from store::load_all_contacts() |
| TUI-02 | Split-pane contact detail view | Layout::default().direction(Horizontal) with Constraint::Percentage; Paragraph widget for detail rendering |
| TUI-03 | Keyboard nav with vim j/k and / search | TEA message pattern: KeyCode::Char('j'/'k') mapped to SelectNext/SelectPrev messages; '/' enters search mode |
| TUI-04 | Follow-up dashboard (overdue + upcoming) | Reuse due.rs logic (filter by next_follow_up vs today); separate Tab/view in the app model |
| TUI-05 | Real-time search/filter | App state holds search query string; filter contacts Vec on each keystroke; re-render filtered table |
| TUI-06 | Log interaction from TUI | Modal overlay with text input fields (type, summary); calls existing log.rs logic; refreshes contact data |
| TUI-07 | Color-coded priority/status indicators | ratatui Style with Color::Red/Yellow/Green for priority; Status enum maps to style via match |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.29 | TUI framework (widgets, layout, rendering) | De facto Rust TUI library; successor to tui-rs; massive ecosystem |
| crossterm | (via ratatui re-export) | Terminal backend (raw mode, events, colors) | Default ratatui backend; cross-platform; use ratatui::crossterm |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tui-textarea | latest | Multi-line text input widget | For notes field in interaction log modal |
| unicode-width | 0.2 | Accurate column width for Unicode names | If contact names include non-ASCII characters |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ratatui 0.29 | ratatui 0.30 | 0.30 has new run() API but requires MSRV 1.86, Rust 2024 edition, and has breaking changes to Block/Table APIs |
| crossterm | termion | termion is Unix-only; crossterm works on Windows too |
| tui-textarea | raw crossterm input | textarea handles cursor, scrolling, word wrap for free |

**Installation:**
```bash
cargo add ratatui@0.29
# crossterm is included via ratatui's default features -- use ratatui::crossterm
# Optional:
cargo add tui-textarea
```

**Important:** Do NOT add crossterm as a separate dependency. Use `ratatui::crossterm` re-export to avoid version conflicts.

## Architecture Patterns

### Recommended Project Structure
```
src/
  tui/
    mod.rs          # pub mod declarations, App struct, run() entry point
    app.rs          # App state (Model), Screen enum, message types
    event.rs        # Event polling, key-to-message mapping
    ui.rs           # Top-level view dispatcher
    views/
      mod.rs
      contact_list.rs    # Table view with search bar
      contact_detail.rs  # Split-pane detail view
      follow_up.rs       # Dashboard view
    widgets/
      mod.rs
      log_modal.rs       # Interaction logging overlay
      search_bar.rs      # Search input component
      status_badge.rs    # Color-coded status/priority indicators
  commands/
    tui.rs          # CLI subcommand handler that calls tui::run()
```

### Pattern 1: TEA (The Elm Architecture)
**What:** Separate app into Model (state), Message (events), Update (state transitions), View (rendering)
**When to use:** Always -- this is the standard ratatui pattern
**Example:**
```rust
// Source: https://ratatui.rs/concepts/application-patterns/the-elm-architecture/

enum Screen {
    ContactList,
    ContactDetail(usize),  // index into contacts
    FollowUpDashboard,
}

enum InputMode {
    Normal,
    Search,
    LogInteraction,
}

struct App {
    contacts: Vec<ContactFile>,
    filtered: Vec<usize>,      // indices into contacts
    screen: Screen,
    input_mode: InputMode,
    search_query: String,
    table_state: TableState,
    running: bool,
    crm_root: PathBuf,
}

enum Message {
    Quit,
    SelectNext,
    SelectPrev,
    Enter,
    Back,
    StartSearch,
    SearchInput(char),
    SearchBackspace,
    SearchConfirm,
    SwitchToDashboard,
    SwitchToList,
    StartLog,
    SubmitLog { interaction_type: String, summary: String },
}
```

### Pattern 2: Terminal Init/Restore
**What:** Enter raw mode before TUI, restore on exit (including panics)
**When to use:** Always at TUI entry/exit
**Example:**
```rust
use std::io;
use ratatui::crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn init_terminal() -> io::Result<ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    ratatui::Terminal::new(backend)
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
```

### Pattern 3: Split-Pane Layout for Detail View
**What:** Horizontal split with contact list on left, detail on right
**When to use:** TUI-02 contact detail view
**Example:**
```rust
// Source: https://ratatui.rs/concepts/layout/
let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
        Constraint::Percentage(40),  // contact list
        Constraint::Percentage(60),  // detail pane
    ])
    .split(area);
```

### Pattern 4: Stateful Table with Selection
**What:** Table widget with TableState tracking selected row and scroll offset
**When to use:** TUI-01 contact list, TUI-04 dashboard
**Example:**
```rust
// Source: https://docs.rs/ratatui/latest/ratatui/widgets/struct.Table.html
let rows: Vec<Row> = app.filtered.iter().map(|&idx| {
    let c = &app.contacts[idx].contact;
    Row::new(vec![
        Cell::from(c.name.clone()),
        Cell::from(c.company.clone()),
        Cell::from(format_status(&c.status)),
        Cell::from(format_date(&c.last_contacted)),
    ])
}).collect();

let table = Table::new(rows, [
    Constraint::Percentage(30),
    Constraint::Percentage(25),
    Constraint::Percentage(20),
    Constraint::Percentage(25),
])
.header(Row::new(vec!["Name", "Company", "Status", "Last Contact"]).style(Style::new().bold()))
.block(Block::bordered().title("Contacts"))
.row_highlight_style(Style::new().reversed())
.highlight_symbol(">> ");

frame.render_stateful_widget(table, area, &mut app.table_state);
```

### Anti-Patterns to Avoid
- **Loading contacts on every frame:** Load once at startup, reload only after mutations (log interaction). 985 files is fine in memory but slow to re-parse every 16ms.
- **Mixing business logic into view functions:** Keep view functions pure (just rendering). All state changes go through update/message.
- **Forgetting terminal restore on panic:** Use std::panic::set_hook to restore terminal before panic output, or the user gets a broken terminal.
- **Blocking the event loop:** All I/O (file reads/writes) should complete quickly. With 985 contacts, initial load takes <100ms -- acceptable at startup.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal raw mode management | Custom terminal setup/teardown | ratatui Terminal + crossterm | Handles alternate screen, raw mode, signal handling |
| Text input with cursor | Char-by-char input handling | tui-textarea or simple single-line state | Cursor movement, backspace, insert -- surprisingly complex |
| Table scrolling | Custom scroll offset math | ratatui TableState | Handles viewport, selection tracking, auto-scroll |
| ANSI color output | Manual escape codes | ratatui Style/Color system | Cross-platform, composable styles |

**Key insight:** ratatui's stateful widgets (TableState, ListState) handle the fiddly scroll/selection math. Use them instead of tracking offsets manually.

## Common Pitfalls

### Pitfall 1: Terminal Not Restored After Panic
**What goes wrong:** App panics, terminal stays in raw mode, user sees garbled output
**Why it happens:** Raw mode disables line buffering and echo; alternate screen hides normal output
**How to avoid:** Install a panic hook that calls restore_terminal() before the default panic handler
**Warning signs:** User reports "broken terminal" after crash

### Pitfall 2: Crossterm Version Conflicts
**What goes wrong:** Compilation errors about mismatched Event types
**Why it happens:** Adding crossterm as a direct dependency at a different version than ratatui uses
**How to avoid:** Always use `ratatui::crossterm` re-export, never add crossterm to Cargo.toml directly
**Warning signs:** "expected Event, found Event" type errors

### Pitfall 3: Flickering on Redraw
**What goes wrong:** Screen flickers when redrawing
**Why it happens:** Not using double-buffered rendering
**How to avoid:** ratatui's Terminal::draw() handles double-buffering automatically -- just use it correctly (render everything in the draw closure)
**Warning signs:** Visible flicker during typing or scrolling

### Pitfall 4: Search Filtering Performance
**What goes wrong:** UI feels sluggish during search with many contacts
**Why it happens:** Filtering 985 contacts on every keystroke with complex string matching
**How to avoid:** Pre-lowercase all searchable fields at load time; filter on pre-processed data. 985 contacts is small enough that simple contains() on lowercase strings will be <1ms.
**Warning signs:** Noticeable delay between keystrokes

### Pitfall 5: Blocking Event Read Prevents Timed Updates
**What goes wrong:** UI doesn't update until user presses a key
**Why it happens:** Using crossterm::event::read() which blocks indefinitely
**How to avoid:** Use event::poll(Duration::from_millis(250)) before read() -- this allows periodic redraws and timeout-based updates
**Warning signs:** Clock/status doesn't update, UI feels "stuck"

## Code Examples

### Terminal Init with Panic Hook
```rust
fn run_tui() -> anyhow::Result<()> {
    // Install panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    let mut terminal = init_terminal()?;
    let result = run_app(&mut terminal);
    restore_terminal()?;
    result
}
```

### Event Loop with Poll Timeout
```rust
fn run_app(terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
    let mut app = App::new()?;

    while app.running {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        if crossterm::event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind == KeyEventKind::Press {
                    let msg = handle_key(&app, key);
                    if let Some(m) = msg {
                        app.update(m)?;
                    }
                }
            }
        }
    }
    Ok(())
}
```

### Color-Coded Status/Priority (TUI-07)
```rust
fn status_style(status: &Option<Status>) -> Style {
    match status {
        Some(Status::Active) => Style::new().fg(Color::Green),
        Some(Status::Dormant) => Style::new().fg(Color::Yellow),
        Some(Status::LostTouch) => Style::new().fg(Color::Red),
        Some(Status::Archived) => Style::new().fg(Color::DarkGray),
        None => Style::default(),
    }
}

fn priority_style(priority: &Option<Priority>) -> Style {
    match priority {
        Some(Priority::High) => Style::new().fg(Color::Red).bold(),
        Some(Priority::Medium) => Style::new().fg(Color::Yellow),
        Some(Priority::Low) => Style::new().fg(Color::DarkGray),
        None => Style::default(),
    }
}
```

### Search Filtering (TUI-05)
```rust
fn filter_contacts(contacts: &[ContactFile], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..contacts.len()).collect();
    }
    let q = query.to_lowercase();
    contacts.iter().enumerate()
        .filter(|(_, cf)| {
            let c = &cf.contact;
            c.name.to_lowercase().contains(&q)
                || c.company.to_lowercase().contains(&q)
                || c.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect()
}
```

### Modal Overlay Pattern (for TUI-06 Log Interaction)
```rust
fn draw_log_modal(frame: &mut Frame, app: &App) {
    // Dim the background
    let area = frame.area();
    let block = Block::default().style(Style::default().bg(Color::DarkGray));
    frame.render_widget(Clear, area);

    // Center the modal
    let modal_area = centered_rect(60, 40, area);
    let modal = Block::bordered().title("Log Interaction");
    let inner = modal.inner(modal_area);
    frame.render_widget(modal, modal_area);

    // Render form fields inside inner
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // type input
            Constraint::Length(3),  // summary input
            Constraint::Length(3),  // action buttons
        ])
        .split(inner);
    // ... render input widgets in chunks
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tui-rs | ratatui | 2023 fork | tui-rs abandoned; ratatui is the active successor |
| ratatui monolithic crate | ratatui modular workspace | 0.30 (Dec 2025) | Can depend on ratatui-core for stable API; 0.29 still works fine |
| Manual terminal setup | ratatui::run() | 0.30 | Simplifies boilerplate but 0.29 manual init is well-understood |

**Deprecated/outdated:**
- tui-rs: Abandoned, use ratatui
- ratatui < 0.26: Old APIs, missing key features like TableState improvements

## Open Questions

1. **tui-textarea vs simple single-line input**
   - What we know: tui-textarea handles multi-line editing well but adds a dependency
   - What's unclear: Whether interaction log notes need multi-line input or if single-line summary is sufficient
   - Recommendation: Start with simple single-line input (type + summary fields); add tui-textarea only if multi-line notes are needed

2. **ratatui 0.29 vs 0.30**
   - What we know: 0.30 has nicer APIs but requires MSRV 1.86 and Rust 2024 edition
   - What's unclear: Whether the project's Rust toolchain meets MSRV 1.86
   - Recommendation: Use 0.29 for stability; upgrade to 0.30 later if desired

## Sources

### Primary (HIGH confidence)
- [ratatui.rs official docs](https://ratatui.rs/) - Layout, TEA pattern, event handling
- [docs.rs/ratatui](https://docs.rs/ratatui/latest/ratatui/) - Table widget API, StatefulWidget
- [ratatui v0.30 highlights](https://ratatui.rs/highlights/v030/) - Version comparison, breaking changes

### Secondary (MEDIUM confidence)
- [ratatui GitHub](https://github.com/ratatui/ratatui) - 18.7K stars, active development
- [crates.io/ratatui](https://crates.io/crates/ratatui/) - Version 0.30.0 latest (Dec 2025)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - ratatui is the undisputed standard for Rust TUIs
- Architecture: HIGH - TEA pattern is well-documented by ratatui team
- Pitfalls: HIGH - Common issues are well-known in the ecosystem
- Integration: HIGH - Existing store/models/commands provide clean interfaces to reuse

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable ecosystem, 30 days)
