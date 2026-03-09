# Phase 10: LinkedIn Import - Research

**Researched:** 2026-03-09
**Domain:** CSV parsing, deduplication, field mapping, CLI subcommand integration
**Confidence:** HIGH

## Summary

Phase 10 adds `acrm import linkedin <file>` to import LinkedIn Connections.csv data into the CRM. The implementation involves CSV parsing, name/email-based deduplication against existing contacts, fill-empty-only merge semantics, change detection reporting, and `--dry-run` / `--format json` support. All decisions about matching strategy, merge behavior, and field mapping are locked in CONTEXT.md.

The codebase already has robust patterns for everything needed: ops layer with typed result structs, frontmatter update functions (scalar and array), store load/write functions, validation, output formatting, and CLI command dispatch. The `csv` crate (BurntSushi's, the de facto standard for Rust CSV) provides serde-based deserialization that maps directly to a LinkedIn row struct. The primary complexity is in deduplication logic (name OR email matching with ambiguity handling) and the fill-empty-only merge behavior with change detection reporting.

**Primary recommendation:** Add the `csv` crate, create `ops/import.rs` with an `import_linkedin()` function returning `ImportResult`, add a new `Import` CLI subcommand, and follow existing patterns exactly (raw frontmatter updates, validation before write, BulkResult-style reporting).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **Dedup matching**: Match by name OR email (case-insensitive exact name, not fuzzy). Ambiguous matches skipped with warning.
- **Change detection**: Fill empty fields only. Detect changes in LinkedIn data and report them but do not apply. Array fields (email) merged with dedup. Auto-add "linkedin" tag.
- **Field mapping**: First Name + Last Name -> name, Email Address -> email array, Company -> company, Position -> role, Connected On -> met_date, source="linkedin", relationship="colleague" for new contacts. No source_id.
- **Output**: Summary counts (created/updated/skipped/warnings), list of created/updated contacts, separate "Detected changes" section, --dry-run flag, --format json support, ops returns ImportResult.

### Claude's Discretion
- CSV parsing library choice (csv crate vs manual)
- LinkedIn CSV date format parsing ("Connected On" format varies by locale)
- Internal ops module structure (add to existing ops/contact.rs or new ops/import.rs)
- How to handle malformed CSV rows (skip with warning vs fail)
- Whether to retire the shell script scripts/import-linkedin.sh after Rust implementation

### Deferred Ideas (OUT OF SCOPE)
None
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LNKD-01 | User can import LinkedIn CSV via `acrm import linkedin <file>` | CLI subcommand pattern from main.rs, ops layer pattern from ops/contact.rs, csv crate for parsing |
| LNKD-02 | Import deduplicates against existing contacts by name and email | store::load_all_contacts() for loading, new name/email matching functions (distinct from sync/dedup.rs source_id matching) |
| LNKD-03 | Re-import detects changes and updates only modified fields | frontmatter::update_field/update_array_field for surgical updates, change detection in ImportResult reporting |
| LNKD-04 | Import maps all available LinkedIn CSV fields to contact schema | Serde deserialization with #[serde(rename)] for LinkedIn headers, field mapping to Contact struct |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| csv | 1.x (latest) | CSV parsing with serde | BurntSushi's csv is THE Rust CSV library. Handles quoting, escaping, flexible delimiters, serde integration |
| serde | 1 (already in project) | Deserialize CSV rows into structs | Already used throughout; csv crate integrates natively |
| chrono | 0.4 (already in project) | Parse "Connected On" dates | Already used for NaiveDate throughout contact model |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| uuid | 1 (already in project) | Generate UUIDs for new contacts | Already in ops::contact::add() |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| csv crate | Manual parsing (like shell script) | Manual parsing breaks on quoted fields with commas, embedded newlines; csv crate handles all edge cases |

**Installation:**
```bash
cargo add csv
```

**Recommendation (Claude's Discretion):** Use the `csv` crate. It handles RFC 4180 edge cases (quoted fields containing commas, embedded newlines, escaped quotes) that the existing shell script does not. It integrates seamlessly with serde for struct deserialization.

## Architecture Patterns

### Recommended Project Structure
```
src/
├── ops/
│   ├── import.rs       # NEW: import_linkedin() ops function
│   ├── contact.rs      # Existing ops (reuse add patterns)
│   ├── error.rs        # Existing OpsError (no changes needed)
│   └── mod.rs          # Add: pub mod import;
├── commands/
│   ├── import.rs       # NEW: CLI handler for `acrm import linkedin`
│   └── mod.rs          # Add: pub mod import;
└── main.rs             # Add: Import subcommand with LinkedIn sub-subcommand
```

**Recommendation (Claude's Discretion):** Create a new `ops/import.rs` rather than adding to `ops/contact.rs`. The import logic is self-contained and substantial enough (CSV parsing, dedup matching, merge logic, change detection) to warrant its own module. This follows the pattern of `ops/sync.rs` being separate from `ops/contact.rs`.

### Pattern 1: LinkedIn CSV Row Struct
**What:** Serde-deserializable struct matching LinkedIn CSV headers
**When to use:** Parsing each CSV row
**Example:**
```rust
#[derive(Debug, serde::Deserialize)]
struct LinkedInRow {
    #[serde(rename = "First Name")]
    first_name: String,
    #[serde(rename = "Last Name")]
    last_name: String,
    #[serde(rename = "Email Address")]
    email_address: String,
    #[serde(rename = "Company")]
    company: String,
    #[serde(rename = "Position")]
    position: String,
    #[serde(rename = "Connected On")]
    connected_on: String,
}
```

### Pattern 2: ImportResult (following BulkResult pattern)
**What:** Typed result struct for import operation
**When to use:** Returned by ops::import::import_linkedin()
**Example:**
```rust
#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub created: Vec<ImportChange>,
    pub updated: Vec<ImportChange>,
    pub skipped: Vec<ImportSkip>,
    pub warnings: Vec<String>,
    pub detected_changes: Vec<DetectedChange>,
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct ImportChange {
    pub name: String,
    pub path: String,
    pub fields: Vec<String>,  // which fields were set
}

#[derive(Debug, Serialize)]
pub struct ImportSkip {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct DetectedChange {
    pub name: String,
    pub field: String,
    pub crm_value: String,
    pub linkedin_value: String,
}
```

### Pattern 3: Dedup Matching (name OR email)
**What:** Find existing contact by case-insensitive exact name match OR email match
**When to use:** For each CSV row, before deciding create vs update
**Key points:**
- Name match: `existing.name.to_lowercase() == csv_name.to_lowercase()` (exact, not contains)
- Email match: any email in existing.email array matches csv email (case-insensitive)
- If BOTH name and email match different contacts, that is ambiguous -> skip with warning
- If multiple contacts match by name (unlikely with exact match), ambiguous -> skip with warning

### Pattern 4: Fill-Empty-Only Merge
**What:** Only fill CRM fields that are empty; never overwrite existing values
**When to use:** When updating an existing contact from LinkedIn data
**Key points:**
- Scalar fields: only set if current value is empty string or None
- Array fields (email): merge -- append new values not already present
- Tags: add "linkedin" if not already present (deduplicate)
- Source: only set if currently empty (don't overwrite "icloud" with "linkedin")
- Change detection: when LinkedIn has a value AND CRM has a DIFFERENT value, record as DetectedChange

### Pattern 5: CSV Note Lines Handling
**What:** LinkedIn CSV sometimes has informational lines before the header row
**When to use:** When opening the CSV file
**Key approach:** The csv crate's `flexible(true)` and `has_headers(true)` settings handle most cases. If the first line doesn't match expected headers, skip lines until finding the header row. Alternatively, use `csv::ReaderBuilder::new().flexible(true)` to handle varying column counts.

**Recommendation (Claude's Discretion):** Handle this robustly by checking if the first record matches expected headers. If note lines are present, they will likely cause a serde deserialization error on the first few rows -- skip those rows with a warning. The `csv` crate's `flexible(true)` mode prevents hard failures on rows with unexpected column counts.

### Pattern 6: CLI Subcommand Nesting
**What:** `acrm import linkedin <file>` uses nested subcommands
**When to use:** main.rs command dispatch
**Example:**
```rust
// In Commands enum:
Import {
    #[command(subcommand)]
    source: ImportSource,
},

// New enum:
#[derive(Subcommand)]
enum ImportSource {
    /// Import from LinkedIn Connections.csv
    LinkedIn {
        /// Path to LinkedIn Connections.csv file
        file: PathBuf,
        /// Preview changes without writing
        #[arg(long)]
        dry_run: bool,
    },
}
```
This follows the existing pattern of `Sync { action: SyncAction }` nesting.

### Anti-Patterns to Avoid
- **Fuzzy name matching for dedup:** Locked decision is case-insensitive EXACT match, not contains/fuzzy. Don't reuse `find_contact()` which uses `.contains()`.
- **Overwriting CRM data:** Fill-empty-only is a locked decision. Never replace non-empty CRM fields with LinkedIn data.
- **Building a custom CSV parser:** The shell script's naive `IFS=','` parsing breaks on quoted fields. Use the csv crate.
- **Modifying ops/contact.rs find_contact():** The import needs different matching semantics (exact name, not contains). Create import-specific matching functions.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CSV parsing | Manual field splitting | `csv` crate | Handles RFC 4180 (quotes, commas in fields, newlines, escaping) |
| UUID generation | Custom IDs | `uuid::Uuid::new_v4()` | Already used in ops::contact::add() |
| Date parsing | Custom parser | `chrono::NaiveDate::parse_from_str()` | Handles multiple date formats |
| Contact file I/O | Direct file writes | `store::write_contact()` / `store::serialize_contact_file()` | Handles validation, slug generation, proper formatting |
| Frontmatter updates | Direct string manipulation | `frontmatter::update_field()` / `update_array_field()` | Preserves comments, field order, YAML structure |

## Common Pitfalls

### Pitfall 1: LinkedIn CSV Note Lines
**What goes wrong:** The file may start with informational text lines before the actual CSV headers
**Why it happens:** LinkedIn sometimes prepends notes about missing email addresses
**How to avoid:** Use flexible mode in csv reader. If deserialization fails on early rows, skip them. Only error if zero valid rows are found.
**Warning signs:** First row fails to deserialize into LinkedInRow struct

### Pitfall 2: "Connected On" Date Format Varies by Locale
**What goes wrong:** Date parsing fails for non-US locales
**Why it happens:** LinkedIn uses locale-dependent date formats: "01 Jan 2024" (UK), "Jan 01, 2024" (US), "2024-01-01" (ISO)
**How to avoid:** Try multiple date format strings with chrono. Common formats: `%d %b %Y`, `%b %d, %Y`, `%Y-%m-%d`, `%m/%d/%y`, `%m/%d/%Y`. If all fail, skip the field with a warning rather than erroring.
**Warning signs:** met_date is None for contacts that should have a Connected On date

### Pitfall 3: Empty Email Column
**What goes wrong:** Most LinkedIn connections won't have an email in the CSV
**Why it happens:** LinkedIn only includes email if the connection enabled sharing
**How to avoid:** Treat empty email as normal (not an error). When email is empty and dedup is needed, fall back to name-only matching (per locked decision).
**Warning signs:** Large number of contacts matched only by name

### Pitfall 4: Duplicate Names Without Email
**What goes wrong:** Two CRM contacts with the same name, CSV row has no email to disambiguate
**Why it happens:** Common names (e.g., "John Smith")
**How to avoid:** Per locked decision, ambiguous matches are skipped with a warning. The user resolves manually.
**Warning signs:** Skipped count is high

### Pitfall 5: Raw Frontmatter Not Loaded for Updates
**What goes wrong:** Updating a contact without loading fresh raw_frontmatter corrupts the file
**Why it happens:** ContactFile.raw_frontmatter must be loaded from disk before applying update_field
**How to avoid:** For each update, re-load the contact file from disk using `store::parse_contact_file()` (same pattern as bulk_update in ops/contact.rs)
**Warning signs:** Comments and field order lost after update

### Pitfall 6: Tag "linkedin" vs "linkedin-import"
**What goes wrong:** Inconsistency with existing shell script which uses "linkedin-import" tag
**Why it happens:** Shell script uses "linkedin-import", CONTEXT.md says "linkedin"
**How to avoid:** Use "linkedin" per CONTEXT.md locked decision. This supersedes the shell script.
**Warning signs:** Contacts have both tags after mixed imports

## Code Examples

### CSV Reader Setup
```rust
// Source: csv crate docs + project patterns
use std::path::Path;

fn read_linkedin_csv(path: &Path) -> Result<Vec<LinkedInRow>, OpsError> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| OpsError::Internal(format!("Failed to open CSV: {e}")))?;

    let mut rows = Vec::new();
    let mut warnings = Vec::new();

    for (i, result) in rdr.deserialize().enumerate() {
        match result {
            Ok(row) => rows.push(row),
            Err(e) => warnings.push(format!("Row {}: {e}", i + 2)),
        }
    }

    Ok(rows)
}
```

### Name/Email Dedup Matching
```rust
// Source: project pattern from sync/dedup.rs + CONTEXT.md decisions
fn find_existing_contact<'a>(
    contacts: &'a [ContactFile],
    name: &str,
    email: &str,
) -> Result<Option<&'a ContactFile>, ImportMatchError> {
    let name_lower = name.to_lowercase();
    let email_lower = email.to_lowercase();

    let name_matches: Vec<_> = contacts.iter()
        .filter(|cf| cf.contact.name.to_lowercase() == name_lower)
        .collect();

    let email_matches: Vec<_> = if !email_lower.is_empty() {
        contacts.iter()
            .filter(|cf| cf.contact.email.iter()
                .any(|e| e.to_lowercase() == email_lower))
            .collect()
    } else {
        vec![]
    };

    // Combine matches, check for ambiguity
    // ... (see implementation notes in Architecture Patterns)
}
```

### Fill-Empty-Only Field Update
```rust
// Source: project pattern from frontmatter.rs + ops/contact.rs
fn fill_if_empty(raw_fm: &str, key: &str, value: &str) -> (String, bool) {
    // Check if existing value is empty
    let pattern = format!(r"(?m)^{}:\s*(.*)$", regex::escape(key));
    let re = regex::Regex::new(&pattern).unwrap();
    if let Some(caps) = re.captures(raw_fm) {
        let existing = caps[1].trim().trim_matches('"');
        if !existing.is_empty() {
            return (raw_fm.to_string(), false); // not updated
        }
    }
    // Value is empty or field doesn't exist -- fill it
    let updated = frontmatter::update_field(raw_fm, key, value);
    (updated, true)
}
```

### Date Parsing with Multiple Formats
```rust
// Source: chrono docs + LinkedIn locale observations
fn parse_connected_on(date_str: &str) -> Option<chrono::NaiveDate> {
    let formats = [
        "%d %b %Y",    // "01 Jan 2024" (UK/international)
        "%b %d, %Y",   // "Jan 01, 2024" (US)
        "%Y-%m-%d",    // "2024-01-01" (ISO)
        "%m/%d/%Y",    // "01/01/2024" (US numeric)
        "%m/%d/%y",    // "01/01/24" (US short year)
    ];
    let trimmed = date_str.trim();
    for fmt in &formats {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(trimmed, fmt) {
            return Some(d);
        }
    }
    None
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Shell script import (scripts/import-linkedin.sh) | Rust ops import via `acrm import linkedin` | Phase 10 | Proper CSV parsing, dedup, change detection, --dry-run |
| source_id-based dedup (sync/dedup.rs) | Name/email-based dedup for LinkedIn | Phase 10 | LinkedIn has no stable UID; different matching strategy needed |

**Recommendation (Claude's Discretion):** Keep `scripts/import-linkedin.sh` but add a deprecation note at the top pointing users to `acrm import linkedin`. Don't delete it -- users may have referenced it, and it serves as documentation of the original approach.

## Open Questions

1. **LinkedIn CSV note lines before headers**
   - What we know: Some exports have informational lines before the actual CSV header row
   - What's unclear: Exact format and content of these note lines; whether they are consistent across locales
   - Recommendation: Use csv crate's flexible mode. If the first few rows fail deserialization, skip with warning. If all rows fail, return error. This handles both with-notes and without-notes CSVs.

2. **Malformed CSV rows**
   - What we know: Some rows may have unexpected column counts or invalid data
   - Recommendation (Claude's Discretion): Skip malformed rows with a warning rather than failing the entire import. Count them in the warnings list. This is more user-friendly for large imports.

## Sources

### Primary (HIGH confidence)
- Codebase inspection: ops/contact.rs, frontmatter.rs, store.rs, main.rs, sync/dedup.rs, models/contact.rs -- all patterns verified from source
- Codebase inspection: scripts/import-linkedin.sh -- existing LinkedIn import behavior and field mapping
- CONTEXT.md -- locked decisions on matching, merge, field mapping, and output

### Secondary (MEDIUM confidence)
- [csv crate docs](https://docs.rs/csv) -- serde integration, flexible mode, trim options
- [csv crate GitHub](https://github.com/BurntSushi/rust-csv) -- current version, API stability
- LinkedIn CSV format -- columns confirmed as First Name, Last Name, Email Address, Company, Position, Connected On via multiple sources and existing shell script

### Tertiary (LOW confidence)
- LinkedIn "Connected On" date format -- varies by locale, multiple format strings needed. Exact set of formats is based on community reports, not official docs.
- LinkedIn note lines before CSV headers -- reported by some users but not officially documented

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - csv crate is the only real option for Rust CSV; all other deps already in project
- Architecture: HIGH - follows established ops/commands patterns exactly; no novel architecture needed
- Pitfalls: MEDIUM - LinkedIn CSV format quirks (date formats, note lines) based on community knowledge, not official docs

**Research date:** 2026-03-09
**Valid until:** 2026-04-09 (stable domain, LinkedIn CSV format rarely changes)
