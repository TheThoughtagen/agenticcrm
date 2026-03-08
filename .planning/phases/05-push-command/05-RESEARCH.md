# Phase 5: Push Command - Research

**Researched:** 2026-03-08
**Domain:** CLI command implementation, clap subcommands, push execution wiring
**Confidence:** HIGH

## Summary

Phase 5 is a CLI wiring phase, not an infrastructure phase. All push building blocks exist from Phase 4: `compute_push_changeset` (fully tested, 9 unit tests), `contact_to_vcard`, `merge_contact_to_vcard`, `put_vcard`, `delete_vcard`, and the vCard cache system. The one critical gap is that `execute_push` in `src/sync/push.rs` is a stub returning hardcoded zeros -- this must be implemented as part of this phase.

The work divides into two clear pieces: (1) implement the `execute_push` function body that wires changeset computation to actual CardDAV operations, and (2) add the `acrm sync push` CLI subcommand with `--dry-run` and `--force` flags plus a summary report. The existing `acrm sync` command currently only does pull. The CLI needs restructuring to support `acrm sync pull` (current behavior) and `acrm sync push` (new) as separate subcommands.

**Primary recommendation:** Add `Push` and `Pull` variants to the existing `SyncAction` enum in `main.rs`. Implement `execute_push` to iterate over the changeset, call the appropriate CardDAV methods, update frontmatter and cache, and return a `PushResult`. Create a `PushSyncResult` struct implementing `Display` and `Serialize` for the summary output.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CMD-01 | `acrm sync push` pushes all local changes to iCloud | execute_push implementation + new CLI subcommand wiring to compute_push_changeset -> execute_push pipeline |
| CMD-02 | `acrm sync push --dry-run` previews changes without pushing | compute_push_changeset already returns the full changeset without side effects; dry-run prints changeset details without calling execute_push |
| CMD-03 | `acrm sync push --force` skips conflict checks | execute_push already takes `force: bool` param; when true, conflicts are treated as updates |
| CMD-04 | Push reports summary (X created, Y updated, Z deleted, W conflicts) | PushResult struct already has created/updated/deleted/conflicted/failed counts + details vec; needs Display + Serialize impl for output |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | 4.x | CLI argument parsing with subcommands | Already in use; derive macro for subcommand variants |
| serde | 1.x | JSON serialization for PushResult output | Already in use; `--format json` support |
| anyhow | 1.x | Error handling with context | Already in use throughout codebase |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| uuid | 1.x | Generate UUID for new contact URLs | Already a dependency; used when creating new contacts on iCloud |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| clap subcommands | Positional args | Subcommands are cleaner for push/pull/setup hierarchy and match existing pattern |

**Installation:** No new dependencies needed.

## Architecture Patterns

### Recommended Project Structure
```
src/
  main.rs              # MODIFY: Add Push/Pull to SyncAction enum
  commands/
    sync.rs            # MODIFY: Add run_push() function, refactor run_sync -> run_pull
  sync/
    push.rs            # MODIFY: Implement execute_push body (currently a stub)
    carddav.rs         # EXISTING: put_vcard, delete_vcard (no changes)
    vcard_write.rs     # EXISTING: serialization + cache (no changes)
```

### Pattern 1: CLI Subcommand Extension
**What:** Add `Push` and `Pull` variants to the existing `SyncAction` enum. The bare `acrm sync` (no subcommand) currently maps to pull -- preserve this as default behavior.
**When to use:** Adding the push command.
**Example:**
```rust
// Source: existing main.rs pattern
#[derive(Subcommand)]
enum SyncAction {
    /// Set up iCloud credentials
    Setup,
    /// Pull contacts from iCloud
    Pull,
    /// Push local changes to iCloud
    Push,
}

// In Commands::Sync match:
Commands::Sync { action, force, dry_run } => match action {
    Some(SyncAction::Setup) => commands::sync::run_setup(fmt),
    Some(SyncAction::Push) => commands::sync::run_push(force, dry_run, fmt),
    Some(SyncAction::Pull) | None => commands::sync::run_sync(force, dry_run, fmt),
}
```

### Pattern 2: Execute Push Implementation
**What:** Implement the stub `execute_push` function to iterate over the changeset and call CardDAV methods.
**When to use:** The core of this phase.
**Example:**
```rust
// Source: existing push.rs stub signature
pub fn execute_push(
    client: &CardDavClient,
    addressbook_url: &Url,
    crm_root: &Path,
    changeset: &PushChangeset,
    force: bool,
) -> Result<PushResult> {
    let mut result = PushResult { created: 0, updated: 0, deleted: 0, conflicted: 0, failed: 0, details: vec![] };

    // Creates: contact_to_vcard -> PUT with etag=None -> update frontmatter + cache
    for cf in &changeset.creates {
        let vcard_text = vcard_write::contact_to_vcard(&cf.contact)?;
        let uuid = uuid::Uuid::new_v4().to_string();
        let url = CardDavClient::build_vcard_url(addressbook_url, &uuid)?;
        match client.put_vcard(&url, &vcard_text, None) {
            Ok(new_etag) => {
                // Update frontmatter: source=icloud, source_id=uuid, etag=new_etag
                update_contact_frontmatter(cf, "icloud", &uuid, &new_etag)?;
                vcard_write::write_cached_vcard(crm_root, &uuid, &vcard_text)?;
                result.created += 1;
                result.details.push(PushDetail { name: cf.contact.name.clone(), action: "created".into(), error: None });
            }
            Err(e) => {
                result.failed += 1;
                result.details.push(PushDetail { name: cf.contact.name.clone(), action: "failed".into(), error: Some(e.to_string()) });
            }
        }
    }

    // Updates: merge_contact_to_vcard -> PUT with If-Match -> update cache
    // Deletes: DELETE with If-Match -> remove cache
    // Conflicts: if force, treat as updates; else skip and report
    // ...
    Ok(result)
}
```

### Pattern 3: Push Result Reporting (Display + Serialize)
**What:** Implement `Display` for human-readable summary and `Serialize` for JSON output, matching the existing `SyncResult` pattern.
**When to use:** CMD-04 summary reporting.
**Example:**
```rust
// Source: existing SyncResult Display pattern in commands/sync.rs
impl fmt::Display for PushSyncResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if self.dry_run { "[DRY RUN] " } else { "" };
        write!(f, "{}Push complete: {} created, {} updated, {} deleted, {} conflicts",
            prefix, self.created, self.updated, self.deleted, self.conflicted)?;
        if self.failed > 0 {
            write!(f, ", {} failed", self.failed)?;
        }
        Ok(())
    }
}
```

### Pattern 4: Frontmatter Update After Push
**What:** After a successful PUT for a new contact, update the contact's markdown file to record source=icloud, source_id=UUID, etag=new_etag. After an update, just update the etag.
**When to use:** Every successful create or update.
**Example:**
```rust
// Source: existing update_existing_contact pattern in commands/sync.rs
fn update_contact_frontmatter(cf: &ContactFile, source: &str, source_id: &str, etag: &str) -> Result<()> {
    let mut fm = cf.raw_frontmatter.clone();
    fm = frontmatter::update_field(&fm, "source", source);
    fm = frontmatter::update_field(&fm, "source_id", &format!("\"{}\"", source_id));
    fm = frontmatter::update_field(&fm, "etag", &format!("\"{}\"", etag));
    // ... write back to cf.path
}
```

### Anti-Patterns to Avoid
- **Calling execute_push in dry-run mode:** Dry-run should only compute the changeset and display it. Never call execute_push (which makes network requests) during dry-run.
- **Swallowing errors from individual PUT/DELETE:** Each failed operation should be recorded in PushResult.details with the error message, not silently ignored. The overall push should continue even if individual contacts fail.
- **Forgetting to PROPFIND for missing ETags:** If put_vcard returns an empty ETag string, the caller should do a PROPFIND to fetch the new ETag. Otherwise subsequent pushes will fail conflict detection.
- **Not updating cache after PUT:** The vCard cache must be updated with the serialized text after a successful PUT, or the next push will detect the same contact as changed again.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI subcommand parsing | Manual arg parsing | clap derive macro with `#[derive(Subcommand)]` | Already established pattern in main.rs |
| Output formatting | Manual println branches | `format::output()` with Display + Serialize | Already established pattern for human/json output |
| Frontmatter updates | Manual string manipulation | `frontmatter::update_field()` | Already handles regex matching, field ordering, missing fields |
| Contact file writing | Manual file I/O | `store::serialize_contact_file()` + `std::fs::write` | Preserves comments and field order |

**Key insight:** This phase is almost entirely wiring existing infrastructure together. The only new code is the execute_push body (~80-100 lines), the run_push command handler (~50 lines), and the CLI routing changes (~10 lines).

## Common Pitfalls

### Pitfall 1: Borrow Checker Issues with PushChangeset Iteration
**What goes wrong:** `execute_push` takes `&PushChangeset` but needs to mutate contact files (update frontmatter) during iteration.
**Why it happens:** Rust's borrow checker prevents mutating data you're iterating over.
**How to avoid:** The changeset contains owned `ContactFile` values (cloned during compute). Read the path from the changeset item, then do file I/O independently. Do not try to mutate the changeset itself.
**Warning signs:** Compilation errors about mutable/immutable borrows.

### Pitfall 2: Empty ETag After PUT
**What goes wrong:** iCloud sometimes does not return an ETag in the PUT response. If stored as empty string, next push will skip conflict detection (empty etag comparison).
**Why it happens:** iCloud server behavior is inconsistent.
**How to avoid:** When put_vcard returns empty string, immediately call `client.fetch_vcard_list(addressbook_url)` and find the new ETag for this resource by matching the href. Store that ETag instead.
**Warning signs:** Empty etag field in contact frontmatter after push.

### Pitfall 3: Dry-Run Changeset Display Without Leaking Side Effects
**What goes wrong:** Computing the changeset itself has no side effects (pure comparison), but printing contact details might trigger lazy evaluation of network resources.
**Why it happens:** Confusion between changeset computation and execution.
**How to avoid:** `compute_push_changeset` is already pure -- it only reads local files and compares. Safe to call in dry-run. Just format and display the changeset categories.
**Warning signs:** Network requests during --dry-run.

### Pitfall 4: Race Between Pull Cache and Push
**What goes wrong:** User pulls, edits a contact, pushes. But pull already cached the vCard, so `compute_push_changeset` compares the new CRM data against the pull-cached version. If the serialized form differs, it correctly detects the change.
**Why it happens:** This is actually correct behavior -- the cache captures "what the server last had" and the comparison detects local changes.
**How to avoid:** No action needed -- just understand that this is the expected flow. The cache is populated by pull and updated by push.

### Pitfall 5: Concurrent File Writes During Push
**What goes wrong:** If two contacts map to the same file (shouldn't happen, but edge case), concurrent writes could corrupt the file.
**Why it happens:** File-based CRM with one file per contact should prevent this.
**How to avoid:** The changeset deduplicates by source_id. Each ContactFile has a unique path. No special handling needed.

## Code Examples

### Run Push Command Handler
```rust
// Source: pattern from existing run_sync in commands/sync.rs
pub fn run_push(force: bool, dry_run: bool, fmt: &OutputFormat) -> Result<()> {
    let (apple_id, app_password) = config::load_credentials()
        .context("Run `acrm sync setup` first")?;
    let client = CardDavClient::new(&apple_id, &app_password)?;

    println!("Discovering address book...");
    let addressbook_url = client.discover_address_book()?;

    // Fetch server state for conflict detection
    let server_entries = client.fetch_vcard_list(&addressbook_url)?;

    // Load local contacts
    let crm_root = store::find_crm_root()?;
    let contacts = store::load_all_contacts(&crm_root)?;

    // Compute changeset (pure, no side effects)
    let changeset = push::compute_push_changeset(&crm_root, contacts, &server_entries)?;

    if dry_run {
        // Display what would be pushed
        print_push_preview(&changeset);
        return Ok(());
    }

    // Execute the push
    let result = push::execute_push(&client, &addressbook_url, &crm_root, &changeset, force)?;

    // Display summary
    format::output(&result, fmt)
}
```

### Dry-Run Preview Output
```rust
fn print_push_preview(changeset: &PushChangeset) {
    println!("[DRY RUN] Push preview:");
    for cf in &changeset.creates {
        println!("  + Would create: {}", cf.contact.name);
    }
    for (cf, _) in &changeset.updates {
        println!("  ~ Would update: {}", cf.0.contact.name);
    }
    for (_, _, name) in &changeset.deletes {
        println!("  - Would delete: {}", name);
    }
    for (cf, local, server) in &changeset.conflicts {
        println!("  ! Conflict: {} (local: {}, server: {})", cf.contact.name, local, server);
    }
    let total = changeset.creates.len() + changeset.updates.len()
        + changeset.deletes.len() + changeset.conflicts.len();
    if total == 0 {
        println!("  (no changes to push)");
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| execute_push as stub | Fully implemented execute_push | Phase 5 (this phase) | Enables actual push functionality |
| `acrm sync` = pull only | `acrm sync push` / `acrm sync pull` subcommands | Phase 5 (this phase) | Clear separation of pull vs push |

## Open Questions

1. **ETag refresh after PUT with empty response ETag**
   - What we know: put_vcard returns empty string when iCloud omits ETag from PUT response
   - What's unclear: How frequently this happens in practice
   - Recommendation: Implement PROPFIND fallback to fetch ETag after empty response. Use existing `fetch_vcard_list` and filter by href. LOW risk -- the code path is straightforward.

2. **Default behavior of bare `acrm sync`**
   - What we know: Currently `acrm sync` (no subcommand) runs pull. Phase 6 will make it run pull-then-push (BIDI-01).
   - What's unclear: Whether to keep `acrm sync` as pull-only in Phase 5, or to change it now.
   - Recommendation: Keep `acrm sync` as pull-only in Phase 5. Phase 6 explicitly addresses this (BIDI-01). Changing it now would mix scope.

## Sources

### Primary (HIGH confidence)
- **Existing codebase** (src/sync/push.rs, src/commands/sync.rs, src/main.rs) - All interfaces, patterns, and types verified by reading actual source
- **Phase 4 RESEARCH.md** - Architecture patterns carry forward
- **Phase 4 VERIFICATION.md** - Confirms execute_push is a stub, documents exact gaps

### Secondary (MEDIUM confidence)
- **clap 4.x derive documentation** - Subcommand patterns verified against existing usage in main.rs

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies, all libraries already in use
- Architecture: HIGH - extending existing patterns with clear examples from codebase
- Pitfalls: HIGH - identified through code reading, not speculation
- execute_push implementation: HIGH - all building blocks tested, just wiring needed

**Research date:** 2026-03-08
**Valid until:** 2026-04-08 (stable domain, internal wiring phase)
