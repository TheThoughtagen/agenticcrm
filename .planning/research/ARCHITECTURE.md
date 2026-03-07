# Architecture Patterns

**Domain:** Two-way iCloud CardDAV sync -- push, conflict detection, selective filtering
**Researched:** 2026-03-07
**Confidence:** HIGH (RFC 6352 verified, existing codebase analyzed, calcard API confirmed)

## Current Architecture (v1.0)

```
CLI (main.rs)
  |
  v
commands/sync.rs        -- orchestrates pull sync flow
  |
  +-- sync/config.rs    -- credentials (keychain + TOML)
  +-- sync/carddav.rs   -- CardDavClient: PROPFIND, GET only
  +-- sync/vcard_map.rs -- vCard->Contact (one direction only)
  +-- sync/dedup.rs     -- find_existing_by_source_id, should_update
  +-- store.rs          -- file I/O, ContactFile parse/serialize/write
  +-- frontmatter.rs    -- raw YAML manipulation preserving comments
```

**Data flow (pull only):**
```
iCloud --PROPFIND--> vcard list --GET--> vcard text --vcard_map--> Contact --store--> .md file
```

## Recommended Architecture (v1.1)

### New and Modified Components

| Component | Status | Purpose |
|-----------|--------|---------|
| `sync/carddav.rs` | **MODIFY** | Add `put_vcard()` and `delete_vcard()` methods to `CardDavClient` |
| `sync/vcard_map.rs` | **MODIFY** | Add `map_contact_to_vcard()` (reverse direction) |
| `sync/push.rs` | **NEW** | Push sync orchestration: diff detection, push loop, result reporting |
| `sync/filter.rs` | **NEW** | Selective sync filter logic (tag/status predicates) |
| `commands/sync.rs` | **MODIFY** | Add `run_push()`, wire up `acrm sync push` subcommand, apply filters to pull |
| `main.rs` | **MODIFY** | Add `Push` variant to `SyncAction` enum |
| `sync/config.rs` | **MODIFY** | Add `auto_push` and filter config parsing from `sync.toml` |
| `sync/mod.rs` | **MODIFY** | Export new `push` and `filter` modules |

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `CardDavClient` | HTTP transport only (PROPFIND, GET, PUT, DELETE) | iCloud server |
| `vcard_map` | Bidirectional mapping: `Contact <-> VCard` | `calcard` crate |
| `push` | Push orchestration: load locals, compare ETags, decide actions, call client | `carddav`, `vcard_map`, `store`, `dedup`, `filter` |
| `filter` | Predicate functions for tag/status filtering | `models::Contact` |
| `dedup` | Source ID matching, ETag comparison (unchanged) | `models::ContactFile` |
| `store` | File I/O (unchanged) | filesystem |
| `config` | Credentials + sync settings | filesystem, keychain |

## Data Flow

### Push Flow (new)

```
store::load_all_contacts()
  |
  v
filter::matches_push_filter(contact, config)  -- selective sync
  |
  v
For each pushable contact:
  |
  +-- No source_id? --> NEW: generate UID, map_contact_to_vcard(), PUT (If-None-Match: *)
  |                      --> Store returned ETag + source_id in frontmatter
  |
  +-- Has source_id + etag? --> PROPFIND to get server ETag
  |     |
  |     +-- Server ETag == local ETag? --> Local changed, server unchanged
  |     |     --> map_contact_to_vcard(), PUT with If-Match: local_etag
  |     |     --> Update stored ETag from response
  |     |
  |     +-- Server ETag != local ETag? --> CONFLICT
  |           --> Warn user, CRM wins (per project constraint)
  |           --> PUT with If-Match: * (force overwrite)
  |           --> Update stored ETag from response
  |
  +-- Status == Archived AND source == "icloud"? --> DELETE with If-Match
        --> Clear source_id and etag from frontmatter
```

### Pull Flow (modified)

```
Existing pull flow, with one addition:
  |
  v
filter::matches_pull_filter(mapped_contact, config)  -- selective sync
  |
  +-- Passes filter? --> proceed as before (create/update/skip)
  +-- Fails filter? --> skip this contact
```

### Conflict Detection Detail

The conflict detection model uses ETags per RFC 6352:

1. **No conflict (common case):** Local ETag matches server ETag. Local has been modified since last sync. PUT with `If-Match: "local_etag"` succeeds with 200/204. Store new ETag from response.

2. **Conflict detected:** Local ETag does not match server ETag (both sides changed). Per project constraint "CRM wins on all sync conflicts": warn the user, then PUT with `If-Match: *` to force overwrite. Store new ETag.

3. **Stale local (no local changes):** Detected by comparing ETag values. If the local ETag matches the server ETag, the contact is unchanged on both sides -- skip it. If the server ETag differs but the local contact has not been modified since last pull, next pull will update it. For MVP, always push if ETag differs and let CRM-wins rule apply.

4. **Server returns 412 Precondition Failed:** ETag changed between PROPFIND check and PUT. Retry once with fresh ETag check, then force-overwrite if still conflicting.

## Patterns to Follow

### Pattern 1: CardDAV PUT with ETag (RFC 6352 Section 6.3.2)

**What:** Create or update a contact on the server using HTTP PUT with conditional headers.
**When:** Pushing a contact to iCloud.

```rust
// In CardDavClient -- new method
pub fn put_vcard(
    &self,
    vcard_url: &Url,
    vcard_body: &str,
    etag: Option<&str>,  // None = new contact, Some = update
) -> Result<PutResult> {
    let method = Method::PUT;
    let mut request = self.client
        .request(method, vcard_url.as_str())
        .header("Content-Type", "text/vcard; charset=utf-8")
        .basic_auth(&self.apple_id, Some(&self.app_password))
        .body(vcard_body.to_string());

    match etag {
        Some(etag) => {
            // Update existing -- server rejects if ETag changed
            request = request.header("If-Match", format!("\"{}\"", etag));
        }
        None => {
            // Create new -- server rejects if resource already exists
            request = request.header("If-None-Match", "*");
        }
    }

    let response = request.send()?;
    match response.status().as_u16() {
        200 | 201 | 204 => {
            let new_etag = response.headers()
                .get("ETag")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim_matches('"').to_string());
            Ok(PutResult { new_etag, conflict: false })
        }
        412 => Ok(PutResult { new_etag: None, conflict: true }),
        status => bail!("PUT failed with status {}", status),
    }
}
```

**Key iCloud behavior:** iCloud may not return an ETag in the PUT response. If absent, immediately do a HEAD or PROPFIND on the single resource URL to retrieve the new ETag.

### Pattern 2: Contact-to-VCard Serialization via calcard

**What:** Build a vCard 3.0 string from a `Contact` struct using calcard's builder API.
**When:** Before every PUT to iCloud.

calcard's `VCard` struct has a public `entries: Vec<VCardEntry>` field. `VCardEntry::new(prop).with_value(val)` provides a fluent builder. Serialization is via `vcard.to_string()` (Display impl).

```rust
// In vcard_map.rs -- new function
pub fn map_contact_to_vcard(contact: &Contact, uid: &str, notes: &str) -> String {
    let mut vcard = VCard::default();

    // FN (formatted name)
    vcard.entries.push(
        VCardEntry::new(VCardProperty::Fn)
            .with_value(VCardValue::Text(contact.name.clone()))
    );

    // N (structured name: Family;Given;;;)
    let (given, family) = split_name(&contact.name);
    vcard.entries.push(
        VCardEntry::new(VCardProperty::N)
            .with_values(vec![
                VCardValue::Text(family),
                VCardValue::Text(given),
                VCardValue::Text(String::new()),
                VCardValue::Text(String::new()),
                VCardValue::Text(String::new()),
            ])
    );

    // UID
    vcard.entries.push(
        VCardEntry::new(VCardProperty::Uid)
            .with_value(VCardValue::Text(uid.to_string()))
    );

    // EMAIL (one entry per address)
    for email in &contact.email {
        vcard.entries.push(
            VCardEntry::new(VCardProperty::Email)
                .with_value(VCardValue::Text(email.clone()))
        );
    }

    // TEL (one entry per number)
    for phone in &contact.phone {
        vcard.entries.push(
            VCardEntry::new(VCardProperty::Tel)
                .with_value(VCardValue::Text(phone.clone()))
        );
    }

    // ORG
    if !contact.company.is_empty() {
        vcard.entries.push(
            VCardEntry::new(VCardProperty::Org)
                .with_value(VCardValue::Text(contact.company.clone()))
        );
    }

    // TITLE
    if !contact.role.is_empty() {
        vcard.entries.push(
            VCardEntry::new(VCardProperty::Title)
                .with_value(VCardValue::Text(contact.role.clone()))
        );
    }

    // URL
    if !contact.website.is_empty() {
        vcard.entries.push(
            VCardEntry::new(VCardProperty::Url)
                .with_value(VCardValue::Text(contact.website.clone()))
        );
    }

    // BDAY
    if let Some(bday) = contact.birthday {
        vcard.entries.push(
            VCardEntry::new(VCardProperty::Bday)
                .with_value(VCardValue::Text(bday.format("%Y-%m-%d").to_string()))
        );
    }

    // NOTE (from markdown body, not frontmatter)
    if !notes.is_empty() {
        vcard.entries.push(
            VCardEntry::new(VCardProperty::Note)
                .with_value(VCardValue::Text(notes.to_string()))
        );
    }

    vcard.to_string()
}

/// Split "First Last" into (given, family). Handles multi-word names.
fn split_name(name: &str) -> (String, String) {
    let parts: Vec<&str> = name.splitn(2, ' ').collect();
    match parts.len() {
        0 => (String::new(), String::new()),
        1 => (parts[0].to_string(), String::new()),
        _ => (parts[0].to_string(), parts[1].to_string()),
    }
}
```

**Field mapping table:**

| Contact Field | vCard Property | Notes |
|---------------|---------------|-------|
| `name` | `FN` | Direct mapping |
| `name` (split) | `N` | Family;Given;;; structured format |
| `email[]` | `EMAIL` | One entry per address |
| `phone[]` | `TEL` | One entry per number |
| `company` | `ORG` | Single value |
| `role` | `TITLE` | vCard uses TITLE not ROLE |
| `website` | `URL` | Single value |
| `birthday` | `BDAY` | YYYY-MM-DD format |
| body notes | `NOTE` | Extract from ## Notes section |
| `source_id` / uid | `UID` | Must match URL filename |
| -- | -- | tags, status, priority, follow_up_cadence are CRM-only, NOT serialized |

### Pattern 3: Selective Sync Filters

**What:** Predicate functions that determine whether a contact should be included in push/pull.
**When:** Before processing each contact in the sync loop.

```rust
// In sync/filter.rs -- new module
pub struct SyncFilter {
    pub tags: Vec<String>,         // include contacts with ANY of these tags
    pub exclude_tags: Vec<String>, // exclude contacts with ANY of these tags
    pub statuses: Vec<Status>,     // include contacts with ANY of these statuses
}

impl SyncFilter {
    pub fn matches(&self, contact: &Contact) -> bool {
        // If no filters set, match everything
        if self.tags.is_empty() && self.statuses.is_empty() && self.exclude_tags.is_empty() {
            return true;
        }
        // Check exclude tags first (exclusion wins)
        if self.exclude_tags.iter().any(|t| contact.tags.contains(t)) {
            return false;
        }
        // Check include tags (empty = no tag constraint)
        let tag_match = self.tags.is_empty()
            || self.tags.iter().any(|t| contact.tags.contains(t));
        // Check statuses (empty = no status constraint)
        let status_match = self.statuses.is_empty()
            || contact.status.as_ref().map_or(false, |s| {
                self.statuses.iter().any(|fs| std::mem::discriminant(s) == std::mem::discriminant(fs))
            });

        tag_match && status_match
    }

    pub fn empty() -> Self {
        Self { tags: vec![], exclude_tags: vec![], statuses: vec![] }
    }
}
```

Config format in `~/.config/acrm/sync.toml`:
```toml
apple_id = "user@icloud.com"
auto_push = false

[push_filter]
tags = ["professional", "family"]
exclude_tags = ["private"]

[pull_filter]
# empty = pull everything (default)
```

### Pattern 4: CardDAV DELETE

**What:** Remove a contact from the server.
**When:** Contact archived/deleted locally with `source == "icloud"`.

```rust
// In CardDavClient -- new method
pub fn delete_vcard(&self, vcard_url: &Url, etag: &str) -> Result<bool> {
    let response = self.client
        .request(Method::DELETE, vcard_url.as_str())
        .header("If-Match", format!("\"{}\"", etag))
        .basic_auth(&self.apple_id, Some(&self.app_password))
        .send()?;

    match response.status().as_u16() {
        200 | 204 => Ok(true),
        404 => Ok(true),  // Already gone, not an error
        412 => bail!("Conflict: server version changed since last sync. Re-sync first."),
        status => bail!("DELETE failed with status {}", status),
    }
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Full Re-download on Every Push

**What:** Fetching all vCards from iCloud before pushing to compare content.
**Why bad:** Wastes bandwidth and time. With 700+ contacts, this adds 10+ seconds per sync.
**Instead:** Use ETags for change detection. PROPFIND the list (lightweight, returns hrefs + etags only) to get current server state, then compare locally. Only fetch full vCards when needed.

### Anti-Pattern 2: Storing Sync State Outside Contact Files

**What:** Creating a separate sync ledger/database (e.g., `.sync/carddav-state.yaml`) to track push state.
**Why bad:** Violates the flat-file architecture. Adds another state to keep in sync. Git history already tracks changes. The v1.0 architecture already stores `source`, `source_id`, and `etag` in frontmatter -- this is the right approach.
**Instead:** The contact's `etag` and `source_id` fields in frontmatter ARE the sync state. After a successful push, update these fields in place. A contact with empty `source_id` has never been pushed. A contact with `source: "icloud"` and a `source_id` is tracked.

### Anti-Pattern 3: Async HTTP for Push

**What:** Switching to tokio/async reqwest for parallel pushes.
**Why bad:** Adds significant complexity (tokio runtime, async infection). The existing codebase uses `reqwest::blocking` consistently. Parallel requests to iCloud may also trigger rate limiting.
**Instead:** Push contacts sequentially. For typical pushes (0-5 changed contacts), this takes seconds. For a full initial push of 700 contacts, ~2-3 minutes is acceptable for a CLI tool run once.

### Anti-Pattern 4: Two-Way Field-Level Merge on Conflict

**What:** Attempting to merge individual fields when both local and server have changes.
**Why bad:** Enormous complexity. Which fields win? What about deleted fields? Merge semantics for arrays (emails, phones) are ambiguous. The project constraint explicitly says "CRM wins."
**Instead:** On conflict, warn the user and overwrite the entire server vCard. Log which contacts had conflicts in the push result output.

## Integration Points

### 1. CardDavClient (sync/carddav.rs) -- 2 new methods

Add `put_vcard()` and `delete_vcard()` to the existing `CardDavClient` struct. These follow the same pattern as existing methods (basic auth, error handling, status code matching) but use PUT and DELETE methods.

New struct needed:
```rust
pub struct PutResult {
    pub new_etag: Option<String>,
    pub conflict: bool,
}
```

### 2. VCard Serialization (sync/vcard_map.rs) -- 1 new public function + 1 helper

Add `map_contact_to_vcard(contact: &Contact, uid: &str, notes: &str) -> String` as the reverse of `map_vcard_to_contact()`. Pure function, no side effects, fully unit-testable with round-trip tests (parse -> map to contact -> map back to vcard -> parse again -> compare).

Add `split_name(name: &str) -> (String, String)` helper for structured name decomposition.

### 3. Push Orchestration (sync/push.rs) -- new module

The largest new component. Orchestrates:

1. Load all local contacts via `store::load_all_contacts()`
2. Apply push filter via `filter::SyncFilter::matches()`
3. Discover address book (reuses `CardDavClient::discover_address_book()`)
4. PROPFIND to get current server ETag list (reuses `CardDavClient::fetch_vcard_list()`)
5. Build server ETag index: `HashMap<String, String>` mapping source_id -> server_etag
6. For each filtered local contact, determine action:
   - **New (no source_id):** Generate UUID as UID, build vCard URL as `{addressbook_url}/{uid}.vcf`, PUT, store source_id + ETag
   - **Update (source_id exists, ETag matches server):** PUT with If-Match
   - **Conflict (source_id exists, ETag differs):** Warn, PUT with If-Match: * (CRM wins)
   - **Delete (archived + has source_id):** DELETE, clear sync fields
   - **Unchanged (source_id exists, no local changes detected):** Skip
7. Update contact frontmatter on disk after each successful push
8. Return `PushResult` struct with counts and details

### 4. Filter Module (sync/filter.rs) -- new module

Pure predicate logic. Constructed from config. Applied in both push and pull paths. No dependencies beyond `models::Contact`.

### 5. Config Extension (sync/config.rs) -- modify

Add parsing for new TOML fields: `auto_push` (bool), `push_filter` (table), `pull_filter` (table). Backward-compatible: missing fields default to `false` / empty filters.

### 6. CLI Wiring (main.rs + commands/sync.rs)

```rust
#[derive(Subcommand)]
enum SyncAction {
    /// Set up iCloud credentials
    Setup,
    /// Push local changes to iCloud
    Push {
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,
        /// Force push all contacts (ignore conflicts)
        #[arg(long)]
        force: bool,
    },
}
```

The existing `acrm sync` (no subcommand) remains pull-only for backward compatibility. `acrm sync push` is the new push command. Filters apply to both directions automatically.

### 7. Frontmatter Updates After Push

After a successful PUT, update the contact file's frontmatter:
- `source` -> `"icloud"` (if was empty)
- `source_id` -> the UID used in the vCard URL
- `etag` -> the new ETag from the server response

Use existing `frontmatter::update_field()` + direct file write, matching the pattern already used in `update_existing_contact()` in `commands/sync.rs`.

## URL Construction for New Contacts

For new contacts (no existing server resource), construct the PUT URL:

```
{addressbook_url}/{uid}.vcf
```

Where `uid` is a newly generated UUID (via `uuid::Uuid::new_v4()`). iCloud accepts UUID-format filenames for new vCard resources. The UID property inside the vCard body must match this filename (without the `.vcf` extension).

## Suggested Build Order

Build order follows dependency chain. Each step can be tested independently before proceeding:

```
Step 1: vcard_map (reverse mapping)     -- no network deps, pure functions
    |
Step 2: carddav (PUT/DELETE methods)    -- no new struct deps
    |
Step 3: filter (predicate module)       -- no deps beyond models
    |
Step 4: config (extended TOML parsing)  -- depends on filter types
    |
Step 5: push (orchestration)            -- depends on steps 1-4
    |
Step 6: CLI wiring                      -- depends on step 5
    |
Step 7: auto-push on save (optional)    -- depends on step 6
```

**Rationale:**
- Steps 1-3 are independent and could be built in parallel
- Step 4 depends on filter types being defined
- Step 5 is the integration point that wires everything together
- Step 6 is thin CLI plumbing
- Step 7 is a refinement that hooks into existing edit/log commands

## Scalability Considerations

| Concern | Current (~700 contacts) | At 5K contacts | At 50K contacts |
|---------|------------------------|-----------------|------------------|
| PROPFIND list | <1s | 2-3s | 10-15s |
| Sequential PUTs (all) | ~2min | ~15min | Impractical |
| Typical push (changed only) | <5s | <5s | <5s |
| Mitigation | Push only changed | Push only changed | Need change tracking log |

For the current scale, sequential push of changed-only contacts is the right approach. Most syncs will push 0-5 contacts.

## Sources

- [RFC 6352 - CardDAV](https://datatracker.ietf.org/doc/html/rfc6352) -- ETag conflict detection (If-Match), PUT/DELETE semantics, Section 6.3.2 (HIGH confidence)
- [calcard docs - VCard struct](https://docs.rs/calcard/latest/calcard/vcard/struct.VCard.html) -- `entries: Vec<VCardEntry>`, `Default` impl, `.to_string()` serialization (HIGH confidence)
- [calcard docs - VCardEntry](https://docs.rs/calcard/latest/calcard/vcard/struct.VCardEntry.html) -- `new(prop).with_value(val)` builder API, `with_values()`, `with_param()` (HIGH confidence)
- [calcard docs - VCardValue](https://docs.rs/calcard/latest/calcard/vcard/enum.VCardValue.html) -- `Text(String)`, `Component(Vec<String>)`, `From<String>` impl (HIGH confidence)
- [calcard GitHub](https://github.com/stalwartlabs/calcard) -- `.to_string()` produces valid vCard text (HIGH confidence)
- Existing codebase analysis: `src/sync/carddav.rs`, `src/sync/vcard_map.rs`, `src/store.rs`, `src/models/contact.rs`, `src/commands/sync.rs`, `src/sync/dedup.rs` (HIGH confidence)
