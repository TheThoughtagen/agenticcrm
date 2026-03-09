# Phase 8: Bulk Operations & Query Engine - Research

**Researched:** 2026-03-09
**Domain:** CLI query parsing, bulk file operations, Unix composability (Rust)
**Confidence:** HIGH

## Summary

Phase 8 adds a query engine for filtering contacts by field predicates and bulk operations (update, delete, archive, tag) on matched sets. The codebase already has all the primitives needed: `store::load_all_contacts()` loads all contacts into memory, `ops::contact` has individual edit/delete/archive functions, the `Contact` struct has typed fields with serde, and `OutputFormat::Json` already handles JSON output. The `frontmatter` module handles raw YAML editing preserving comments.

The core new work is: (1) a predicate parser that turns `status=dormant AND tags~friend` into filters over `Contact` structs, (2) a `bulk` subcommand with preview/confirm/dry-run flow, and (3) a `bulk-update --stdin` command that reads JSON from stdin for Unix piping.

**Primary recommendation:** Build a simple predicate engine directly in Rust (no external query library needed) -- the Contact struct has ~30 fields with known types. Parse `field=value`, `field!=value`, `field~substring` predicates connected by implicit AND. Reuse existing `ops::contact::edit()` logic for applying `--set` changes, and reuse `ops::contact::archive()`/`confirm_delete()` patterns for bulk archive/delete.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| BULK-01 | Query contacts with field-based predicates (`acrm bulk 'status=dormant'`) | Predicate parser over Contact struct; load_all_contacts + filter |
| BULK-02 | Bulk update fields on matched contacts (`--set field=value`) | Reuse ops::contact::edit() pattern with raw_frontmatter editing |
| BULK-03 | Bulk delete or archive matched contacts | Reuse ops::contact::confirm_delete() and archive() patterns |
| BULK-04 | Bulk add/remove tags on matched contacts | Extend frontmatter::update_array_field(); add/remove semantics |
| BULK-05 | Preview and require confirmation (or `--yes` to skip) | dialoguer::Confirm pattern from commands::delete.rs |
| BULK-06 | `--dry-run` to preview without changes | Report what would change without calling write functions |
| BULK-07 | JSON pipe input (`acrm search --json \| acrm bulk-update --stdin`) | Read JSON array of {name, path} from stdin; serde_json::from_reader |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.x | CLI argument parsing for `bulk` subcommand | Already in project, derive-based |
| serde_json | 1.x | JSON stdin parsing for `--stdin` mode | Already in project |
| dialoguer | 0.11 | Confirmation prompts for bulk operations | Already in project, used by delete command |
| colored | 3.x | Terminal output formatting | Already in project |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| regex | 1.x | Substring matching in predicates | Already in project, use for `~` operator |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom predicate parser | SQL-like parser (sqlparser-rs) | Overkill for ~30 known fields; custom is simpler and type-aware |
| Load all + filter | Build an index/SQLite | Out of scope per REQUIREMENTS.md ("Flat file scan is sufficient for personal scale <10K contacts") |

**Installation:**
No new dependencies needed. All required crates are already in Cargo.toml.

## Architecture Patterns

### Recommended Project Structure
```
src/
├── ops/
│   ├── contact.rs      # Add: query(), bulk_update(), bulk_delete(), bulk_archive(), bulk_tag()
│   ├── error.rs         # Existing OpsError (no changes needed)
│   └── mod.rs
├── commands/
│   ├── bulk.rs          # NEW: CLI handler for `acrm bulk` subcommand
│   └── mod.rs           # Add: pub mod bulk;
├── query.rs             # NEW: Predicate parser and matcher
└── main.rs              # Add: Bulk subcommand to Commands enum
```

### Pattern 1: Predicate Parser
**What:** Parse simple field=value predicates into a filter chain over Contact structs
**When to use:** BULK-01 query parsing
**Example:**
```rust
// query.rs
pub enum Op {
    Eq,       // field=value (exact match, case-insensitive)
    NotEq,    // field!=value
    Contains, // field~value (substring match)
}

pub struct Predicate {
    pub field: String,
    pub op: Op,
    pub value: String,
}

pub struct Query {
    pub predicates: Vec<Predicate>,  // implicit AND
}

impl Query {
    /// Parse "status=dormant,tags~friend" or "status=dormant tags~friend"
    pub fn parse(input: &str) -> Result<Self, OpsError> { ... }

    /// Test whether a Contact matches all predicates
    pub fn matches(&self, contact: &Contact) -> bool { ... }
}
```

**Field accessor pattern:** Use a `get_field_value(contact: &Contact, field: &str) -> FieldValue` function that returns either a single string or a list of strings, dispatching on the known field names. This avoids reflection and keeps it type-safe.

```rust
enum FieldValue {
    Single(String),
    List(Vec<String>),
    Date(Option<NaiveDate>),
    None,
}

fn get_field_value(contact: &Contact, field: &str) -> FieldValue {
    match field {
        "name" => FieldValue::Single(contact.name.clone()),
        "status" => contact.status.as_ref()
            .map(|s| FieldValue::Single(format!("{s:?}").to_lowercase()))
            .unwrap_or(FieldValue::None),
        "tags" => FieldValue::List(contact.tags.clone()),
        "company" => FieldValue::Single(contact.company.clone()),
        // ... all other fields
        _ => FieldValue::None,
    }
}
```

### Pattern 2: Bulk Operation with Preview/Confirm
**What:** Show what will change, ask for confirmation, then apply
**When to use:** BULK-02 through BULK-06
**Example:**
```rust
// ops/contact.rs
pub struct BulkResult {
    pub matched: usize,
    pub affected: usize,
    pub changes: Vec<BulkChange>,
    pub dry_run: bool,
}

pub struct BulkChange {
    pub name: String,
    pub path: String,
    pub action: String,  // "updated field X", "deleted", "archived", "added tag Y"
}

/// Query contacts matching predicates, return matched set
pub fn query(root: &Path, query: &Query) -> Result<Vec<ContactFile>, OpsError> {
    let contacts = store::load_all_contacts(root).map_err(internal)?;
    Ok(contacts.into_iter().filter(|cf| query.matches(&cf.contact)).collect())
}

/// Bulk update matched contacts
pub fn bulk_update(
    root: &Path,
    matched: &[ContactFile],
    sets: &[String],
    dry_run: bool,
) -> Result<BulkResult, OpsError> { ... }
```

### Pattern 3: Stdin JSON Piping
**What:** Read JSON array from stdin for Unix composability
**When to use:** BULK-07
**Example:**
```rust
// In commands/bulk.rs
use std::io::{self, Read};

fn read_contacts_from_stdin() -> Result<Vec<StdinContact>, OpsError> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let contacts: Vec<StdinContact> = serde_json::from_str(&input)
        .map_err(|e| OpsError::ValidationFailed(format!("Invalid JSON from stdin: {e}")))?;
    Ok(contacts)
}

#[derive(Deserialize)]
struct StdinContact {
    name: String,
    #[serde(default)]
    path: Option<String>,
}
```

The stdin format matches the existing `SearchMatch` JSON output: `[{"name": "...", "company": "...", "path": "..."}]`. The `bulk-update` command ignores unknown fields, only needs `name` to re-resolve contacts.

### Pattern 4: CLI Subcommand Structure
**What:** Clap subcommand with query + action flags
**When to use:** Command definition
**Example:**
```rust
/// Bulk query and operate on contacts
Bulk {
    /// Query predicate (e.g. 'status=dormant', 'tags~friend')
    query: String,
    /// Set field values (repeatable, e.g. --set status=active)
    #[arg(long = "set", num_args = 1)]
    sets: Vec<String>,
    /// Delete matched contacts
    #[arg(long)]
    delete: bool,
    /// Archive matched contacts
    #[arg(long)]
    archive: bool,
    /// Add tag to matched contacts (repeatable)
    #[arg(long = "add-tag", num_args = 1)]
    add_tags: Vec<String>,
    /// Remove tag from matched contacts (repeatable)
    #[arg(long = "remove-tag", num_args = 1)]
    remove_tags: Vec<String>,
    /// Skip confirmation prompt
    #[arg(short, long)]
    yes: bool,
    /// Preview changes without writing
    #[arg(long)]
    dry_run: bool,
},
/// Bulk update contacts from JSON stdin
BulkUpdate {
    /// Read contact list from stdin (JSON array)
    #[arg(long)]
    stdin: bool,
    /// Set field values (repeatable)
    #[arg(long = "set", num_args = 1)]
    sets: Vec<String>,
    /// Delete matched contacts
    #[arg(long)]
    delete: bool,
    /// Archive matched contacts
    #[arg(long)]
    archive: bool,
    /// Add tag (repeatable)
    #[arg(long = "add-tag", num_args = 1)]
    add_tags: Vec<String>,
    /// Remove tag (repeatable)
    #[arg(long = "remove-tag", num_args = 1)]
    remove_tags: Vec<String>,
    /// Skip confirmation
    #[arg(short, long)]
    yes: bool,
    /// Dry run
    #[arg(long)]
    dry_run: bool,
},
```

### Anti-Patterns to Avoid
- **Loading contacts multiple times:** Load once, filter, then apply changes in a loop. Do NOT call `edit()` per contact (it calls `load_all_contacts` each time).
- **Modifying while iterating:** Collect all changes first, apply after the filter pass. Archive/delete moves files so paths could become invalid.
- **Over-engineering the query language:** No need for OR, parentheses, or nested expressions. Simple field=value with implicit AND covers all real use cases for a personal CRM.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal confirmation | Custom y/n parser | `dialoguer::Confirm` | Already used in delete command, handles edge cases |
| JSON serialization | Manual string building | `serde_json` | Already in project, handles escaping correctly |
| CLI parsing | Manual arg parsing | `clap` derive macros | Already in project, handles help, validation, conflicts |

**Key insight:** Almost everything needed for bulk ops already exists as single-contact operations in `ops::contact`. The main new code is the predicate parser and the loop-over-matched-contacts orchestration.

## Common Pitfalls

### Pitfall 1: File Path Invalidation During Bulk Delete/Archive
**What goes wrong:** Deleting or archiving a contact changes the filesystem state. If you're iterating over a collected list of ContactFiles, subsequent operations work fine since paths are independent -- but if you re-load between operations, indices shift.
**Why it happens:** Archive moves files from contacts/ to archive/; delete removes them entirely.
**How to avoid:** Collect all matched ContactFiles first, then apply operations in a single pass. Each file path is independent so deleting one doesn't affect another.
**Warning signs:** "file not found" errors during bulk operations.

### Pitfall 2: Confirmation UX for Destructive Operations
**What goes wrong:** User accidentally deletes 500 contacts because the confirmation didn't make the scope clear.
**Why it happens:** Preview says "N contacts matched" but doesn't show which ones.
**How to avoid:** Always show the list of affected contact names (truncated if >20), the operation that will be applied, and require explicit confirmation. For delete operations especially, show count prominently.
**Warning signs:** User complaints about accidental data loss.

### Pitfall 3: Status Enum Comparison
**What goes wrong:** User types `status=dormant` but the Status enum serializes differently.
**Why it happens:** `Status::Dormant` serde serializes as `"dormant"` (kebab-case via `#[serde(rename_all = "kebab-case")]`), and `Status::LostTouch` serializes as `"lost-touch"`. The Debug format (`format!("{s:?}")`) gives `"Dormant"` (PascalCase).
**How to avoid:** In `get_field_value`, serialize the status with serde_yaml/serde_json to get the canonical string, or use a hardcoded match. Case-insensitive comparison helps.
**Warning signs:** `status=lost-touch` not matching any contacts.

### Pitfall 4: Array Field Matching
**What goes wrong:** `tags=friend` does exact match on the entire array instead of checking if "friend" is in the tags list.
**Why it happens:** `=` on an array field should mean "contains this element" not "equals this array".
**How to avoid:** In the `matches()` function, check field type: for `FieldValue::List`, `=` means "any element equals value", `~` means "any element contains substring".
**Warning signs:** Array field predicates returning zero results.

### Pitfall 5: Stdin Piping Hangs
**What goes wrong:** `acrm bulk-update --stdin` hangs waiting for input when user forgets to pipe something.
**Why it happens:** `stdin().read_to_string()` blocks until EOF.
**How to avoid:** Check `atty::is(atty::Stream::Stdin)` or just document clearly. If stdin is a TTY, print a helpful error message. The `atty` crate is lightweight, or use `std::io::stdin().is_terminal()` (stabilized in Rust 1.70).
**Warning signs:** Command appears to hang with no output.

## Code Examples

### Existing Edit Pattern (reuse for bulk update)
```rust
// From ops/contact.rs - edit() function
// This pattern: parse key=value, update raw_frontmatter, re-parse, validate, write
// Should be extracted into a helper that bulk_update can call per-contact
for set_arg in sets {
    let (key, value) = set_arg.split_once('=').ok_or_else(|| ...)?;
    if ARRAY_FIELDS.contains(&key) {
        cf.raw_frontmatter = frontmatter::update_array_field(&cf.raw_frontmatter, key, &values);
    } else {
        cf.raw_frontmatter = frontmatter::update_field(&cf.raw_frontmatter, key, &yaml_value);
    }
}
```

### Existing Delete Pattern (reuse for bulk delete)
```rust
// Two-phase: find_delete_target + confirm_delete
// For bulk: skip the find phase (already have matched contacts), just delete each file
std::fs::remove_file(&cf.path)?;
```

### Existing Search JSON Output (format for stdin piping)
```json
// acrm search "smith" --format json produces:
[
  {
    "name": "Jane Smith",
    "company": "Acme",
    "path": "contacts/jane-smith.md"
  }
]
```
The `bulk-update --stdin` command should accept this format, using `name` to re-match contacts.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `std::io::stdin().is_terminal()` needs nightly | Stabilized in std since Rust 1.70 | 2023-06 | No need for atty crate |
| Manual clap arg conflicts | `#[arg(conflicts_with)]` in clap 4 | clap 4.0 | Can prevent --delete + --archive together |

**Deprecated/outdated:**
- `atty` crate: Use `std::io::stdin().is_terminal()` instead (stabilized)

## Open Questions

1. **Query syntax: space-separated vs comma-separated predicates?**
   - Recommendation: Support both. Split on whitespace and commas. Simple to implement.

2. **Should `acrm bulk` with no action flags just display matches?**
   - Recommendation: Yes. `acrm bulk 'status=dormant'` with no --set/--delete/--archive flags should just list matching contacts (like a filtered search). This covers the success criterion "User can run `acrm bulk 'status=dormant'` and see all contacts matching the query."

3. **Tag add/remove semantics for --set vs --add-tag/--remove-tag?**
   - `--set tags=a,b` replaces the entire tags array (existing behavior from edit)
   - `--add-tag friend` appends to existing tags (new)
   - `--remove-tag old-tag` removes from existing tags (new)
   - These are distinct operations and should coexist.

4. **Mutual exclusivity of --delete and --archive?**
   - Recommendation: Make them mutually exclusive via clap `conflicts_with`. Can combine --set with --add-tag/--remove-tag.

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `src/ops/contact.rs`, `src/store.rs`, `src/models/contact.rs`, `src/commands/*.rs`
- Codebase analysis: `Cargo.toml` for dependency inventory
- `.schemas/contact.yaml` for complete field list
- `.planning/REQUIREMENTS.md` for BULK-01 through BULK-07 specs and out-of-scope notes

### Secondary (MEDIUM confidence)
- Rust std library `is_terminal()` stabilized in 1.70 (well-documented)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies needed, all crates already in project
- Architecture: HIGH - straightforward extension of existing ops/commands patterns
- Pitfalls: HIGH - derived from direct codebase analysis (enum serialization, array field semantics)
- Query engine: HIGH - simple predicate parser over known struct fields, no ambiguity

**Research date:** 2026-03-09
**Valid until:** 2026-04-09 (stable domain, no external API dependencies)
