# Phase 10: LinkedIn Import - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Import LinkedIn connection CSV data into the CRM via `acrm import linkedin <file>`. Supports intelligent deduplication against existing contacts, change detection on re-import, and field mapping from LinkedIn CSV columns to the contact schema. Playwright automation and auto-export are separate (future LNKD-05/06).

</domain>

<decisions>
## Implementation Decisions

### Dedup Matching Strategy
- Match by name OR email — if either matches an existing contact, treat as the same person
- Name matching is case-insensitive exact (not fuzzy/contains)
- When CSV row has no email, fall back to name-only matching
- Ambiguous matches (multiple contacts match) are skipped with a warning — user resolves manually
- Consistent with existing ops `AmbiguousMatch` error pattern

### Change Detection & Merge Behavior
- Fill empty fields only — LinkedIn data never overwrites existing CRM data
- Detect changes in LinkedIn data (e.g., new company) and report them, but do not apply
- Array fields (email) are merged: append new values, keep existing, deduplicate within array
- Automatically add "linkedin" tag to all imported contacts (deduplicated if already present)

### Field Mapping
- `First Name` + `Last Name` → `name` (combined as "First Last")
- `Email Address` → `email` array (merged with existing)
- `Company` → `company` (fill empty only)
- `Position` → `role` (fill empty only)
- `Connected On` → `met_date` (fill empty only)
- `source` set to "linkedin" for new contacts (don't overwrite if already set, e.g., "icloud")
- No `source_id` — LinkedIn CSV has no stable unique identifier
- Default `relationship` to "colleague" for new contacts

### Output & Reporting
- Summary counts: created, updated, skipped, warnings
- List contacts that were created or updated (not skipped)
- Separate "Detected changes" section showing field changes not applied (e.g., "John Smith: company Acme → Globex")
- `--dry-run` flag: show what would happen without writing
- `--format json` support: consistent with all other CLI commands
- Result struct pattern: ops returns `ImportResult`, CLI formats it

### Claude's Discretion
- CSV parsing library choice (csv crate vs manual)
- LinkedIn CSV date format parsing ("Connected On" format varies by locale)
- Internal ops module structure (add to existing ops/contact.rs or new ops/import.rs)
- How to handle malformed CSV rows (skip with warning vs fail)
- Whether to retire the shell script `scripts/import-linkedin.sh` after Rust implementation

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Key principle: follow existing ops patterns (typed result structs, OpsError, CLI as thin wrapper). The import should feel like a natural extension of the existing `acrm` CLI.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ops::contact::add()`: Creates new contacts with UUID generation — import can reuse for new contacts
- `frontmatter::update_field()` / `update_array_field()`: Surgical field updates for existing contacts
- `store::load_all_contacts()`: Load all contacts for dedup matching
- `sync/dedup.rs`: Pattern for dedup functions (source_id-based) — LinkedIn will add name/email-based variants
- `models::Contact`: Full contact struct with all schema fields
- `validation::validate_contact()`: Validate before writing

### Established Patterns
- Ops layer returns typed result structs (AddResult, SyncPullResult, BulkResult)
- CLI commands support `--format json` via OutputFormat enum
- Bulk ops have `--dry-run` and `--yes` flags
- Raw frontmatter preservation through update_field pattern
- Two-phase operations (preview then apply) established in bulk ops and delete

### Integration Points
- `main.rs` command dispatch: add `import` subcommand with `linkedin` sub-subcommand
- `ops/` module: new import functions alongside existing contact/sync ops
- `scripts/import-linkedin.sh`: existing shell implementation to be superseded

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 10-linkedin-import*
*Context gathered: 2026-03-09*
