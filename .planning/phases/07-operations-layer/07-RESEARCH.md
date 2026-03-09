# Phase 7: Operations Layer - Research

**Researched:** 2026-03-09
**Domain:** Rust module architecture / business logic extraction refactor
**Confidence:** HIGH

## Summary

Phase 7 is a pure refactoring phase: extract business logic from 10 CLI command handlers (add, list, search, show, edit, log, due, delete, archive, sync) into a shared `ops` module. The codebase is well-structured at ~1,500 lines across command handlers, with clear patterns that make extraction mechanical. Every handler follows the same shape: find CRM root, load contacts, do logic, format output. The ops layer cuts at "do logic" -- ops functions accept `&Path` (CRM root) plus plain arguments, return typed result structs, and never touch `OutputFormat`, `clap`, or `find_crm_root()`.

The sync commands are more complex (~550 lines) with credential loading, CardDAV client setup, and filter construction. Per user decisions, sync ops functions receive credentials and filters as arguments -- callers are responsible for loading credentials and constructing filters.

**Primary recommendation:** Use an `ops/` directory with submodules (`ops/contact.rs` for CRUD, `ops/sync.rs` for sync operations, `ops/error.rs` for `OpsError` enum). Move business logic incrementally per command, running `cargo test` after each extraction to ensure no regressions. Keep result structs in ops (they are already defined in command files with `Serialize` -- move them, re-export if needed). The TUI's `submit_log` duplicates CLI log logic and should be replaced with an ops call.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Each ops function returns a typed result struct (e.g., `AddResult { path, contact }`, `SearchResult { matches }`, `SyncPullResult { created, updated, skipped }`)
- Callers (CLI, future MCP, future bulk) are responsible for formatting output from these structs
- CRM root path passed as argument to all ops functions -- ops never calls `find_crm_root()` itself
- Contact lookup (name -> ContactFile) handled inside ops -- callers pass a name string, ops does fuzzy matching internally
- Ops module uses its own error enum (`OpsError::NotFound`, `OpsError::AmbiguousMatch`, `OpsError::ValidationFailed`, etc.) -- not anyhow. MCP can map these to proper error codes, CLI can show user-friendly messages
- Sync operations (pull, push, bidirectional) extracted into ops alongside CRUD commands
- Sync setup (interactive credential prompting) stays CLI-only -- not useful for MCP
- Credentials provided by caller: `ops::sync_pull(root, credentials, filter, opts)` -- ops never loads from keyring
- Sync filter construction (merging config + CLI flags) stays in callers -- ops receives a `SyncFilter`, doesn't know about config files
- Fix `update_existing_contact` bypassing `store::serialize_contact_file` -- route all writes through same path during extraction
- Remove unused `SyncConfig` struct field (dead_code warning: `apple_id` field is never read)
- Fix TUI dead_code warning
- Target: zero compiler warnings after Phase 7
- TUI calls ops functions for all operations: listing contacts, logging interactions, any future mutations
- TUI uses `ops::list()` for contact loading on startup and reload
- TUI's in-memory search filtering stays as-is (real-time keystroke filtering on loaded data -- not a business logic concern, performance-critical)

### Claude's Discretion
- Module structure (single `ops.rs` vs `ops/` directory with submodules)
- Exact OpsError variant names and structure
- Internal refactoring approach (extract-and-test vs move-and-verify)
- How to handle the delete confirmation prompt (likely: ops returns what would be deleted, CLI prompts, then calls ops::confirm_delete)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| OPS-01 | Business logic extracted from CLI handlers into shared ops module | Architecture patterns section defines module structure, extraction strategy per command, OpsError enum design |
| OPS-02 | All existing CLI commands delegate to ops layer (no behavior change) | Code examples section shows before/after patterns for each command, verification approach ensures identical behavior |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| thiserror | 1.x/2.x | Typed error enum for OpsError | Standard Rust approach for library error types; derives std::error::Error with display messages; better than anyhow for APIs that consumers need to match on |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| anyhow | (existing) | CLI-level error handling | Keep in CLI handlers and main.rs for user-facing error messages; ops functions return OpsError, CLI maps to anyhow |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| thiserror | Manual Error impl | thiserror eliminates boilerplate; no reason to hand-roll |
| thiserror | anyhow in ops | anyhow erases error types; MCP needs to match on specific error variants (NotFound -> 404, AmbiguousMatch -> 400) |

**Installation:**
```bash
cargo add thiserror
```

## Architecture Patterns

### Recommended Project Structure
```
src/
├── ops/
│   ├── mod.rs          # pub use re-exports, OpsError enum
│   ├── error.rs        # OpsError definition
│   ├── contact.rs      # add, list, search, show, edit, log, due, delete, archive, unarchive
│   └── sync.rs         # sync_pull, sync_push, sync_bidirectional (not setup)
├── commands/
│   ├── add.rs          # Thin wrapper: find_crm_root(), call ops::add(), format output
│   ├── list.rs         # (same pattern for all)
│   └── sync.rs         # Loads credentials, builds filter, calls ops::sync_*(), formats output
├── store.rs            # File I/O layer (unchanged)
├── frontmatter.rs      # YAML manipulation (unchanged)
├── models/             # Contact, ContactFile (unchanged)
├── format.rs           # OutputFormat, output(), output_list() (unchanged)
├── tui/
│   └── app.rs          # Calls ops:: instead of duplicating logic
└── main.rs             # Clap parsing, dispatch to commands (unchanged)
```

### Pattern 1: Ops Function Signature
**What:** Every ops function takes `&Path` (CRM root) plus plain typed arguments, returns `Result<TypedResult, OpsError>`
**When to use:** All ops functions
**Example:**
```rust
// src/ops/contact.rs
use std::path::Path;
use crate::models::ContactFile;
use super::error::OpsError;

pub struct AddResult {
    pub name: String,
    pub path: String,
}

pub fn add(root: &Path, name: &str) -> Result<AddResult, OpsError> {
    // Business logic from commands/add.rs
    // Uses store::*, frontmatter::*, validation::*
    // Returns typed result, no OutputFormat dependency
}
```

### Pattern 2: CLI Thin Wrapper
**What:** CLI command handlers become: find root, call ops, format output
**When to use:** Every command handler after extraction
**Example:**
```rust
// src/commands/add.rs (after refactor)
use crate::format::{self, OutputFormat};
use crate::ops;
use crate::store;

pub fn run(name: &str, output_format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let result = ops::add(&root, name)?;  // OpsError auto-converts to anyhow via From
    format::output(&result, output_format)
}
```

### Pattern 3: Delete Two-Phase Pattern
**What:** Ops exposes `find_delete_target()` returning what would be deleted, and `confirm_delete()` that actually deletes. CLI handles the confirmation prompt between calls.
**When to use:** Delete command (only command with interactive confirmation)
**Example:**
```rust
// src/ops/contact.rs
pub struct DeleteTarget {
    pub name: String,
    pub path: String,
}

pub fn find_delete_target(root: &Path, name: &str) -> Result<DeleteTarget, OpsError> {
    // Load contacts, fuzzy match, return target info
}

pub fn confirm_delete(root: &Path, name: &str) -> Result<DeleteResult, OpsError> {
    // Actually perform the deletion
}

// src/commands/delete.rs
pub fn run(name: &str, yes: bool, format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let target = ops::find_delete_target(&root, name)?;

    let confirmed = yes || Confirm::new()
        .with_prompt(format!("Delete {}? This cannot be undone.", target.name))
        .default(false)
        .interact()?;

    if confirmed {
        let result = ops::confirm_delete(&root, name)?;
        format::output(&result, format)
    } else {
        // Handle cancelled case
    }
}
```

### Pattern 4: Sync Credentials Struct
**What:** Bundle credentials into a plain struct that callers construct and pass to ops
**When to use:** All sync ops functions
**Example:**
```rust
// src/ops/sync.rs
pub struct SyncCredentials {
    pub apple_id: String,
    pub app_password: String,
}

pub struct SyncOpts {
    pub force: bool,
    pub dry_run: bool,
}

pub fn sync_pull(
    root: &Path,
    credentials: &SyncCredentials,
    filter: &SyncFilter,
    opts: &SyncOpts,
) -> Result<SyncPullResult, OpsError> {
    // All pull logic from commands/sync.rs::run_sync()
}
```

### Pattern 5: OpsError Enum
**What:** Typed error enum that downstream consumers (MCP, bulk) can match on
**When to use:** All ops function error returns
**Example:**
```rust
// src/ops/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpsError {
    #[error("no contact matching '{0}'")]
    NotFound(String),

    #[error("multiple contacts match '{query}': {matches}")]
    AmbiguousMatch { query: String, matches: String },

    #[error("validation failed: {0}")]
    ValidationFailed(String),

    #[error("sync error: {0}")]
    SyncError(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Internal(String),
}

// Enable CLI to use ? with anyhow
impl From<OpsError> for anyhow::Error {
    fn from(e: OpsError) -> Self {
        anyhow::anyhow!(e)
    }
}
```

### Anti-Patterns to Avoid
- **OutputFormat in ops:** Ops functions must never accept or reference `OutputFormat` or any formatting concern. Result structs can implement `Serialize` (needed for JSON output in CLI) but not `Display` with colored output -- that stays in CLI.
- **find_crm_root() in ops:** Ops receives `&Path`, never resolves it. This enables testing with temp dirs and future MCP where root is configured differently.
- **Credential loading in ops:** Ops receives credentials as arguments. Never touches keyring or config files.
- **Duplicating logic between CLI and TUI:** The TUI currently duplicates log interaction logic. After refactor, both call `ops::log_interaction()`.
- **Leaking clap types:** No `clap::ValueEnum` or `clap::Args` types should appear in ops function signatures.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Typed error enum | Manual `impl Error` + `impl Display` | `thiserror` derive macro | Eliminates boilerplate, auto-generates Display from #[error("...")] |
| Error conversion | Manual `From` impls for every error type | `thiserror` `#[from]` attribute | Handles `?` operator conversion automatically |

**Key insight:** This is a refactoring phase, not a new-feature phase. The primary risk is behavioral regression, not choosing wrong libraries. The only new dependency is `thiserror` for the error enum.

## Common Pitfalls

### Pitfall 1: Result Struct Placement
**What goes wrong:** Result structs (AddResult, SearchResult, etc.) are currently defined in command files and implement both `Serialize` and `Display` with colored output. Moving them to ops but keeping `Display` with `colored` crate usage means ops depends on a presentation concern.
**Why it happens:** Existing structs mix data and presentation.
**How to avoid:** Move result structs to ops WITHOUT the `Display` impl. Keep `Display` impls in command files (or a separate formatting module). Result structs in ops only need `Serialize` and `Debug`. CLI commands can wrap ops structs in display-aware types if needed.
**Warning signs:** `colored` crate appearing in ops module imports.

### Pitfall 2: update_existing_contact Bypass
**What goes wrong:** The current `update_existing_contact` in `commands/sync.rs` (line 451) writes directly with `format!("---\n{}---\n\n{}", fm, existing.body)` instead of using `store::serialize_contact_file()`. This could produce subtly different output (e.g., missing newlines).
**Why it happens:** Was written independently of the serialization function.
**How to avoid:** During extraction, route this through `store::serialize_contact_file()`. Verify with a diff that output is identical for existing contacts.
**Warning signs:** Multiple code paths that write contact files to disk.

### Pitfall 3: TUI Log Interaction Duplication
**What goes wrong:** `tui/app.rs::submit_log()` (lines 270-323) reimplements the interaction logging logic from `commands/log.rs::run()`. It operates on raw file strings instead of `ContactFile` structs, uses `frontmatter::update_field` on the full file content (not just frontmatter), and doesn't validate.
**Why it happens:** TUI can't call `commands::log::run()` because it prints to stdout, which interferes with the terminal UI.
**How to avoid:** Extract log interaction logic to `ops::log_interaction()` that returns a result without printing. TUI calls ops directly. The `next_follow_up()` utility function already exists in `commands/log.rs` -- move it to ops as well.
**Warning signs:** `use crate::commands::log::next_follow_up` in TUI code.

### Pitfall 4: Behavioral Regression in Edge Cases
**What goes wrong:** Subtle behavior differences after refactoring, e.g., empty list handling ("No contacts found." vs "[]"), error message formatting, exit codes.
**Why it happens:** Business logic and presentation are currently interleaved.
**How to avoid:** Run the full test suite (121 tests) after each extraction. For commands without tests, do manual before/after comparison of output for: normal case, empty case, error case.
**Warning signs:** Tests pass but manual behavior differs.

### Pitfall 5: Circular Dependencies
**What goes wrong:** `ops` module depends on `store`, `frontmatter`, `models`, `validation`, `sync`. If command modules also depend on ops AND ops depends on command utilities, you get cycles.
**Why it happens:** `next_follow_up()` is defined in `commands/log.rs` but used by both CLI and TUI.
**How to avoid:** Move ALL business logic utilities to ops. Nothing in `commands/` should contain business logic -- only argument parsing, ops calls, and output formatting.
**Warning signs:** `use crate::commands::*` appearing in ops module.

## Code Examples

### Current State: Command Handler Pattern (Before)
```rust
// src/commands/add.rs - typical pattern
pub fn run(name: &str, output_format: &OutputFormat) -> Result<()> {
    let root = store::find_crm_root()?;           // 1. Find root
    let contact = Contact { /* ... */ };            // 2. Business logic
    let raw_fm = store::generate_raw_frontmatter(&contact, &root)?;
    let cf = ContactFile { /* ... */ };
    let path = store::write_contact(&root, &cf)?;
    let result = AddResult { /* ... */ };           // 3. Build result
    format::output(&result, output_format)          // 4. Format output
}
```

### Target State: Ops + Thin Wrapper (After)
```rust
// src/ops/contact.rs
pub fn add(root: &Path, name: &str) -> Result<AddResult, OpsError> {
    let contact = Contact { /* ... */ };            // Business logic only
    let raw_fm = store::generate_raw_frontmatter(&contact, root)
        .map_err(|e| OpsError::Internal(e.to_string()))?;
    let cf = ContactFile { /* ... */ };
    let path = store::write_contact(root, &cf)
        .map_err(|e| OpsError::Internal(e.to_string()))?;
    Ok(AddResult {
        name: name.to_string(),
        path: path.display().to_string(),
    })
}

// src/commands/add.rs - thin wrapper
pub fn run(name: &str, output_format: &OutputFormat) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let result = ops::add(&root, name)?;
    format::output(&result, output_format)
}
```

### OpsError → anyhow Conversion
```rust
// In CLI commands, OpsError converts to anyhow automatically:
let result = ops::show(&root, name)?;  // OpsError -> anyhow::Error via From impl

// In MCP handlers (Phase 9), match on specific variants:
match ops::show(&root, name) {
    Ok(result) => /* success response */,
    Err(OpsError::NotFound(n)) => /* 404 response */,
    Err(OpsError::AmbiguousMatch { .. }) => /* 400 response */,
    Err(e) => /* 500 response */,
}
```

### Sync Extraction Pattern
```rust
// src/ops/sync.rs
pub fn sync_push(
    root: &Path,
    credentials: &SyncCredentials,
    filter: &SyncFilter,
    opts: &SyncOpts,
) -> Result<PushResult, OpsError> {
    let client = CardDavClient::new(&credentials.apple_id, &credentials.app_password)
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    let addressbook_url = client.discover_address_book()
        .map_err(|e| OpsError::SyncError(e.to_string()))?;

    // ... rest of push logic from commands/sync.rs::run_push()
}

// src/commands/sync.rs - thin wrapper
pub fn run_push(force: bool, dry_run: bool, filter: &SyncFilter, fmt: &OutputFormat) -> anyhow::Result<()> {
    let (apple_id, app_password) = config::load_credentials()?;
    let creds = ops::SyncCredentials { apple_id, app_password };
    let root = store::find_crm_root()?;
    let opts = ops::SyncOpts { force, dry_run };
    let result = ops::sync_push(&root, &creds, filter, &opts)?;
    format::output(&result, fmt)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `anyhow` for all errors | `thiserror` for library errors, `anyhow` for app errors | Rust convention since ~2020 | Enables downstream consumers to match error variants |
| Monolithic command handlers | Thin CLI + ops layer | Standard pattern for multi-consumer Rust apps | Enables MCP, bulk ops, and testing without CLI |

## Open Questions

1. **Result struct `Display` impls with colored output**
   - What we know: Current result structs implement `Display` with `colored` crate for terminal formatting. Ops should not depend on presentation concerns.
   - What's unclear: Whether to keep `Display` impls on ops result structs (without colors) or only implement them in CLI wrapper types.
   - Recommendation: Keep result structs in ops with `Serialize` + `Debug` only. Add `Display` impls in the command files that wrap or reference the ops structs. This keeps ops clean for MCP consumers. Alternatively, plain `Display` (no colors) in ops is acceptable since MCP won't use Display.

2. **Sync progress messages (println! during sync)**
   - What we know: Current sync commands print progress ("Discovering address book...", "Found N contacts on iCloud", "Pushing N changes...") interleaved with business logic.
   - What's unclear: How ops should report progress without printing to stdout.
   - Recommendation: For Phase 7, ops sync functions can return results with enough data for callers to construct their own progress messages. The progress println!s can stay in CLI wrappers since they are presentation, not business logic. Alternatively, a callback/progress-reporter pattern could be used but adds complexity -- defer to Phase 9 if MCP needs it.

## Validation Architecture

> `workflow.nyquist_validation` is not present in config.json -- skipping this section.

## Sources

### Primary (HIGH confidence)
- Direct codebase analysis of all 10 command handlers, store.rs, models, sync module, TUI app
- `cargo build` output: 1 warning (SyncConfig.apple_id never read)
- `cargo test`: 121 tests, all passing
- Rust `thiserror` crate: well-established, standard for typed error enums

### Secondary (MEDIUM confidence)
- Rust API design patterns (ops layer / service layer pattern) -- standard Rust convention for multi-consumer architectures

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - thiserror is the standard Rust approach; no novel dependencies needed
- Architecture: HIGH - ops/commands split is a well-understood refactoring pattern; codebase structure is clear
- Pitfalls: HIGH - identified from direct code reading; specific line numbers and code paths documented

**Research date:** 2026-03-09
**Valid until:** 2026-04-09 (stable refactoring domain, no external API dependencies)
