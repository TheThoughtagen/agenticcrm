# Phase 6: Selective Sync & Bidirectional - Research

**Researched:** 2026-03-08
**Domain:** CLI filtering, config file extension, command routing
**Confidence:** HIGH

## Summary

Phase 6 adds two features to the existing sync infrastructure: (1) selective filtering of contacts by tag and status during sync operations, and (2) a unified `acrm sync` command that performs pull-then-push bidirectionally. Both features build directly on the existing Phase 4/5 push/pull infrastructure with no new external dependencies.

The filtering feature requires extending the `sync.toml` config file with filter sections, adding `--tag` and `--status` CLI flags to sync commands, and inserting a filter step between contact loading and changeset computation. The bidirectional sync command requires re-routing `acrm sync` (currently an alias for pull) to perform pull-then-push sequentially.

**Primary recommendation:** Extend `sync.toml` with TOML filter sections, add a shared `SyncFilter` struct used by both pull and push paths, and reroute the `Sync { action: None }` match arm to call pull-then-push.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FILT-01 | User can configure push tag/status filters in sync config | Extend sync.toml with `[push_filters]` section; parse in config.rs |
| FILT-02 | User can configure pull tag/status filters in sync config | Extend sync.toml with `[pull_filters]` section; parse in config.rs |
| FILT-03 | User can override filters via `--tag` and `--status` CLI flags | Add clap args to Sync, Pull, Push variants; merge with config filters |
| FILT-04 | Default (no filters) syncs everything | SyncFilter::default() returns empty/pass-all; filter logic is no-op when empty |
| BIDI-01 | `acrm sync` performs pull-then-push in one command | Change `None` match arm in main.rs to call run_sync then run_push |
| BIDI-02 | User can still run `acrm sync pull` and `acrm sync push` separately | Already works -- preserve existing subcommand routing |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4 (derive) | CLI argument parsing | Already in use; `--tag` and `--status` flags use standard derive patterns |
| serde | 1 | Config deserialization | Already in use; needed for TOML filter config |

### Supporting
No new dependencies needed. The existing hand-rolled TOML parser in `config.rs` is sufficient for the simple key-value additions, though a proper TOML parser would be cleaner.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled TOML parsing | `toml` crate | Cleaner but adds a dependency for 4 simple key-value pairs; hand-rolled is consistent with existing config.rs |

**No new installation needed.**

## Architecture Patterns

### Current Architecture (key files)
```
src/
├── main.rs              # CLI definition (Cli, Commands, SyncAction enums)
├── commands/sync.rs     # run_sync (pull), run_push, run_setup
├── sync/
│   ├── config.rs        # SyncConfig, credential storage, sync.toml parsing
│   ├── push.rs          # compute_push_changeset, execute_push
│   ├── dedup.rs         # find_existing_by_source_id, should_update
│   └── ...
└── models/contact.rs    # Contact struct with tags: Vec<String>, status: Option<Status>
```

### Pattern 1: SyncFilter Struct
**What:** A shared filter struct that both pull and push paths consume
**When to use:** Every sync operation
**Example:**
```rust
/// Filters applied to sync operations. Empty vectors mean "match all".
#[derive(Debug, Clone, Default)]
pub struct SyncFilter {
    pub tags: Vec<String>,
    pub statuses: Vec<String>,
}

impl SyncFilter {
    /// Returns true if the contact passes the filter (or filter is empty).
    pub fn matches(&self, contact: &Contact) -> bool {
        let tag_ok = self.tags.is_empty()
            || contact.tags.iter().any(|t| self.tags.contains(t));
        let status_ok = self.statuses.is_empty()
            || contact.status.as_ref().map_or(false, |s| {
                let s_str = format!("{:?}", s).to_lowercase().replace('_', "-");
                self.statuses.contains(&s_str)
            });
        tag_ok && status_ok
    }

    /// Merge CLI overrides with config filters. CLI overrides replace config (not union).
    pub fn from_config_and_cli(
        config_tags: &[String],
        config_statuses: &[String],
        cli_tags: &[String],
        cli_statuses: &[String],
    ) -> Self {
        SyncFilter {
            tags: if cli_tags.is_empty() { config_tags.to_vec() } else { cli_tags.to_vec() },
            statuses: if cli_statuses.is_empty() { config_statuses.to_vec() } else { cli_statuses.to_vec() },
        }
    }
}
```

### Pattern 2: Config File Extension
**What:** Extend sync.toml with filter sections
**When to use:** When user wants persistent filters
**Example config format:**
```toml
apple_id = "user@icloud.com"

[push_filters]
tags = ["work", "vip"]
statuses = ["active"]

[pull_filters]
tags = []
statuses = ["active", "dormant"]
```

### Pattern 3: Filter Insertion Points
**What:** Where filters apply in the sync pipeline
**When to use:** Understanding the data flow

For **push**: Filter contacts AFTER `store::load_all_contacts()` but BEFORE `compute_push_changeset()`. This is the simplest insertion point -- just `contacts.retain(|cf| filter.matches(&cf.contact))`.

For **pull**: Filter AFTER vCard-to-Contact mapping but BEFORE creating/updating the local file. The pull loop in `run_sync()` maps each vCard to a Contact, then decides create/update/unchanged. Insert filter check right after mapping: if the mapped contact doesn't pass the filter, skip it.

For **bidirectional** (`acrm sync`): Apply pull filters to the pull phase, then push filters to the push phase. These are independent filter sets.

### Pattern 4: Bidirectional Command Routing
**What:** `acrm sync` (no subcommand) becomes pull-then-push
**When to use:** BIDI-01 implementation
**Example:**
```rust
// In main.rs match arm:
None => {
    // Bidirectional: pull then push
    commands::sync::run_sync(force, dry_run, &filter_pull, fmt)?;
    commands::sync::run_push(force, dry_run, &filter_push, fmt)?;
    Ok(())
}
```

**Key decision:** The existing `None` arm currently calls `run_sync` (pull only). This is a behavior change that BIDI-01 explicitly requires.

### Pattern 5: CLI Flag Design
**What:** How `--tag` and `--status` flags work with subcommands
**Current pattern in main.rs:** Flags on parent `Sync` and on `Push`/`Pull` subcommands, merged via OR (e.g., `force || f`). Follow the same pattern for `--tag` and `--status`.
**Example:**
```rust
Sync {
    #[command(subcommand)]
    action: Option<SyncAction>,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    tag: Vec<String>,      // NEW
    #[arg(long)]
    status: Vec<String>,   // NEW
}
```
And similarly on `Pull` and `Push` subcommands. Merge logic: CLI flags from either level override config. If specified at both parent and subcommand, union them.

### Anti-Patterns to Avoid
- **Filtering by modifying the server query:** Never filter at the CardDAV PROPFIND level. Always fetch all from server, filter locally. Server-side filtering adds complexity and CardDAV PROPFIND filters are inconsistent across servers.
- **Applying push filters to archive/delete detection:** Archived contacts with source=icloud should still be deleted from server regardless of tag/status filters. Only filter active contacts for push.
- **Mutating Contact structs for filtering:** Use `retain()` on the Vec, don't modify contact data.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML parsing (if full sections needed) | Manual string parsing for sections | Hand-rolled line parser extended OR `toml` crate | Current parser only handles `key = "value"` -- sections like `[push_filters]` need section awareness |
| Status string matching | Ad-hoc string comparison | Centralized match on Status enum variants | Status uses kebab-case serde, need consistent matching |

**Key insight:** The existing hand-rolled TOML parser in `config.rs` only handles flat `key = "value"` lines. Adding `[section]` support and array values (`tags = ["a", "b"]`) significantly increases complexity. Two options:
1. Add the `toml` crate (small, well-maintained, serde-compatible) -- cleaner
2. Extend hand-rolled parser with section tracking -- consistent with existing code but fragile

**Recommendation:** Add the `toml` crate. It's ~30KB, serde-compatible, and avoids bugs in hand-rolled section/array parsing. Migrate the existing `apple_id` parsing to use it too.

## Common Pitfalls

### Pitfall 1: Pull Filter on Unmapped Contacts
**What goes wrong:** During pull, the vCard is fetched and parsed before we know whether the resulting Contact matches the filter. Wasted HTTP requests.
**Why it happens:** Filter criteria (tags, status) only exist after mapping vCard to Contact.
**How to avoid:** Accept the overhead -- at personal CRM scale (hundreds of contacts), fetching all vCards and filtering locally is fine. The alternative (server-side filtering) is not reliable with iCloud CardDAV.
**Warning signs:** N/A at personal scale.

### Pitfall 2: Status Enum Serialization Mismatch
**What goes wrong:** Status enum uses `#[serde(rename_all = "kebab-case")]` so `LostTouch` serializes as `"lost-touch"`. CLI flag `--status active` must match the kebab-case format.
**Why it happens:** Rust enum variants are PascalCase, YAML/config values are kebab-case.
**How to avoid:** Normalize CLI input to lowercase and compare against kebab-case serialized forms. Document accepted values: `active`, `dormant`, `lost-touch`, `archived`.
**Warning signs:** `--status LostTouch` silently matches nothing.

### Pitfall 3: Filter Interaction with Archive Detection
**What goes wrong:** Push filters exclude an archived contact, so it doesn't get deleted from the server.
**Why it happens:** The filter runs before archive detection in `compute_push_changeset`.
**How to avoid:** Apply push filters only to creates and updates, NOT to archive/delete detection. Archived contacts from `archive/` directory should always be processed for server deletion.
**Warning signs:** Archived contacts persist on iCloud after push with filters.

### Pitfall 4: Bidirectional Sync with Conflicting Filters
**What goes wrong:** Push filter includes tag "work" but pull filter excludes it. Contact pulled without "work" tag, then push skips it.
**Why it happens:** Asymmetric filters create logical gaps.
**How to avoid:** This is a valid user configuration (e.g., "only push work contacts to iCloud but pull everything"). Document that asymmetric filters are intentional. No code prevention needed.

### Pitfall 5: Empty Filter = All Contacts (FILT-04)
**What goes wrong:** Code accidentally treats empty filter list as "match nothing" instead of "match everything."
**Why it happens:** Naive implementation checks `filter.contains(tag)` on empty list, which returns false.
**How to avoid:** Guard: `if self.tags.is_empty() { return true; }` before checking membership. This is the FILT-04 requirement.

### Pitfall 6: Pull Creates Contacts That Don't Match Filter
**What goes wrong:** User configures pull filter for `tags = ["work"]`. Server contact has no tags (tags come from CRM, not vCard). Contact is skipped forever.
**Why it happens:** vCards from iCloud typically don't have tags/categories.
**How to avoid:** For pull, filtering by tag only makes sense for contacts that already exist locally (updates). New contacts from server won't have CRM tags. Consider: pull filter only applies to updates, not creates. OR: document that pull tag filters only work on contacts already in CRM.
**Warning signs:** New server contacts silently ignored during filtered pull.

## Code Examples

### Filter Application in Push Path
```rust
// In run_push(), after loading contacts:
let crm_root = store::find_crm_root()?;
let mut contacts = store::load_all_contacts(&crm_root)?;

// Apply push filter (FILT-01, FILT-03)
if !filter.is_empty() {
    contacts.retain(|cf| filter.matches(&cf.contact));
}

// Compute changeset with filtered contacts
let changeset = push::compute_push_changeset(&crm_root, contacts, &server_entries)?;
```

### Filter Application in Pull Path
```rust
// In run_sync() pull loop, after mapping vCard to Contact:
let mapped = match vcard_map::map_vcard_to_contact(&vcard_text, &uid, &entry.etag) {
    Ok(m) => m,
    Err(e) => { /* ... */ continue; }
};

// Apply pull filter (FILT-02, FILT-03)
// Note: for NEW contacts, tags/status may be empty -- decide policy
if !filter.is_empty() && !filter.matches(&mapped.contact) {
    // Skip contacts that don't match pull filter
    continue;
}
```

### Extended Config Parsing (with toml crate)
```rust
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct SyncConfig {
    pub apple_id: String,
    #[serde(default)]
    pub push_filters: FilterConfig,
    #[serde(default)]
    pub pull_filters: FilterConfig,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FilterConfig {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub statuses: Vec<String>,
}
```

### Bidirectional Sync Command
```rust
// In run_bidi() or inline in main.rs:
pub fn run_bidi(force: bool, dry_run: bool, fmt: &OutputFormat) -> Result<()> {
    println!("Starting bidirectional sync...");

    // Phase 1: Pull
    println!("\n--- Pull ---");
    run_sync(force, dry_run, fmt)?;

    // Phase 2: Push
    println!("\n--- Push ---");
    run_push(force, dry_run, fmt)?;

    println!("\nBidirectional sync complete.");
    Ok(())
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `acrm sync` = pull only | `acrm sync` = pull-then-push | Phase 6 | Breaking behavior change for users who relied on `sync` = pull |
| No filtering | Tag/status filters in config + CLI | Phase 6 | All sync operations can be scoped |
| Hand-rolled TOML (flat) | `toml` crate (structured) | Phase 6 | Config gains sections, arrays, proper parsing |

## Open Questions

1. **Pull filter behavior for new contacts**
   - What we know: New contacts from iCloud won't have CRM-specific tags or status fields. vCards don't carry CRM metadata.
   - What's unclear: Should pull filters apply to new contacts (which would skip most of them) or only to updates?
   - Recommendation: Pull tag filters should only apply to contacts that already exist locally. New contacts always come through (they can be filtered on next sync after user tags them). Pull status filters skip new contacts too (no status = unfiltered). Document this.

2. **Whether to add `toml` crate or extend hand-rolled parser**
   - What we know: Current parser handles only `apple_id = "value"`. Need sections and arrays.
   - What's unclear: Project preference for minimal deps vs. clean code.
   - Recommendation: Add `toml` crate. It's 1 dependency, serde-compatible, and avoids bugs. The hand-rolled parser would need ~50 lines of section/array parsing that the `toml` crate handles perfectly.

3. **Filter merge semantics: CLI override vs. union**
   - What we know: Existing pattern for `--force` and `--dry-run` is OR (either level triggers it).
   - What's unclear: Should `--tag work` on CLI add to config tags or replace them?
   - Recommendation: CLI **replaces** config filters (not union). Rationale: `--tag work` means "I want only work contacts right now" regardless of config. This matches grep/find CLI conventions where flags override defaults.

## Sources

### Primary (HIGH confidence)
- Project codebase: `src/main.rs`, `src/commands/sync.rs`, `src/sync/config.rs`, `src/sync/push.rs`, `src/models/contact.rs` -- full review of existing architecture
- `.planning/REQUIREMENTS.md` -- FILT-01 through FILT-04, BIDI-01, BIDI-02 definitions
- `.planning/STATE.md` -- accumulated decisions from v1.0 and v1.1

### Secondary (MEDIUM confidence)
- clap derive documentation -- flag patterns for Vec<String> args are standard
- `toml` crate -- well-known Rust ecosystem crate, serde-compatible

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies needed (except optional `toml` crate), all patterns exist in codebase
- Architecture: HIGH - filter insertion points are clear from code review, bidirectional routing is a simple match arm change
- Pitfalls: HIGH - identified from direct code analysis (Status enum serialization, archive/filter interaction, empty filter semantics)

**Research date:** 2026-03-08
**Valid until:** 2026-04-08 (stable domain, no external API changes)
