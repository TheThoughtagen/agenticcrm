# Phase 4: Push Infrastructure - Research

**Researched:** 2026-03-07
**Domain:** CardDAV PUT/DELETE, vCard 3.0 serialization, round-trip data preservation
**Confidence:** HIGH

## Summary

Phase 4 adds the write side of iCloud sync. The existing codebase has a complete CardDAV pull pipeline (PROPFIND discovery, GET vCards, parse with calcard, map to Contact model). This phase must add the reverse: serialize CRM Contact data back to vCard 3.0, PUT new/updated vCards to iCloud, DELETE removed contacts, and preserve iCloud-only data (photos, X-properties, TYPE params) that the CRM does not model.

The calcard crate (v0.3.2, already a dependency) supports both parsing and serialization. VCard objects have a `write_to` method and implement `Display` (via `to_string()`). VCardEntry supports a builder pattern (`new()`, `with_value()`, `with_param()`). The `VCardProperty::Other` variant handles Apple-specific extensions (X-ABUID, X-ABLABEL) transparently during parse-modify-serialize cycles. The key architectural pattern is "parse cached vCard, overlay CRM fields, serialize" rather than "build from scratch" -- this is what makes round-tripping lossless.

**Primary recommendation:** Use a vCard cache (`.sync/vcards/{source_id}.vcf`) to store the raw vCard text fetched during pull. For push, parse the cached vCard, overwrite mapped fields from CRM data, serialize, and PUT. For new contacts (no cache), build a minimal vCard 3.0 from scratch. This avoids data loss for iCloud-only properties.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PUSH-01 | Push new CRM contact to iCloud (creates vCard on server) | CardDAV PUT with new UUID URL, vCard 3.0 serialization from Contact model, calcard builder API |
| PUSH-02 | Push updated CRM contact to iCloud (replaces vCard on server) | CardDAV PUT with If-Match ETag header, parse-modify-serialize via vCard cache |
| PUSH-03 | Push CRM deletion/archive to iCloud (removes contact from server) | CardDAV DELETE with If-Match ETag header, detect archived contacts via source_id |
| PUSH-04 | Push preserves iCloud-only data via vCard cache | Cache raw vCard text during pull, parse-overlay-serialize pattern preserves X-properties, PHOTO, TYPE params |
| PUSH-05 | Conflict warning when iCloud has newer version | ETag comparison: fetch current ETag list from server, compare with stored ETags, warn on mismatch |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| calcard | 0.3.2 | vCard 3.0 parse + serialize | Already in use for pull; supports builder pattern, VCardProperty::Other for X-properties, write_to/to_string |
| reqwest | 0.13.2 | HTTP client for CardDAV PUT/DELETE | Already in use for pull; blocking mode, custom methods supported |
| quick-xml | 0.39.2 | Parse PROPFIND XML responses | Already in use for ETag list fetching |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| uuid | 1 | Generate UUIDs for new vCard resources | New contacts need a UUID-based .vcf filename |
| url | 2.5.8 | URL construction for PUT/DELETE targets | Build resource URLs from addressbook base + UUID |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| calcard for serialization | Manual string building | Fragile, error-prone, misses escaping; calcard handles it correctly |
| File-based vCard cache | SQLite cache | Over-engineering for personal scale; files are git-friendly and inspectable |

**Installation:** No new dependencies needed. All libraries already in Cargo.toml.

## Architecture Patterns

### Recommended Project Structure
```
src/
  sync/
    mod.rs              # Add vcard_write, push modules
    carddav.rs          # Add put_vcard(), delete_vcard() methods to CardDavClient
    vcard_map.rs         # Existing: vCard -> Contact
    vcard_write.rs       # NEW: Contact -> vCard (serialize), cache merge logic
    push.rs              # NEW: Push orchestration (diff detection, PUT/DELETE dispatch)
    config.rs            # Existing: credentials
    dedup.rs             # Existing: source_id matching
.sync/
  vcards/               # NEW: Raw vCard cache directory (gitignored)
    {source_id}.vcf      # Cached raw vCard text from last pull
```

### Pattern 1: Parse-Overlay-Serialize (Round-Trip Preservation)
**What:** For contacts that originated from iCloud (have source_id + cached vCard), parse the cached vCard, overwrite CRM-mapped fields, then serialize. Unmapped properties (X-ABUID, X-ABLABEL, PHOTO, custom TYPE params) survive untouched.
**When to use:** Updating an existing iCloud contact (PUSH-02, PUSH-04).
**Example:**
```rust
// Source: calcard docs.rs API + project vcard_map.rs patterns
pub fn merge_contact_to_vcard(contact: &Contact, cached_vcard_text: &str) -> Result<String> {
    let mut vcard = VCard::parse(cached_vcard_text)
        .map_err(|e| anyhow::anyhow!("vCard parse error: {:?}", e))?;

    // Remove CRM-mapped properties, then re-add with current values
    // This preserves all non-CRM properties (X-*, PHOTO, etc.)
    remove_crm_properties(&mut vcard);
    add_crm_properties(&mut vcard, contact);

    Ok(vcard.to_string())
}
```

### Pattern 2: Build Minimal vCard (New Contacts)
**What:** For contacts created in CRM with no iCloud history, build a fresh vCard 3.0 with only the properties the CRM maps.
**When to use:** Creating a new contact on iCloud (PUSH-01).
**Example:**
```rust
// Source: calcard docs.rs VCardEntry API
pub fn contact_to_vcard(contact: &Contact) -> Result<String> {
    // Build entries using VCardEntry::new(prop).with_value(val)
    let mut entries = vec![
        VCardEntry::new(VCardProperty::Version)
            .with_value(VCardValue::Text("3.0".to_string())),
        VCardEntry::new(VCardProperty::Fn)
            .with_value(VCardValue::Text(contact.name.clone())),
        VCardEntry::new(VCardProperty::Uid)
            .with_value(VCardValue::Text(contact.source_id.clone())),
    ];
    // Add N, EMAIL, TEL, ORG, TITLE, URL, BDAY, NOTE as needed
    // ...
    // Construct VCard from entries and serialize
    let vcard = VCard { entries, ..Default::default() };
    Ok(vcard.to_string())
}
```

### Pattern 3: CardDAV PUT with ETag Precondition
**What:** Use If-Match header for optimistic concurrency control.
**When to use:** Every PUT/DELETE to iCloud.
**Example:**
```rust
// Source: RFC 6352 + sabre.io CardDAV client guide
impl CardDavClient {
    pub fn put_vcard(&self, url: &Url, vcard_text: &str, etag: Option<&str>) -> Result<String> {
        let mut req = self.client.put(url.as_str())
            .header("Content-Type", "text/vcard; charset=utf-8")
            .basic_auth(&self.apple_id, Some(&self.app_password))
            .body(vcard_text.to_string());

        if let Some(etag) = etag {
            // Update: require matching ETag
            req = req.header("If-Match", format!("\"{}\"", etag));
        } else {
            // Create: require resource does NOT exist
            req = req.header("If-None-Match", "*");
        }

        let response = req.send()?;
        match response.status().as_u16() {
            201 | 204 => {
                // Extract new ETag from response
                let new_etag = response.headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.trim_matches('"').to_string())
                    .unwrap_or_default();
                Ok(new_etag)
            }
            412 => anyhow::bail!("Conflict: server has newer version"),
            _ => anyhow::bail!("PUT failed with status {}", response.status()),
        }
    }

    pub fn delete_vcard(&self, url: &Url, etag: &str) -> Result<()> {
        let response = self.client
            .delete(url.as_str())
            .header("If-Match", format!("\"{}\"", etag))
            .basic_auth(&self.apple_id, Some(&self.app_password))
            .send()?;

        match response.status().as_u16() {
            200 | 204 => Ok(()),
            412 => anyhow::bail!("Conflict: server has newer version"),
            404 => Ok(()), // Already gone, idempotent
            _ => anyhow::bail!("DELETE failed with status {}", response.status()),
        }
    }
}
```

### Pattern 4: vCard Cache Management
**What:** Store raw vCard text fetched during pull in `.sync/vcards/`. Update cache after successful PUT.
**When to use:** Every pull stores cache; every push reads cache for merge.
**Example:**
```rust
pub fn cache_dir(crm_root: &Path) -> PathBuf {
    crm_root.join(".sync").join("vcards")
}

pub fn read_cached_vcard(crm_root: &Path, source_id: &str) -> Option<String> {
    let path = cache_dir(crm_root).join(format!("{}.vcf", source_id));
    std::fs::read_to_string(path).ok()
}

pub fn write_cached_vcard(crm_root: &Path, source_id: &str, vcard_text: &str) -> Result<()> {
    let dir = cache_dir(crm_root);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.vcf", source_id));
    std::fs::write(path, vcard_text)?;
    Ok(())
}
```

### Anti-Patterns to Avoid
- **Building vCards from scratch for existing contacts:** This destroys iCloud-only data (PHOTO, X-ABUID, X-ABLABEL, custom TYPE params). Always parse-overlay-serialize when a cached vCard exists.
- **Skipping If-Match header:** Without ETags, concurrent edits from Apple Contacts or other devices silently overwrite each other. Always use If-Match for updates.
- **Storing vCard cache inside contacts/ directory:** Cache files are binary-ish artifacts, not CRM data. They belong in .sync/ (gitignored).
- **Using If-None-Match: * for updates:** If-None-Match: * means "create only if not exists." Use If-Match with the stored ETag for updates.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| vCard serialization | String concatenation with \r\n | calcard VCard::to_string() / write_to() | Escaping rules (semicolons, commas, line folding) are complex and version-dependent |
| vCard property parameters | Manual TYPE=WORK;CELL param strings | VCardEntry::with_param() | Parameter quoting and encoding varies by vCard version |
| UUID generation for new resources | Random string formatting | uuid::Uuid::new_v4().to_string() | Already a dependency; guaranteed uniqueness format |
| ETag quoting | Manual quote wrapping | Consistent trim_matches('"') on read, format!("\"{}\"", etag) on write | ETags may or may not arrive pre-quoted from server |

**Key insight:** The calcard library already handles the vCard 3.0 format details (line folding, escaping, property ordering). The main work is mapping CRM Contact fields to VCardEntry objects and managing the cache lifecycle.

## Common Pitfalls

### Pitfall 1: iCloud Rewriting vCard Content
**What goes wrong:** iCloud modifies vCards after PUT (adds PRODID, reorders properties, normalizes TEL format). The returned ETag differs from what you'd compute from your submitted content.
**Why it happens:** iCloud server-side processing is opaque and undocumented.
**How to avoid:** After a successful PUT, always use the ETag from the response header (not computed from content). If no ETag is returned in the PUT response, immediately do a PROPFIND to fetch the new ETag. Update both the CRM frontmatter `etag` field and the vCard cache with the server's version.
**Warning signs:** Subsequent push of the same unchanged contact triggers an update because ETags don't match.

### Pitfall 2: Missing Content-Type Header
**What goes wrong:** PUT without `Content-Type: text/vcard; charset=utf-8` may be rejected by iCloud or interpreted incorrectly.
**Why it happens:** Easy to forget when copying from GET request patterns.
**How to avoid:** Always set `Content-Type: text/vcard; charset=utf-8` on PUT requests.
**Warning signs:** 400 or 415 responses from PUT.

### Pitfall 3: vCard Line Endings
**What goes wrong:** vCard 3.0 spec requires CRLF (\r\n) line endings. Some servers reject LF-only.
**Why it happens:** Rust's default newline is \n; calcard may or may not add \r\n.
**How to avoid:** Verify calcard's output uses CRLF. If not, do a simple .replace("\n", "\r\n") post-serialization (being careful not to double-convert existing \r\n).
**Warning signs:** Parse errors on iCloud after PUT, or contacts appearing empty/malformed.

### Pitfall 4: Stale ETag After Pull-Then-Push
**What goes wrong:** Pull updates the local ETag, but someone edits the contact on iCloud between pull and push. The push succeeds with the old ETag, and iCloud returns 412 Precondition Failed.
**Why it happens:** ETag represents a point-in-time snapshot; time passes between operations.
**How to avoid:** This is actually correct behavior -- the 412 means conflict detection is working. Handle 412 gracefully with a user-facing warning (PUSH-05 requirement). The user can re-pull then re-push.
**Warning signs:** 412 responses during push.

### Pitfall 5: Resource URL for New Contacts
**What goes wrong:** Using a non-UUID or improperly formatted URL for new vCard resources.
**Why it happens:** iCloud expects specific URL patterns under the addressbook collection.
**How to avoid:** Generate a UUID-v4, use it as both the UID property in the vCard and as the filename: `{addressbook_url}/{uuid}.vcf`. Store this as the contact's `source_id`.
**Warning signs:** 403 or 409 responses on PUT for new contacts.

### Pitfall 6: Detecting Contacts to Push
**What goes wrong:** No clear mechanism to know which local contacts have changed since last sync.
**Why it happens:** The CRM is file-based with no change tracking beyond git.
**How to avoid:** Compare CRM contacts against the vCard cache. For each contact with `source: icloud`: serialize CRM -> vCard, compare with cached vCard. If different, it needs pushing. For contacts with no source_id, they're new. For archived contacts with source_id, they need DELETE.
**Warning signs:** Pushing every contact every time (slow, unnecessary server load).

## Code Examples

### CRM Field to vCard Property Mapping
```rust
// Reverse of the existing vcard_map.rs mapping
fn add_crm_properties(vcard: &mut VCard, contact: &Contact) {
    // FN (required in vCard 3.0)
    vcard.entries.push(
        VCardEntry::new(VCardProperty::Fn)
            .with_value(VCardValue::Text(contact.name.clone()))
    );

    // N (structured name: Family;Given;;;)
    let parts: Vec<&str> = contact.name.splitn(2, ' ').collect();
    let (given, family) = match parts.as_slice() {
        [first, last] => (*first, *last),
        [name] => (*name, ""),
        _ => ("", ""),
    };
    vcard.entries.push(
        VCardEntry::new(VCardProperty::N)
            .with_value(VCardValue::Component(vec![
                family.to_string(), given.to_string(),
                String::new(), String::new(), String::new(),
            ]))
    );

    // EMAIL entries
    for email in &contact.email {
        vcard.entries.push(
            VCardEntry::new(VCardProperty::Email)
                .with_value(VCardValue::Text(email.clone()))
        );
    }

    // TEL entries
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
}

// Properties the CRM maps -- these get removed before re-adding
const CRM_MAPPED_PROPERTIES: &[VCardProperty] = &[
    VCardProperty::Fn, VCardProperty::N, VCardProperty::Email,
    VCardProperty::Tel, VCardProperty::Org, VCardProperty::Title,
    VCardProperty::Url, VCardProperty::Bday, VCardProperty::Note,
];

fn remove_crm_properties(vcard: &mut VCard) {
    vcard.entries.retain(|entry| {
        !CRM_MAPPED_PROPERTIES.contains(&entry.name)
    });
}
```

### Push Change Detection
```rust
/// Determine what needs to be pushed to iCloud.
pub struct PushChangeset {
    pub creates: Vec<ContactFile>,   // New contacts (no source_id)
    pub updates: Vec<ContactFile>,   // Modified contacts (source_id + changed content)
    pub deletes: Vec<(String, String)>, // (source_id, etag) for archived/deleted contacts
    pub conflicts: Vec<(ContactFile, String)>, // (contact, server_etag) for ETag mismatches
}

pub fn compute_push_changeset(
    crm_root: &Path,
    contacts: &[ContactFile],
    archived: &[ContactFile],
    server_entries: &[VCardEntry],  // From fetch_vcard_list
) -> Result<PushChangeset> {
    let server_etags: HashMap<&str, &str> = server_entries.iter()
        .map(|e| (extract_uid_from_href(&e.href).as_str(), e.etag.as_str()))
        .collect();

    let mut changeset = PushChangeset::default();

    for cf in contacts {
        if cf.contact.source.is_empty() || cf.contact.source != "icloud" {
            if has_pushable_data(cf) {
                changeset.creates.push(cf.clone());
            }
            continue;
        }

        // Existing iCloud contact -- check if CRM data changed
        let cached = read_cached_vcard(crm_root, &cf.contact.source_id);
        // ... compare serialized CRM vCard with cache to detect changes
        // ... check server_etags for conflict detection
    }

    // Detect deletes: archived contacts with source_id
    for cf in archived {
        if cf.contact.source == "icloud" && !cf.contact.source_id.is_empty() {
            changeset.deletes.push((
                cf.contact.source_id.clone(),
                cf.contact.etag.clone(),
            ));
        }
    }

    Ok(changeset)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Build vCard strings manually | calcard builder API + to_string() | calcard 0.2+ | Proper escaping, line folding, property ordering |
| No conflict detection | ETag-based If-Match headers (RFC 6352) | Always standard | Prevents silent data loss on concurrent edits |
| Full re-upload on each sync | Change detection via vCard cache diff | Common pattern | Reduces server requests, avoids rate limiting |

**Deprecated/outdated:**
- vCard 2.1: iCloud requires 3.0 minimum. Do not emit VERSION:2.1.
- Sync-token (RFC 6578): Out of scope per REQUIREMENTS.md. ETag-based detection is sufficient at personal scale.

## Open Questions

1. **calcard VCard struct field access for modification**
   - What we know: VCard has an `entries` field (Vec<VCardEntry>), and VCardEntry can be constructed via builder methods
   - What's unclear: Whether `entries` is pub and can be directly mutated, or if we need to reconstruct the VCard. The docs show 0.06% documentation coverage.
   - Recommendation: Test with a small spike -- parse a vCard, push a new entry, call to_string(). If entries is not pub, use parse -> filter entries -> add new entries -> construct new VCard from entries.

2. **iCloud rate limiting behavior**
   - What we know: Undocumented rate limits exist (noted in STATE.md blockers)
   - What's unclear: Exact thresholds and response codes
   - Recommendation: Add 200ms delay between PUT/DELETE requests. If 429 or 503 received, implement exponential backoff (1s, 2s, 4s, max 30s). This is a defensive measure, not a validated requirement.

3. **CRLF handling in calcard serialization**
   - What we know: vCard 3.0 requires CRLF line endings
   - What's unclear: Whether calcard's to_string() produces CRLF or LF
   - Recommendation: Test and add CRLF normalization if needed. Simple post-processing step.

## Sources

### Primary (HIGH confidence)
- [RFC 6352 - CardDAV specification](https://tools.ietf.org/html/rfc6352) - PUT, DELETE, If-Match, ETag requirements
- [sabre.io CardDAV client guide](https://sabre.io/dav/building-a-carddav-client/) - Practical PUT/DELETE request format, URL naming, Content-Type
- [calcard docs.rs](https://docs.rs/calcard/0.3.2/calcard/vcard/struct.VCard.html) - VCard API: parse, write_to, to_string, Display trait
- [calcard VCardEntry docs.rs](https://docs.rs/calcard/0.3.2/calcard/vcard/struct.VCardEntry.html) - Builder API: new(), with_value(), with_param(), with_group()
- [calcard VCardProperty docs.rs](https://docs.rs/calcard/0.3.2/calcard/vcard/enum.VCardProperty.html) - 53 property variants including Other for X-properties

### Secondary (MEDIUM confidence)
- [calcard GitHub README](https://github.com/stalwartlabs/calcard) - to_string() serialization confirmed, builder docs "coming soon"
- [vdirsyncer iCloud issue #1145](https://github.com/pimutils/vdirsyncer/issues/1145) - Real-world iCloud PUT Content-Type: text/vcard

### Tertiary (LOW confidence)
- iCloud rate limits: No official documentation found. 200ms delay is community convention from various CardDAV client implementations.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all libraries already in use, APIs verified via docs.rs
- Architecture: HIGH - parse-overlay-serialize is the established pattern for lossless vCard round-tripping; CardDAV PUT/DELETE is RFC-specified
- Pitfalls: HIGH - well-documented in RFC 6352 and real-world CardDAV client implementations
- calcard builder API: MEDIUM - docs sparse (0.06% coverage), but struct/method signatures confirmed on docs.rs

**Research date:** 2026-03-07
**Valid until:** 2026-04-07 (stable domain, RFC-based protocol)
