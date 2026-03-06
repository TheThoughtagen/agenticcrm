# Phase 1: CLI Foundation - Research

**Researched:** 2026-03-05
**Domain:** Rust CLI tooling, YAML frontmatter round-trip editing, contact data validation
**Confidence:** HIGH

## Summary

Phase 1 extends an existing Rust CLI (`acrm`) that already has `add`, `list`, `search`, `show`, `log`, and `due` commands. The codebase uses clap 4 (derive), serde/serde_yaml 0.9, chrono, colored, anyhow, walkdir, and uuid. The existing `Contact` struct maps 1:1 to the schema, and `ContactFile` pairs parsed frontmatter with a raw markdown body.

The critical technical challenge is **CLI-03: round-trip serialization**. The current `serialize_contact_file` uses `serde_yaml::to_string`, which destroys YAML comments (e.g., `# Contact`, `# Professional`) and reorders fields. The contact template and example files rely on these section comments for readability. The solution is to avoid full re-serialization: parse YAML for reading and validation, but write changes back by doing targeted text replacements on the raw frontmatter string, preserving comments, field order, and unknown fields.

The remaining requirements are straightforward: add a `--format json` global flag (CLI-01), an `edit` subcommand for field updates (CLI-02), validation logic (CLI-04), `delete`/`archive` commands with confirmation (CLI-05), and cadence-based follow-up calculation in the `log` command (CLI-06).

**Primary recommendation:** Implement a raw-text frontmatter editor that parses individual YAML key-value lines with regex, modifies targeted fields in-place, and preserves everything else verbatim. Use `serde_yaml` only for reading/validation, never for writing back to disk.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-01 | JSON output via `--format json` | Add global `--format` flag to clap, add `serde_json` dep, serialize `Contact` struct |
| CLI-02 | Edit contact frontmatter from CLI | New `edit` subcommand with `--field value` args, uses raw-text frontmatter editor |
| CLI-03 | Round-trip serialization without data loss | Raw-text frontmatter editing approach; never re-serialize through serde_yaml |
| CLI-04 | Validate required fields, enums, dates before write | Validation module checking schema constraints before any disk write |
| CLI-05 | Delete or archive contacts from CLI | `delete` (with confirm) and `archive` (sets status, moves to archive/) commands |
| CLI-06 | Auto-calculate next_follow_up from cadence | Parse cadence strings ("weekly", "monthly", "quarterly") and compute next date on `log` |
</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4 (derive) | CLI argument parsing | Industry standard for Rust CLIs |
| serde | 1 (derive) | Serialization framework | Universal Rust serialization |
| serde_yaml | 0.9 | YAML parsing (read-only in this phase) | Used for parsing frontmatter, NOT for writing back |
| chrono | 0.4 (serde) | Date handling | Standard Rust date library |
| anyhow | 1 | Error handling | Ergonomic error propagation |
| colored | 3 | Terminal coloring | Human-readable output formatting |
| walkdir | 2 | Directory traversal | Loading all contacts |
| uuid | 1 (v4) | UUID generation | Contact IDs |
| dirs | 6 | Home directory resolution | CRM root discovery |

### New Dependencies
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| serde_json | 1 | JSON serialization | CLI-01: `--format json` output |
| dialoguer | 0.11+ | Interactive prompts | CLI-05: delete confirmation (`Confirm::new()`) |
| regex | 1 | Pattern matching | Raw frontmatter field editing for round-trip preservation |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Raw-text YAML editing | yamlpatch (0.12) | yamlpatch uses tree-sitter; adds heavy dependency for simple key-value edits. Raw text is sufficient for flat frontmatter. |
| Raw-text YAML editing | rust-yaml (round-trip mode) | Low maturity, unclear download count. Risk not justified for this use case. |
| dialoguer | --yes/-y flag only | Skipping confirmation is scriptable but unsafe for destructive ops. Use both: dialoguer for interactive, --yes for scripts. |

**Installation:**
```bash
cargo add serde_json@1 dialoguer@0.11 regex@1
```

## Architecture Patterns

### Current Project Structure (preserve and extend)
```
src/
├── main.rs              # CLI entry point, clap definition
├── models/
│   ├── mod.rs
│   └── contact.rs       # Contact, ContactFile, enums
├── commands/
│   ├── mod.rs
│   ├── add.rs           # existing
│   ├── list.rs          # existing
│   ├── search.rs        # existing
│   ├── show.rs          # existing
│   ├── log.rs           # existing (needs CLI-06 enhancement)
│   ├── due.rs           # existing
│   ├── edit.rs          # NEW: CLI-02
│   ├── delete.rs        # NEW: CLI-05
│   └── archive.rs       # NEW: CLI-05
├── store.rs             # File I/O, parsing, serialization
├── validation.rs        # NEW: CLI-04
├── format.rs            # NEW: CLI-01 (output formatting)
└── frontmatter.rs       # NEW: CLI-03 (raw-text frontmatter editor)
```

### Pattern 1: Global Output Format Flag (CLI-01)
**What:** Add `--format` as a global arg on the top-level `Cli` struct so all subcommands inherit it.
**When to use:** Every command that produces output.
**Example:**
```rust
#[derive(clap::ValueEnum, Clone, Debug, Default)]
enum OutputFormat {
    #[default]
    Human,
    Json,
}

#[derive(Parser)]
#[command(name = "acrm")]
struct Cli {
    #[arg(short, long, global = true, default_value = "human")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}
```

Each command returns a structured result type that implements both Display (human) and Serialize (JSON). The format flag controls which serialization path is used. Output function pattern:

```rust
fn output<T: Serialize + std::fmt::Display>(data: &T, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Human => println!("{data}"),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(data)?),
    }
    Ok(())
}
```

### Pattern 2: Raw-Text Frontmatter Editor (CLI-03)
**What:** Parse the raw frontmatter text line-by-line, find target fields by key name, replace their values in-place, leave everything else (comments, blank lines, ordering) untouched.
**When to use:** Any write-back operation (edit, log, archive status change).
**Example:**
```rust
/// Update a single field's value in raw YAML frontmatter text.
/// Returns the modified frontmatter string with comments/order preserved.
fn update_field(raw_frontmatter: &str, key: &str, new_value: &str) -> String {
    let re = regex::Regex::new(&format!(r"(?m)^({}:\s*)(.*)$", regex::escape(key))).unwrap();
    if re.is_match(raw_frontmatter) {
        re.replace(raw_frontmatter, format!("${{1}}{new_value}")).to_string()
    } else {
        // Field doesn't exist yet — append before closing delimiter
        format!("{raw_frontmatter}\n{key}: {new_value}")
    }
}
```

For array fields (email, tags, etc.), the editor needs to handle multi-line YAML sequences. Use a state-machine parser that identifies the field start and collects indented continuation lines.

### Pattern 3: Validation Before Write (CLI-04)
**What:** A `validate_contact()` function that checks all constraints before any disk write.
**When to use:** Called in `store::write_contact` and in the `edit` command before committing changes.
**Example:**
```rust
fn validate_contact(contact: &Contact) -> Result<Vec<ValidationError>> {
    let mut errors = Vec::new();
    if contact.name.is_empty() { errors.push(required_field("name")); }
    if contact.id.is_empty() { errors.push(required_field("id")); }
    // Date format validation (chrono already handles via NaiveDate parsing)
    // Enum validation (serde_yaml already handles via enum deserialization)
    // Cadence format validation
    if !contact.follow_up_cadence.is_empty() {
        parse_cadence(&contact.follow_up_cadence)?; // validates format
    }
    Ok(errors)
}
```

### Pattern 4: ContactFile with Raw Frontmatter
**What:** Extend `ContactFile` to store the raw frontmatter string alongside the parsed `Contact`.
**When to use:** Enables round-trip: read raw string, parse for validation, edit raw string, write raw string back.
**Example:**
```rust
pub struct ContactFile {
    pub contact: Contact,
    pub raw_frontmatter: String,  // NEW: the original YAML text between --- delimiters
    pub body: String,
    pub path: std::path::PathBuf,
}
```

### Anti-Patterns to Avoid
- **Re-serializing YAML through serde_yaml for writes:** This destroys comments and field order. The current `serialize_contact_file` in `store.rs` does this. It must be replaced for CLI-03 compliance.
- **Building every field into the Contact struct manually in add.rs:** Use `Default` implementation or builder pattern instead of 30+ field initialization.
- **Matching contacts by exact name:** Current code uses `contains()` which is correct. Don't switch to exact match -- partial matching is a feature.
- **Silent validation failures:** Never silently drop bad data. Always surface validation errors to the user.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI argument parsing | Custom arg parser | clap 4 derive | Already in use; derive macros handle validation, help, completions |
| JSON serialization | Manual JSON string building | serde_json | Contact already derives Serialize |
| Date arithmetic | Manual day counting | chrono Duration/NaiveDate | Leap years, month boundaries, etc. |
| Interactive confirmation | stdin readline loop | dialoguer Confirm | Handles edge cases (piped input, terminal detection) |
| UUID generation | Random string | uuid v4 | Already in use |
| Colored output | ANSI escape codes | colored crate | Already in use; handles NO_COLOR env var |

**Key insight:** The only custom code needed is the raw-text frontmatter editor (`frontmatter.rs`). Everything else has a standard crate.

## Common Pitfalls

### Pitfall 1: serde_yaml Destroys Comments on Write
**What goes wrong:** Using `serde_yaml::to_string(&contact)` to write back contact files strips all YAML comments (`# Contact`, `# Professional`, etc.) from the frontmatter.
**Why it happens:** YAML spec says comments are not part of the data model. serde_yaml follows spec.
**How to avoid:** Never use serde_yaml for writing. Use the raw-text frontmatter editor for all mutations.
**Warning signs:** After an `acrm edit` or `acrm log`, the contact file loses its section comments.

### Pitfall 2: serde_yaml Reorders Fields
**What goes wrong:** `serde_yaml::to_string` serializes fields in struct definition order, not in the original file order. If struct order differs from template order, the file gets shuffled.
**Why it happens:** Serde serializes by iterating struct fields in declaration order.
**How to avoid:** Same as Pitfall 1 -- use raw-text editing, not re-serialization.
**Warning signs:** Fields appear in different order after a round-trip.

### Pitfall 3: Cadence String Parsing Edge Cases
**What goes wrong:** User enters "bi-weekly", "every 2 weeks", "bimonthly" -- non-standard cadence strings.
**Why it happens:** No standard format for cadence strings.
**How to avoid:** Define a strict enum of supported cadences: "weekly", "biweekly", "monthly", "quarterly", "yearly". Validate on input. Document supported values.
**Warning signs:** `next_follow_up` calculates wrong dates or panics on unrecognized cadence.

### Pitfall 4: Partial Name Matching Ambiguity
**What goes wrong:** `acrm edit "j"` matches "Jane Smith", "John Doe", "Jessica Chen" -- user gets an error about ambiguous matches.
**Why it happens:** The existing `contains()` matching is deliberately loose.
**How to avoid:** This is correct behavior. The error message already lists matches. Keep this pattern for `edit`, `delete`, and `archive` commands.
**Warning signs:** N/A -- this is working as designed.

### Pitfall 5: Archive Directory Not Existing
**What goes wrong:** `acrm archive "Jane"` tries to move file to `archive/` but directory doesn't exist.
**Why it happens:** Archive directory is created on demand, not at project init.
**How to avoid:** Create `archive/` directory if it doesn't exist before moving the file.
**Warning signs:** "No such file or directory" error on first archive operation.

### Pitfall 6: Editing Array Fields with Raw Text
**What goes wrong:** User runs `acrm edit "Jane" --tags "rust,cli"` but tags are a YAML array that spans multiple lines.
**Why it happens:** Array fields in YAML can be either flow (`[a, b]`) or block (`- a\n- b`) style.
**How to avoid:** For the `edit` command, support comma-separated values for array fields and convert to the appropriate YAML format. Detect existing format (flow vs block) and match it.
**Warning signs:** Malformed YAML after editing an array field.

## Code Examples

### Cadence to Duration Calculation (CLI-06)
```rust
use chrono::{NaiveDate, Duration, Months};

fn next_follow_up(from_date: NaiveDate, cadence: &str) -> Result<NaiveDate> {
    match cadence.to_lowercase().as_str() {
        "weekly" => Ok(from_date + Duration::weeks(1)),
        "biweekly" | "bi-weekly" => Ok(from_date + Duration::weeks(2)),
        "monthly" => from_date.checked_add_months(Months::new(1))
            .context("Date overflow"),
        "quarterly" => from_date.checked_add_months(Months::new(3))
            .context("Date overflow"),
        "yearly" | "annually" => from_date.checked_add_months(Months::new(12))
            .context("Date overflow"),
        _ => bail!("Unknown cadence: '{cadence}'. Supported: weekly, biweekly, monthly, quarterly, yearly"),
    }
}
```

### Delete with Confirmation (CLI-05)
```rust
use dialoguer::Confirm;

pub fn run(name: &str, yes: bool) -> Result<()> {
    let root = store::find_crm_root()?;
    let cf = find_single_contact(&root, name)?;

    if !yes {
        let confirmed = Confirm::new()
            .with_prompt(format!("Delete {}? This cannot be undone.", cf.contact.name))
            .default(false)
            .interact()?;
        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    std::fs::remove_file(&cf.path)?;
    println!("Deleted: {}", cf.path.display());
    Ok(())
}
```

### Archive Command (CLI-05)
```rust
pub fn run(name: &str) -> Result<()> {
    let root = store::find_crm_root()?;
    let mut cf = find_single_contact(&root, name)?;

    // Update status in raw frontmatter
    cf.raw_frontmatter = update_field(&cf.raw_frontmatter, "status", "archived");

    // Move to archive directory
    let archive_dir = root.join("archive");
    std::fs::create_dir_all(&archive_dir)?;
    let dest = archive_dir.join(cf.path.file_name().unwrap());

    // Write updated content to archive location
    let content = format!("---\n{}\n---\n\n{}", cf.raw_frontmatter, cf.body);
    std::fs::write(&dest, content)?;
    std::fs::remove_file(&cf.path)?;

    println!("Archived {} -> {}", cf.contact.name, dest.display());
    Ok(())
}
```

### Refactored ContactFile Serialization (CLI-03)
```rust
/// Write contact file preserving original frontmatter formatting.
/// Uses raw_frontmatter (with any field updates applied via update_field)
/// instead of re-serializing through serde_yaml.
pub fn serialize_contact_file_preserving(cf: &ContactFile) -> String {
    format!("---\n{}---\n\n{}", cf.raw_frontmatter, cf.body)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| serde_yaml for read+write | serde_yaml read-only, raw-text for write | This phase | Preserves comments and field order |
| No validation | Validate before every write | This phase | Prevents malformed contact files |
| Human-only output | Dual human/JSON output | This phase | Enables scripting and piping to jq |
| Manual follow-up tracking | Auto-calculated from cadence | This phase | Core CRM automation feature |

**Deprecated/outdated:**
- `serialize_contact_file` in `store.rs`: Must be replaced or augmented with a preserving variant

## Open Questions

1. **Array field editing UX**
   - What we know: Array fields (tags, email, phone) need special handling in `edit` command
   - What's unclear: Should `--tags "rust,cli"` replace all tags or append? Should there be `--add-tag` and `--remove-tag` variants?
   - Recommendation: Use `--field value` for replace semantics. Add `--add-tag` / `--remove-tag` convenience flags. Document behavior clearly.

2. **Archive reversibility**
   - What we know: Success criteria says archive is "reversible"
   - What's unclear: Should there be an `unarchive` command, or is manually moving the file back sufficient?
   - Recommendation: Implement `acrm unarchive "name"` that moves file from `archive/` back to `contacts/` and sets status back to `active`. Low effort, high usability.

3. **Edit command field naming**
   - What we know: Schema has fields like `follow_up_cadence`, `how_we_met` (snake_case with underscores)
   - What's unclear: Should CLI accept `--follow-up-cadence` (kebab) or `--follow_up_cadence` (snake)?
   - Recommendation: Accept `--field key=value` pattern where key matches the YAML field name exactly (snake_case). This avoids mapping complexity and matches the file format.

## Sources

### Primary (HIGH confidence)
- Existing codebase: `src/` directory, `Cargo.toml`, `.schemas/contact.yaml`, `templates/contact.md` -- full review of current implementation
- `contacts/example-jane-smith.md` -- confirms YAML comments in frontmatter that must be preserved

### Secondary (MEDIUM confidence)
- [serde_json on crates.io](https://crates.io/crates/serde_json) - v1.0.149, standard JSON serialization
- [dialoguer on docs.rs](https://docs.rs/dialoguer/latest/dialoguer/) - Interactive CLI prompts (Confirm, Input)
- [clap derive documentation](https://docs.rs/clap/latest/clap/_derive/index.html) - Global args, ValueEnum pattern
- [yamlpatch on docs.rs](https://docs.rs/yamlpatch/latest/yamlpatch/) - v0.12, comment-preserving YAML patches (evaluated but not recommended due to tree-sitter dependency weight)
- [chrono documentation](https://docs.rs/chrono/latest/chrono/) - Months::new() for month-based arithmetic

### Tertiary (LOW confidence)
- [rust-yaml on crates.io](https://crates.io/crates/rust-yaml) - Claims round-trip support but maturity unclear; not recommended

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - existing Cargo.toml defines the stack; new deps are trivial additions
- Architecture: HIGH - extending established patterns (one module per command, shared store)
- Pitfalls: HIGH - round-trip YAML issue verified by reading actual contact files with comments
- Cadence parsing: MEDIUM - chrono's `checked_add_months` is verified, but cadence string format is a design choice

**Research date:** 2026-03-05
**Valid until:** 2026-04-05 (stable Rust ecosystem, no fast-moving dependencies)
