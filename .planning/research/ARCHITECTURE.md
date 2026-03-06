# Architecture Patterns

**Domain:** Plain-text personal CRM with CLI, TUI, sync, and MCP server interfaces
**Researched:** 2026-03-05
**Overall confidence:** MEDIUM (based on training data, ratatui and MCP patterns verified from well-known ecosystem conventions; CardDAV specifics are LOW confidence for Rust-native implementation)

## Recommended Architecture

The codebase must support four distinct "frontends" (CLI, TUI, CardDAV sync, MCP server) all operating on the same flat-file contact store. The correct structure is a **library core with multiple binary entry points**, not a monolith with mode flags.

```
                  +------------+   +-----------+   +----------+   +-----------+
                  | CLI (clap) |   | TUI (rat) |   | MCP srv  |   | Sync eng  |
                  +-----+------+   +-----+-----+   +-----+----+   +-----+-----+
                        |               |               |               |
                        v               v               v               v
                  +-------------------------------------------------------------+
                  |                     acrm-core (library crate)               |
                  |                                                             |
                  |  +----------+  +-----------+  +----------+  +------------+ |
                  |  | store    |  | query     |  | mutate   |  | serialize  | |
                  |  | (fs I/O) |  | (filter,  |  | (add,    |  | (json,     | |
                  |  |          |  |  search,  |  |  edit,   |  |  vcard,    | |
                  |  |          |  |  sort)    |  |  log)    |  |  md+yaml)  | |
                  |  +----------+  +-----------+  +----------+  +------------+ |
                  |                                                             |
                  |  +-------------------+  +--------------------------------+ |
                  |  | models (Contact,  |  | diff/merge (conflict           | |
                  |  |  ContactFile,     |  |  resolution, change tracking)  | |
                  |  |  Interaction)     |  +--------------------------------+ |
                  |  +-------------------+                                     |
                  +-------------------------------------------------------------+
                                          |
                                          v
                              +-------------------------+
                              | contacts/*.md (on disk) |
                              +-------------------------+
```

### Component Boundaries

| Component | Responsibility | Communicates With | Crate |
|-----------|---------------|-------------------|-------|
| **acrm-core** | All business logic, data access, query, mutation | File system (contacts/) | `lib` crate |
| **acrm (CLI)** | Parse CLI args, call core, format terminal output | acrm-core | `bin` in workspace |
| **acrm-tui** | Render ratatui UI, handle key events, call core | acrm-core | `bin` in workspace |
| **acrm-mcp** | JSON-RPC stdio server, expose tools/resources | acrm-core | `bin` in workspace |
| **acrm-sync** | CardDAV client, vCard conversion, conflict resolution | acrm-core, remote CardDAV | `bin` or library |
| **store** | Read/write markdown files, parse frontmatter | File system | Module in core |
| **query** | Filter, search, sort contacts | store, models | Module in core |
| **mutate** | Add, edit, log interactions, update fields | store, models | Module in core |
| **serialize** | Convert between Contact and JSON/vCard/markdown | models | Module in core |
| **diff/merge** | Detect changes, resolve conflicts (CRM-wins) | models, store | Module in core |

### Crate Structure (Cargo Workspace)

```
Cargo.toml              (workspace root)
crates/
  acrm-core/            (library: models, store, query, mutate, serialize, diff)
    src/lib.rs
    src/models/
    src/store.rs
    src/query.rs
    src/mutate.rs
    src/serialize.rs     (json, vcard, markdown)
    src/diff.rs
  acrm-cli/             (binary: clap-based CLI)
    src/main.rs
    src/output.rs        (human vs JSON formatting)
  acrm-tui/             (binary: ratatui terminal UI)
    src/main.rs
    src/app.rs           (App state)
    src/ui/              (render functions)
    src/event.rs         (input handling)
  acrm-mcp/             (binary: MCP JSON-RPC server)
    src/main.rs
    src/tools.rs         (tool definitions)
    src/resources.rs     (resource definitions)
  acrm-sync/            (binary or lib: CardDAV sync)
    src/main.rs
    src/carddav.rs       (HTTP/WebDAV client)
    src/vcard.rs         (vCard <-> Contact conversion)
    src/engine.rs        (sync logic, conflict resolution)
```

**Why a workspace instead of feature flags:** Each frontend has fundamentally different dependency trees (clap vs ratatui vs tokio/hyper). Feature flags would bloat every binary with unused dependencies. Workspace crates compile independently and share `acrm-core` as a dependency.

## Data Flow

### CLI Data Flow

```
User input -> clap parse -> command handler -> core::query/mutate -> store (fs) -> stdout
```

Straightforward, synchronous, stateless. Each invocation loads from disk, operates, writes back. This is the current architecture and works well.

### TUI Data Flow

```
Terminal events -> event loop -> App state update -> core::query/mutate -> store (fs)
                                     |
                                     v
                              ratatui render -> terminal draw
```

The TUI introduces **persistent application state**:

```rust
// acrm-tui/src/app.rs
pub struct App {
    pub contacts: Vec<ContactFile>,   // loaded at startup, refreshed on mutation
    pub selected: usize,              // cursor position in list
    pub mode: AppMode,                // List, Detail, Search, Log
    pub search_query: String,         // active search filter
    pub filtered: Vec<usize>,         // indices into contacts matching search
    pub should_quit: bool,
}

pub enum AppMode {
    List,
    Detail,
    Search,
    LogInteraction,
    Help,
}
```

**Key pattern:** The TUI loads all contacts into memory at startup and re-reads from disk after any mutation. For a personal CRM (hundreds, not millions of contacts), full reload is simpler and more correct than incremental cache invalidation.

### MCP Server Data Flow

```
AI agent -> JSON-RPC (stdin/stdout) -> tool dispatch -> core::query/mutate -> store (fs)
                                                                |
                                                                v
                                                        JSON response -> stdout
```

The MCP server is **stateless per-request** like the CLI, but communicates via JSON-RPC over stdio instead of CLI args. Each tool call maps to a core function.

MCP tools to expose:

| Tool | Maps To | Description |
|------|---------|-------------|
| `list_contacts` | `core::query::list` | List/filter contacts |
| `search_contacts` | `core::query::search` | Full-text search |
| `get_contact` | `core::query::get` | Get single contact details |
| `add_contact` | `core::mutate::add` | Create new contact |
| `log_interaction` | `core::mutate::log` | Log an interaction |
| `update_contact` | `core::mutate::update` | Edit contact fields |
| `due_followups` | `core::query::due` | Contacts needing follow-up |

MCP resources to expose:

| Resource | URI Pattern | Description |
|----------|-------------|-------------|
| Contact list | `contacts://list` | All contacts summary |
| Single contact | `contacts://{slug}` | Full contact detail |
| Schema | `schema://contact` | Contact field definitions |

### CardDAV Sync Data Flow

```
                    +------------------+
                    | Remote CardDAV   |
                    | (iCloud/etc)     |
                    +--------+---------+
                             |
                    PROPFIND/GET/PUT/DELETE (HTTP)
                             |
                    +--------v---------+
                    | carddav client   |
                    | (HTTP + WebDAV)  |
                    +--------+---------+
                             |
                    +--------v---------+
                    | vcard convert    |  vCard <-> Contact model
                    +--------+---------+
                             |
                    +--------v---------+
                    | sync engine      |  compare, diff, resolve
                    +--------+---------+
                             |
                    +--------v---------+
                    | core::mutate     |  write changes
                    +--------+---------+
                             |
                    +--------v---------+
                    | contacts/*.md    |
                    +-------------------+
```

Sync state tracking requires a local metadata file:

```yaml
# .sync/carddav-state.yaml
server: https://contacts.icloud.com
last_sync: 2026-03-05T10:30:00Z
etags:
  john-doe: "etag-abc123"
  jane-smith: "etag-def456"
sync_token: "sync-token-xyz"  # for WebDAV sync-collection
```

**Conflict resolution is simple by design:** CRM always wins. On conflict, the local version overwrites the remote. This avoids the entire class of merge problems.

## Patterns to Follow

### Pattern 1: Core Returns Data, Frontends Format

**What:** Core functions return structured data (`Vec<ContactFile>`, `Contact`, etc.), never print to stdout or format output. Each frontend formats appropriately (colored text for CLI, widgets for TUI, JSON for MCP).

**When:** Every core function.

**Example:**

```rust
// acrm-core/src/query.rs -- returns data, no formatting
pub fn search(root: &Path, query: &str) -> Result<Vec<ContactFile>> {
    let contacts = store::load_all_contacts(root)?;
    let query_lower = query.to_lowercase();
    Ok(contacts.into_iter().filter(|cf| matches_query(cf, &query_lower)).collect())
}

// acrm-cli/src/output.rs -- CLI formats for terminal
pub fn print_contacts(contacts: &[ContactFile], format: OutputFormat) {
    match format {
        OutputFormat::Human => { /* colored, tabular */ }
        OutputFormat::Json => { /* serde_json::to_string */ }
    }
}

// acrm-mcp/src/tools.rs -- MCP returns JSON
pub fn handle_search(params: Value) -> Result<Value> {
    let results = core::query::search(&root, &query)?;
    Ok(serde_json::to_value(results)?)
}
```

**Why this matters now:** The current codebase mixes business logic with `println!` and `colored` formatting inside command handlers. This must be separated before adding TUI or MCP.

### Pattern 2: Ratatui Immediate-Mode Rendering

**What:** The TUI uses ratatui's immediate-mode pattern: the entire screen is re-rendered every frame based on application state. No retained widget tree.

**When:** All TUI rendering.

**Example:**

```rust
// acrm-tui/src/ui/mod.rs
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.area());

    draw_contact_list(frame, app, chunks[0]);
    draw_contact_detail(frame, app, chunks[1]);
}
```

### Pattern 3: Sync as Explicit Command, Not Background Process

**What:** Sync runs when the user invokes `acrm sync` (or a TUI keybinding), not as a daemon or watcher. This keeps the architecture simple and predictable.

**When:** CardDAV sync operations.

**Why:** A personal CRM syncing on-demand avoids file-watching complexity, lock contention with git, and surprising mutations. The user decides when to sync.

### Pattern 4: serde Multi-Format Serialization

**What:** The `Contact` model derives `Serialize`/`Deserialize` and the `serialize` module provides format-specific functions (YAML frontmatter, JSON, vCard).

**When:** Any data conversion boundary.

**Example:**

```rust
// acrm-core/src/serialize.rs
pub fn to_json(contact: &Contact) -> Result<String> {
    Ok(serde_json::to_string_pretty(contact)?)
}

pub fn to_vcard(contact: &Contact) -> Result<String> {
    // Manual construction since vCard has specific field mappings
    let mut vcard = String::from("BEGIN:VCARD\nVERSION:3.0\n");
    vcard.push_str(&format!("FN:{}\n", contact.name));
    // ... map fields to vCard properties
    vcard.push_str("END:VCARD\n");
    Ok(vcard)
}

pub fn from_vcard(vcard: &str) -> Result<Contact> {
    // Parse vCard properties into Contact fields
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Feature Flags for Frontend Selection

**What:** Using `#[cfg(feature = "tui")]` to conditionally compile frontends in a single binary.

**Why bad:** Produces bloated binaries, makes dependency management messy, and creates confusing build matrices. A user who wants the CLI should not need ratatui in their dependency tree.

**Instead:** Cargo workspace with separate binary crates sharing a core library.

### Anti-Pattern 2: Global Mutable State in TUI

**What:** Using `lazy_static` or `static mut` for application state in the TUI.

**Why bad:** Makes testing impossible, causes subtle bugs with state ordering, prevents future async operations.

**Instead:** Pass `&mut App` through the event loop. All state lives in one struct.

### Anti-Pattern 3: In-Memory Database / Index

**What:** Building an in-memory index (e.g., SQLite, tantivy) to avoid re-reading files.

**Why bad:** For a personal CRM with likely <1000 contacts, the complexity of cache invalidation vastly outweighs the performance gain. Reading all files takes <50ms. An index must be kept in sync with the filesystem, which creates a second source of truth.

**Instead:** Read from disk on every operation. Profile if it becomes slow (it will not for personal-scale data).

### Anti-Pattern 4: Native CardDAV Protocol Implementation

**What:** Writing a full WebDAV/CardDAV protocol client from scratch in Rust.

**Why bad:** CardDAV (RFC 6352) sits on top of WebDAV (RFC 4918) which sits on HTTP with XML PROPFIND/PROPPATCH/REPORT methods. Implementing this correctly, including authentication (especially iCloud's token-based auth), TLS, redirects, and error handling is a substantial undertaking.

**Instead:** Use the `reqwest` crate for HTTP and build a thin CardDAV client that handles only the specific operations needed: PROPFIND for discovery, GET for vCard retrieval, PUT for updates, and sync-collection REPORT for change detection. Alternatively, shell out to `vdirsyncer` for the sync transport and focus only on vCard-to-Contact conversion. Evaluate Rust CardDAV crates (if any exist and are maintained) before building from scratch.

## Key Refactoring: Extracting acrm-core

The existing codebase must be refactored before new features can be added cleanly. This is the single most important architectural task.

### Current State

```
src/
  main.rs          (clap CLI, directly calls command handlers)
  store.rs         (file I/O, parsing)
  models/          (Contact, ContactFile)
  commands/        (add, list, search, show, log, due -- mix logic with formatting)
```

### Target State

```
crates/
  acrm-core/src/
    lib.rs
    models/        (moved from src/models/)
    store.rs       (moved from src/store.rs)
    query.rs       (extracted from commands/list, search, show, due)
    mutate.rs      (extracted from commands/add, log)
    serialize.rs   (new: json output, vcard later)
  acrm-cli/src/
    main.rs        (clap CLI, calls core, formats output)
    output.rs      (human + json formatting)
```

**Migration strategy:** This can be done in one commit. Move files, update `use` paths, split formatting from logic. No behavior changes, just structural. All existing tests (if any) continue to pass.

## Scalability Considerations

| Concern | At 100 contacts | At 1,000 contacts | At 10,000 contacts |
|---------|-----------------|--------------------|--------------------|
| File load time | <10ms | <100ms | ~500ms, may need lazy loading |
| Search | Brute force fine | Brute force fine | Consider simple index file |
| TUI responsiveness | No concern | No concern | Paginate, load on demand |
| Sync time | Seconds | ~1 minute | Incremental sync essential |
| Git repo size | Trivial | ~5MB | ~50MB, consider shallow clone |

For a personal CRM, 10,000 contacts is an extreme upper bound. The architecture should be optimized for simplicity at 100-1,000 scale, with escape hatches noted but not built.

## Suggested Build Order

The dependency graph dictates the build order:

```
Phase 1: Extract acrm-core (workspace, library extraction)
    |
    +-- Phase 2a: JSON output mode (serialize module, CLI --format flag)
    |       |
    |       +-- Phase 3: MCP server (depends on JSON serialization)
    |
    +-- Phase 2b: TUI (depends on core library, independent of JSON)
    |
    +-- Phase 2c: Contact editing (mutate module, needed before sync)
            |
            +-- Phase 4: CardDAV sync (depends on edit, serialize/vcard)
```

**Rationale:**
1. **Core extraction first** -- every other feature depends on the library/binary split
2. **JSON output and MCP together** -- MCP needs JSON serialization; building JSON output for CLI simultaneously is minimal extra work and validates the serialization
3. **TUI can parallelize** -- independent of JSON/MCP work, only needs core
4. **Editing before sync** -- sync needs to write changes to contacts; editing capability must exist first
5. **CardDAV last** -- highest complexity, most unknowns, needs editing + vCard serialization in place

## Sources

- Ratatui documentation and examples (ratatui.rs) -- architecture patterns, immediate-mode rendering, App state pattern: MEDIUM confidence (from training data, well-established patterns)
- MCP specification (modelcontextprotocol.io) -- tool/resource definitions, JSON-RPC over stdio: MEDIUM confidence (from training data, protocol is standardized)
- CardDAV RFC 6352, WebDAV RFC 4918 -- sync protocol: MEDIUM confidence (RFC standards are stable)
- Rust Cargo workspace documentation -- crate organization: HIGH confidence (stable Rust feature)
- Existing acrm codebase analysis -- current architecture assessment: HIGH confidence (directly inspected)
