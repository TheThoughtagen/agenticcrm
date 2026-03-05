# Technology Stack

**Analysis Date:** 2026-03-05

## Languages

**Primary:**
- Rust (Edition 2024) - CLI application (`src/`)
- Bash - Utility scripts (`scripts/`)

**Secondary:**
- YAML - Contact frontmatter data and schema definitions (`.schemas/contact.yaml`, `contacts/*.md`)
- Markdown - Contact storage format and documentation

## Runtime

**Environment:**
- Rust (stable, edition 2024) - compiled binary
- Bash (system shell) - utility scripts require `uuidgen`, `grep`, `sed`, `date`

**Package Manager:**
- Cargo (Rust) - `Cargo.toml`
- Lockfile: `Cargo.lock` present (untracked in git)

## Frameworks

**Core:**
- clap 4 (with `derive` feature) - CLI argument parsing and subcommand routing (`src/main.rs`)

**Testing:**
- Not detected - no test framework configured

**Build/Dev:**
- Cargo (Rust build system) - standard `cargo build` / `cargo run`

## Key Dependencies

**Critical:**
- `clap` 4 (derive) - CLI interface, defines all subcommands in `src/main.rs`
- `serde` 1 (derive) - Serialization/deserialization of Contact structs (`src/models/contact.rs`)
- `serde_yaml` 0.9 - YAML frontmatter parsing and writing (`src/store.rs`)

**Infrastructure:**
- `uuid` 1 (v4) - Generating contact IDs (`src/commands/add.rs`)
- `chrono` 0.4 (serde) - Date handling for `NaiveDate` fields (`src/models/contact.rs`)
- `walkdir` 2 - Recursive directory traversal for loading contacts (`src/store.rs`)
- `anyhow` 1 - Error handling throughout all modules
- `colored` 3 - Terminal color output for CLI display
- `dirs` 6 - Home directory resolution for CRM root fallback (`src/store.rs`)

## Configuration

**Environment:**
- `ACRM_ROOT` env var (optional) - Override CRM root directory location
- No `.env` file present; no environment secrets required
- CRM root auto-detected: checks `ACRM_ROOT`, then current directory, then `~/repos/agenticcrm`

**Build:**
- `Cargo.toml` - Single crate, no workspace
- Binary name: `acrm`
- Edition: 2024

## Data Format

**Contact Storage:**
- Markdown files with YAML frontmatter in `contacts/` directory
- Schema definition: `.schemas/contact.yaml`
- Template: `templates/contact.md`
- No database - plain text files versioned with git

**Schema:**
- Required fields: `id`, `name`
- Date format: YYYY-MM-DD (using `chrono::NaiveDate`)
- Enums: `relationship`, `status`, `priority` (defined in `src/models/contact.rs`)

## Platform Requirements

**Development:**
- Rust toolchain (edition 2024 support, i.e., Rust 1.85+)
- Bash shell (for utility scripts)
- `uuidgen` command (used by `scripts/add-contact.sh` and `scripts/import-linkedin.sh`)
- Git (version control is part of the design philosophy)

**Production:**
- Single compiled binary (`acrm`)
- Runs locally on user's machine - no server, no cloud deployment
- Filesystem access to `contacts/` directory

---

*Stack analysis: 2026-03-05*
