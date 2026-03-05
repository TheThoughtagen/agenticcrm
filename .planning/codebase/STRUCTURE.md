# Codebase Structure

**Analysis Date:** 2026-03-05

## Directory Layout

```
agenticcrm/
├── .claude/                # Claude Code local settings
│   └── settings.local.json
├── .planning/              # GSD planning documents
│   └── codebase/           # Codebase analysis (this file)
├── .schemas/               # Data format definitions
│   └── contact.yaml        # Contact field schema (documentation, not enforced)
├── contacts/               # THE DATABASE -- one markdown file per contact
│   ├── .gitkeep
│   └── example-jane-smith.md
├── imports/                # Staging area for import source files
│   └── .gitkeep
├── interactions/           # Currently unused -- placeholder directory
│   └── .gitkeep
├── scripts/                # Shell script utilities
│   ├── add-contact.sh      # Create contact from template
│   ├── due-followups.sh    # List overdue follow-ups
│   ├── import-linkedin.sh  # Import from LinkedIn CSV export
│   └── search.sh           # Grep-based contact search
├── src/                    # Rust CLI source code
│   ├── commands/           # One module per CLI subcommand
│   │   ├── mod.rs          # Re-exports all command modules
│   │   ├── add.rs          # `acrm add` -- create new contact
│   │   ├── due.rs          # `acrm due` -- show overdue follow-ups
│   │   ├── list.rs         # `acrm list` -- list all contacts (with optional tag filter)
│   │   ├── log.rs          # `acrm log` -- log an interaction
│   │   ├── search.rs       # `acrm search` -- search contacts by any field
│   │   └── show.rs         # `acrm show` -- display full contact details
│   ├── models/             # Data structures
│   │   ├── mod.rs          # Re-exports Contact, ContactFile, enums
│   │   └── contact.rs      # Contact struct, ContactFile, Relationship/Status/Priority enums
│   ├── main.rs             # CLI entry point (clap parser + dispatch)
│   └── store.rs            # Filesystem I/O (parse/serialize/load/write contacts)
├── templates/              # File templates
│   └── contact.md          # Blank contact template with {{uuid}} placeholder
├── Cargo.toml              # Rust package manifest
├── Cargo.lock              # Rust dependency lockfile (untracked)
├── CLAUDE.md               # Agent instructions for this project
├── README.md               # Project documentation
└── .gitignore              # Git ignore rules
```

## Directory Purposes

**`contacts/`:**
- Purpose: Primary data store -- every contact is a `.md` file here
- Contains: Markdown files with YAML frontmatter (one per person)
- Key files: Named by slug, e.g. `jane-smith.md`
- Naming: `firstname-lastname.md` (lowercase, hyphen-separated, alphanumeric only)

**`src/commands/`:**
- Purpose: CLI command implementations, one file per subcommand
- Contains: Rust modules, each exporting a `pub fn run(...)` function
- Key files: `add.rs`, `list.rs`, `search.rs`, `show.rs`, `log.rs`, `due.rs`

**`src/models/`:**
- Purpose: Domain types for serialization and in-memory representation
- Contains: `Contact` struct, `ContactFile` wrapper, enum types
- Key files: `contact.rs`

**`scripts/`:**
- Purpose: Shell-based utilities for quick operations and data import
- Contains: Bash scripts, each self-contained
- Key files: `import-linkedin.sh` (only import pathway), `add-contact.sh`

**`.schemas/`:**
- Purpose: Human/agent-readable field definitions for the contact format
- Contains: YAML schema file (documentation only, not validated at runtime)
- Key files: `contact.yaml`

**`templates/`:**
- Purpose: Boilerplate files for creating new records
- Contains: Markdown templates with placeholder values
- Key files: `contact.md` (uses `{{uuid}}` placeholder)

**`imports/`:**
- Purpose: Staging area for import source files (CSV, VCF, JSON)
- Contains: Gitignored import files (`.csv`, `.vcf`, `.json`)
- Note: Files here are not committed to git

**`interactions/`:**
- Purpose: Placeholder directory, currently unused
- Contains: Only `.gitkeep`

## Key File Locations

**Entry Points:**
- `src/main.rs`: Rust CLI entry point -- start here for all `acrm` commands
- `scripts/add-contact.sh`: Shell entry point for adding contacts
- `scripts/import-linkedin.sh`: Shell entry point for LinkedIn import

**Configuration:**
- `Cargo.toml`: Rust dependencies and package metadata
- `.schemas/contact.yaml`: Contact field definitions (reference only)
- `CLAUDE.md`: Agent instructions and conventions

**Core Logic:**
- `src/store.rs`: All filesystem I/O -- parsing, serializing, loading, writing contacts
- `src/models/contact.rs`: Contact data model with all fields and enums
- `src/commands/log.rs`: Most complex command -- modifies contact body and frontmatter

**Data:**
- `contacts/*.md`: The actual CRM data files
- `templates/contact.md`: Template for new contacts

## Naming Conventions

**Files:**
- Rust source: `snake_case.rs` (e.g., `contact.rs`, `store.rs`)
- Contact files: `firstname-lastname.md` (lowercase, hyphen-separated)
- Shell scripts: `kebab-case.sh` (e.g., `add-contact.sh`, `due-followups.sh`)
- Schema files: `snake_case.yaml`

**Directories:**
- All lowercase, no separators (e.g., `commands`, `models`, `contacts`)

**Rust Code:**
- Modules: `snake_case` (e.g., `mod commands`, `mod store`)
- Structs/Enums: `PascalCase` (e.g., `Contact`, `ContactFile`, `Commands`)
- Functions: `snake_case` (e.g., `find_crm_root`, `load_all_contacts`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `FRONTMATTER_DELIMITER`)

## Where to Add New Code

**New CLI Subcommand:**
1. Create `src/commands/<name>.rs` with `pub fn run(...) -> Result<()>`
2. Add `pub mod <name>;` to `src/commands/mod.rs`
3. Add variant to `Commands` enum in `src/main.rs`
4. Add match arm in `main()` to dispatch to `commands::<name>::run()`

**New Data Field on Contact:**
1. Add field to `Contact` struct in `src/models/contact.rs` (with `#[serde(default)]`)
2. Add field to `.schemas/contact.yaml` for documentation
3. Add field to `templates/contact.md` with empty/default value
4. Update `commands::add::run()` to initialize the field
5. Update `scripts/import-linkedin.sh` if relevant to imports
6. Update any command handlers that should display/use the new field

**New Shell Script:**
- Place in `scripts/` with kebab-case naming
- Start with `#!/usr/bin/env bash` and `set -euo pipefail`
- Resolve CRM root with: `CRM_ROOT="$(cd "$(dirname "$0")/.." && pwd)"`

**New Import Source:**
- Add script in `scripts/import-<source>.sh`
- Follow the pattern in `scripts/import-linkedin.sh`
- Tag imported contacts with `<source>-import`
- Set `source` field to the source name

**New Model/Type:**
- Place in `src/models/` as a new file
- Re-export from `src/models/mod.rs`

**Utility Functions:**
- Filesystem/storage utilities: add to `src/store.rs`
- If `store.rs` grows too large, consider splitting into `src/store/` module directory

## Special Directories

**`contacts/`:**
- Purpose: The database -- contains all CRM data
- Generated: No (user/agent created)
- Committed: Yes (this is the data store)

**`imports/`:**
- Purpose: Temporary staging for import source files
- Generated: No
- Committed: No (contents gitignored, only `.gitkeep` tracked)

**`target/`:**
- Purpose: Rust build artifacts
- Generated: Yes (by `cargo build`)
- Committed: No (gitignored)

**`.planning/`:**
- Purpose: GSD planning and codebase analysis documents
- Generated: Yes (by GSD tools)
- Committed: Depends on workflow

---

*Structure analysis: 2026-03-05*
