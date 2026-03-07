# Technology Stack

**Project:** AgenticCRM v1.1 - Two-Way iCloud Sync (Push, Conflict Detection, Selective Filtering)
**Researched:** 2026-03-07
**Overall confidence:** HIGH

## Key Finding: Zero New Cargo Dependencies Required

The existing stack handles everything needed for v1.1. This was confirmed by reading calcard 0.3.2 source code locally and verifying reqwest's HTTP method support against the existing PROPFIND implementation.

## Existing Stack (No Changes to Cargo.toml)

| Technology | Version | v1.1 Role | Confidence |
|------------|---------|-----------|------------|
| reqwest | 0.13.2 (blocking) | PUT/DELETE with If-Match/If-None-Match headers. Already creates custom methods (PROPFIND); PUT/DELETE are simpler. | HIGH - verified in carddav.rs |
| calcard | 0.3.2 | VCard construction from Contact + serialization via `write_to()` / `to_string()`. Builder pattern confirmed in source. | HIGH - verified in local source |
| quick-xml | 0.39.2 | Parse error response bodies from PUT/DELETE (same XML parsing as existing PROPFIND responses) | HIGH - already working |
| uuid | 1 (v4) | Generate source_id/UID for new CRM-created contacts pushed to iCloud | HIGH - already a dep |
| chrono | 0.4 | Format birthday dates for vCard BDAY property | HIGH - already a dep |
| url | 2.5.8 | Build PUT/DELETE target URLs: `{addressbook_url}/{source_id}.vcf` | HIGH - already a dep |
| clap | 4 (derive) | New `sync push` subcommand with --force, --dry-run, --tag, --status flags | HIGH - already a dep |
| serde/serde_yaml | 1/0.9 | Extended sync config parsing (push_tags, push_statuses, auto_push) | HIGH - already a dep |
| anyhow | 1 | Error handling for new push/delete operations | HIGH - already a dep |
| keyring | 3.6.3 | Unchanged - same credential storage | HIGH - already working |

## calcard 0.3.2: VCard Construction API (Verified from Source)

**Source:** `/Users/pmannion/.cargo/registry/src/.../calcard-0.3.2/src/vcard/builder.rs` and `writer.rs`

calcard provides everything needed to build vCards programmatically and serialize them:

### Construction Pattern
```rust
use calcard::vcard::{VCard, VCardEntry, VCardProperty, VCardValue, VCardVersion};

let vcard = VCard {
    entries: vec![
        // FN (formatted name) - required by iCloud
        VCardEntry::new(VCardProperty::Fn)
            .with_value(VCardValue::Text("Jane Smith".to_string())),
        // N (structured name) - required by iCloud
        VCardEntry::new(VCardProperty::N)
            .with_values(vec![
                VCardValue::Text("Smith".to_string()),     // Family
                VCardValue::Text("Jane".to_string()),      // Given
                VCardValue::Text(String::new()),           // Middle
                VCardValue::Text(String::new()),           // Prefix
                VCardValue::Text(String::new()),           // Suffix
            ]),
        // UID - must match the .vcf filename
        VCardEntry::new(VCardProperty::Uid)
            .with_value(VCardValue::Text("abc-123-def".to_string())),
        // EMAIL (one VCardEntry per address)
        VCardEntry::new(VCardProperty::Email)
            .with_value(VCardValue::Text("jane@example.com".to_string())),
        // TEL
        VCardEntry::new(VCardProperty::Tel)
            .with_value(VCardValue::Text("+1-555-0100".to_string())),
        // ORG
        VCardEntry::new(VCardProperty::Org)
            .with_value(VCardValue::Text("Acme Corp".to_string())),
        // TITLE (maps from Contact.role)
        VCardEntry::new(VCardProperty::Title)
            .with_value(VCardValue::Text("Engineer".to_string())),
    ],
};
```

### Serialization
```rust
// Option 1: write_to with explicit version (iCloud uses 3.0)
let mut output = String::new();
vcard.write_to(&mut output, VCardVersion::V3_0).unwrap();
// Produces: BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Jane Smith\r\n...END:VCARD\r\n

// Option 2: to_string() uses detected/default version
let vcard_text = vcard.to_string();
```

### Key API Details (from source)
- `VCard { entries: Vec<VCardEntry> }` -- entries field is public, construct directly
- `VCard::default()` -- creates empty VCard (Default derive)
- `VCardEntry::new(VCardProperty) -> Self` -- builder start
- `.with_value(impl Into<VCardValue>)` -- add single value; `String` auto-converts via `From<String>`
- `.with_values(Vec<VCardValue>)` -- set multiple values (for N, ADR structured fields)
- `.with_param(impl Into<VCardParameter>)` -- add TYPE=work, etc.
- `VCardParameter::typ(VCardType::Work.into())` -- convenience constructors
- `vcard.write_to(&mut output, VCardVersion::V3_0)` -- serialize to specific version
- Writer auto-handles: line folding at 75 chars, BEGIN/END/VERSION wrapping, escaping

### Contact-to-VCard Field Mapping

| Contact Field | vCard Property | Mapping Notes |
|---------------|---------------|---------------|
| `name` | `FN` | Direct copy |
| `name` (split) | `N` | Split "First Last" into Family;Given;;; |
| `email[]` | `EMAIL` (multiple entries) | One VCardEntry per email |
| `phone[]` | `TEL` (multiple entries) | One VCardEntry per phone |
| `company` | `ORG` | Skip if empty |
| `role` | `TITLE` | vCard TITLE maps to our "role" field |
| `website` | `URL` | Skip if empty |
| `birthday` | `BDAY` | Format as YYYY-MM-DD text |
| body notes | `NOTE` | Extract from ## Notes section of body |
| `source_id` | `UID` | Must match .vcf filename |
| tags, status, priority, etc. | -- | **NOT serialized** -- CRM-only fields |

## reqwest: PUT/DELETE with Conditional Headers

**Source:** Existing `carddav.rs` already creates custom HTTP methods. PUT and DELETE are standard.

### PUT for Create (new contact)
```rust
client.put(vcard_url.as_str())
    .header("Content-Type", "text/vcard; charset=utf-8")
    .header("If-None-Match", "*")  // Fail if resource already exists
    .basic_auth(&apple_id, Some(&app_password))
    .body(vcard_string)
    .send()?;
// Success: 201 Created. Read ETag from response headers.
```

### PUT for Update (existing contact)
```rust
client.put(vcard_url.as_str())
    .header("Content-Type", "text/vcard; charset=utf-8")
    .header("If-Match", format!("\"{}\"", stored_etag))  // Conflict detection
    .basic_auth(&apple_id, Some(&app_password))
    .body(vcard_string)
    .send()?;
// Success: 204 No Content (or 200). Read new ETag from response headers.
// Conflict: 412 Precondition Failed. Server has newer version.
```

### DELETE (remove contact)
```rust
client.delete(vcard_url.as_str())
    .header("If-Match", format!("\"{}\"", stored_etag))
    .basic_auth(&apple_id, Some(&app_password))
    .send()?;
// Success: 204 No Content.
// Already gone: 404 Not Found (handle gracefully, not an error).
// Conflict: 412 Precondition Failed.
```

### Response Status Handling
| Status | Meaning | Action |
|--------|---------|--------|
| 200/201 | Created/OK | Read ETag header, update frontmatter |
| 204 | No Content (update/delete OK) | Read ETag header, update frontmatter |
| 404 | Not Found (on DELETE) | Treat as success (already gone) |
| 412 | Precondition Failed | ETag mismatch = conflict. Warn user, force-overwrite if --force |
| 401 | Unauthorized | Existing error handling pattern |

### ETag Extraction from Response
```rust
let new_etag = response.headers()
    .get("ETag")
    .and_then(|v| v.to_str().ok())
    .map(|s| s.trim_matches('"').to_string());
// If absent (some servers don't return ETag on PUT), do a HEAD/PROPFIND to get it
```

## Selective Sync Config: Extend Hand-Parser

The existing `sync/config.rs` hand-parses `sync.toml` line by line. For v1.1, add 3 more fields. This is simpler than adding a TOML crate dependency for such minimal config.

### Extended sync.toml format
```toml
apple_id = "user@icloud.com"
auto_push = false
push_tags = ["professional", "family"]
push_statuses = ["active"]
```

### Parsing approach
Extend the existing `parse_apple_id` pattern:
- `auto_push` -- parse as boolean (`"true"/"false"`)
- `push_tags` -- parse as comma-separated or bracket-delimited list
- `push_statuses` -- same as tags

If config grows beyond ~6 fields in future milestones, add the `toml` crate then.

## What NOT to Add

| Crate Considered | Why Not |
|------------------|---------|
| `toml` | Over-engineering for 4 config fields; hand-parser already works |
| `vcard_parser` / `ical_vcard` | calcard already handles both parsing AND serialization |
| `tokio` / async | reqwest blocking works; CLI tool pushes sequentially; existing architecture decision |
| `sha2` / `md5` | ETags from server are the correct conflict detection per RFC 6352 |
| `diff` | CRM-wins strategy means no merge; just ETag comparison |
| `notify` (file watcher) | Auto-push via `--push` flag on commands is simpler than a daemon; defer watcher to later |
| `base64` | reqwest's `.basic_auth()` handles Base64 encoding internally |
| `tracing` | Useful but not required for v1.1; existing eprintln pattern sufficient for sync debugging |

## Integration Points with Existing Code

### Modified: `sync/carddav.rs`
Add 2 methods to `CardDavClient`:
- `put_vcard(&self, url: &Url, body: &str, etag: Option<&str>) -> Result<PutResult>`
- `delete_vcard(&self, url: &Url, etag: &str) -> Result<bool>`

New struct: `PutResult { new_etag: Option<String>, conflict: bool }`

### Modified: `sync/vcard_map.rs`
Add 1 public function (reverse of existing `map_vcard_to_contact`):
- `map_contact_to_vcard(contact: &Contact, uid: &str, notes: &str) -> String`

Add 1 helper: `split_name(name: &str) -> (String, String)`

### New: `sync/push.rs`
Push orchestration: load contacts, filter, compare ETags, determine create/update/delete actions, call client, update frontmatter.

### New: `sync/filter.rs`
Pure predicate functions: `SyncFilter::matches(contact: &Contact) -> bool` based on tags/status config.

### Modified: `sync/config.rs`
Add `auto_push`, `push_tags`, `push_statuses` parsing.

### Modified: `commands/sync.rs`
Add `run_push(force, dry_run, fmt)`. Wire up `acrm sync push` subcommand.

### Modified: `main.rs`
Add `Push` variant to `SyncAction` enum.

## Sources

- calcard 0.3.2 local source: `builder.rs`, `writer.rs`, `mod.rs` -- HIGH confidence, directly inspected
- [calcard docs.rs - VCard](https://docs.rs/calcard/0.3.2/calcard/vcard/struct.VCard.html) -- API reference
- [calcard docs.rs - VCardEntry](https://docs.rs/calcard/0.3.2/calcard/vcard/struct.VCardEntry.html) -- Builder pattern
- [calcard docs.rs - VCardProperty](https://docs.rs/calcard/0.3.2/calcard/vcard/enum.VCardProperty.html) -- 53 property variants
- [calcard GitHub](https://github.com/stalwartlabs/calcard) -- Repository, write_to + to_string confirmed
- [RFC 6352 - CardDAV](https://www.rfc-editor.org/rfc/rfc6352) -- PUT/DELETE/If-Match semantics
- [Google CardDAV docs](https://developers.google.com/people/carddav) -- PUT/DELETE examples confirming ETag patterns
- [vdirsyncer iCloud issue #1145](https://github.com/pimutils/vdirsyncer/issues/1145) -- iCloud CardDAV write behavior
- Existing codebase: `Cargo.toml`, `sync/carddav.rs`, `sync/vcard_map.rs`, `sync/config.rs`, `commands/sync.rs` -- HIGH confidence
- `cargo search calcard` -- confirmed 0.3.2 is latest version

---

*Stack research: 2026-03-07 -- v1.1 milestone (two-way sync push)*
*Previous research (v1.0 milestone): 2026-03-05*
