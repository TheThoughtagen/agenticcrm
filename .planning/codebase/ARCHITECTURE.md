# Architecture

**Analysis Date:** 2026-03-05

## Pattern Overview

**Overall:** File-based CRM with CLI interface and shell script utilities

**Key Characteristics:**
- Plain-text data storage: contacts are markdown files with YAML frontmatter in `contacts/`
- No database, no server, no network services -- everything is local filesystem
- Dual interface: Rust CLI (`acrm`) for structured commands, plus shell scripts for quick operations and imports
- Flat data model: each contact is a self-contained file with no foreign keys or relational links

## Layers

**CLI Interface (Entry Point + Command Dispatch):**
- Purpose: Parse CLI arguments and dispatch to the correct command handler
- Location: `src/main.rs`
- Contains: `Cli` struct (clap derive), `Commands` enum, `main()` function
- Depends on: `clap`, `commands` module
- Used by: End user via `acrm` binary

**Command Handlers:**
- Purpose: Implement business logic for each CLI subcommand
- Location: `src/commands/`
- Contains: One module per command (`add.rs`, `list.rs`, `search.rs`, `show.rs`, `log.rs`, `due.rs`)
- Depends on: `store` module, `models` module, `colored`, `chrono`
- Used by: `main.rs` dispatch

**Data Models:**
- Purpose: Define the `Contact` struct and related enums for serialization/deserialization
- Location: `src/models/`
- Contains: `Contact`, `ContactFile`, `Relationship`, `Status`, `Priority` types
- Depends on: `serde`, `chrono`
- Used by: `store` module, `commands` module

**Storage Layer:**
- Purpose: Read/write/parse markdown+YAML contact files from the filesystem
- Location: `src/store.rs`
- Contains: `parse_contact_file()`, `serialize_contact_file()`, `load_all_contacts()`, `write_contact()`, `find_crm_root()`
- Depends on: `models` module, `serde_yaml`, `walkdir`, `anyhow`
- Used by: All command handlers

**Shell Scripts (Legacy/Utility Interface):**
- Purpose: Provide shell-native alternatives and import utilities
- Location: `scripts/`
- Contains: `add-contact.sh`, `search.sh`, `due-followups.sh`, `import-linkedin.sh`
- Depends on: Standard unix tools (`grep`, `sed`, `uuidgen`), templates
- Used by: End user, AI agents

**Schema & Templates:**
- Purpose: Define the contact data format and provide a blank template
- Location: `.schemas/contact.yaml`, `templates/contact.md`
- Contains: Field definitions, default values, template with `{{uuid}}` placeholder
- Depends on: Nothing
- Used by: Shell scripts (template), AI agents (schema reference)

## Data Flow

**Adding a Contact (Rust CLI):**

1. User runs `acrm add "Jane Smith"`
2. `main.rs` parses args, dispatches to `commands::add::run()`
3. `add::run()` calls `store::find_crm_root()` to locate the CRM directory
4. Creates a `Contact` struct with defaults + provided name, wraps in `ContactFile`
5. Calls `store::write_contact()` which serializes to YAML frontmatter + markdown body
6. Writes file to `contacts/jane-smith.md`

**Adding a Contact (Shell Script):**

1. User runs `./scripts/add-contact.sh "Jane Smith"`
2. Script copies `templates/contact.md`, replaces `{{uuid}}` with `uuidgen` output
3. Sets the `name` field via `sed`
4. Writes to `contacts/jane-smith.md`

**Logging an Interaction:**

1. User runs `acrm log "jane" -t coffee "Caught up at cafe"`
2. `log::run()` loads all contacts, filters by partial name match
3. Builds a markdown entry: `### YYYY-MM-DD | type | summary`
4. Inserts entry after `## Interaction Log` heading in the body
5. Updates `last_contacted` to today
6. Serializes and writes the updated file back to disk

**Searching/Listing:**

1. `store::load_all_contacts()` walks `contacts/` directory, reads every `.md` file
2. Each file is parsed: YAML frontmatter deserialized into `Contact`, remainder kept as `body`
3. Command handler filters/sorts the in-memory list
4. Results printed to stdout with `colored` formatting

**LinkedIn Import:**

1. User runs `./scripts/import-linkedin.sh Connections.csv`
2. Script reads CSV line-by-line, creates a contact markdown file per row
3. Skips existing files (by slug match)
4. Tags imported contacts with `linkedin-import`

**State Management:**
- No runtime state -- every command reads from disk, processes, and (optionally) writes back
- No caching, indexing, or incremental loading
- `find_crm_root()` resolves the project root via: `ACRM_ROOT` env var > current directory detection > `~/repos/agenticcrm` fallback

## Key Abstractions

**Contact:**
- Purpose: The core domain object representing a person in the CRM
- Definition: `src/models/contact.rs` (lines 34-110)
- Pattern: Flat struct with serde derive for YAML serialization
- Has `slug()` method for generating filename from name

**ContactFile:**
- Purpose: Combines parsed `Contact` (frontmatter) with raw markdown `body` and file `path`
- Definition: `src/models/contact.rs` (lines 126-131)
- Pattern: Used as the primary unit of I/O -- loaded from disk, modified in memory, written back

**Commands enum:**
- Purpose: Defines all CLI subcommands and their arguments
- Definition: `src/main.rs` (lines 15-52)
- Pattern: Clap derive macro for declarative CLI definition

**Relationship / Status / Priority enums:**
- Purpose: Constrain CRM metadata to valid values
- Definition: `src/models/contact.rs` (lines 6-32)
- Pattern: Serde rename for kebab-case (Status) and snake_case (Relationship, Priority) serialization

## Entry Points

**Rust CLI (`acrm`):**
- Location: `src/main.rs`
- Triggers: User runs `acrm <subcommand>` from terminal
- Responsibilities: Parse args, dispatch to command handler, return exit code
- Subcommands: `add`, `list`, `search`, `show`, `log`, `due`

**Shell Scripts:**
- Location: `scripts/add-contact.sh`, `scripts/search.sh`, `scripts/due-followups.sh`, `scripts/import-linkedin.sh`
- Triggers: User runs script directly or AI agent invokes it
- Responsibilities: Quick contact operations without compiling Rust

**Direct File Editing:**
- Location: `contacts/*.md`
- Triggers: User or AI agent edits markdown files directly
- Responsibilities: The files ARE the database -- any text editor is a valid interface

## Error Handling

**Strategy:** `anyhow::Result` throughout, with contextual error messages via `.context()` and `.with_context()`

**Patterns:**
- `bail!()` for user-facing errors (no match, ambiguous match)
- `eprintln!("Warning: ...")` for non-fatal parse failures (skipping bad files in `load_all_contacts()`)
- No custom error types -- relies entirely on `anyhow`
- Commands return `Result<()>` which propagates to `main()` for automatic error display

## Cross-Cutting Concerns

**Logging:** No logging framework. User-facing output via `println!()` with `colored` for terminal formatting. Warnings via `eprintln!()`.

**Validation:** Minimal. Serde deserialization validates YAML structure. No business rule validation (e.g., email format, date ranges). The schema at `.schemas/contact.yaml` is documentation only -- not enforced at runtime.

**Authentication:** Not applicable -- local filesystem tool with no network access.

**Contact Resolution:** Partial name matching used by `show`, `log`, and `search` commands. Exact match not required. Ambiguous matches (multiple results) cause `bail!()` in `show` and `log`.

---

*Architecture analysis: 2026-03-05*
