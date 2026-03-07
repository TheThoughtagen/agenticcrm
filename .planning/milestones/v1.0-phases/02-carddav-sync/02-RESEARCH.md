# Phase 2: CardDAV Sync - Research

**Researched:** 2026-03-06
**Domain:** CardDAV protocol (RFC 6352), vCard parsing (RFC 6350/2426), iCloud integration
**Confidence:** MEDIUM

## Summary

This phase implements one-way pull sync from iCloud CardDAV into the existing plain-text CRM. The core challenge is threefold: (1) speaking the CardDAV/WebDAV protocol to iCloud's server, (2) parsing vCard 3.0 data returned by iCloud into CRM YAML frontmatter, and (3) tracking sync metadata to prevent duplicates on re-runs.

The Rust CardDAV ecosystem is immature -- there is no single crate that handles the full CardDAV client flow. The recommended approach is to build a lightweight CardDAV client on top of `reqwest` (HTTP client already in wide use) + `quick-xml` (XML parsing for WebDAV responses), and use the `calcard` crate from Stalwart Labs for vCard parsing. The `calcard` crate supports vCard 3.0 (which iCloud returns) and follows Postel's law for robust parsing of non-conformant data.

**Primary recommendation:** Build a minimal CardDAV client module using reqwest for HTTP + quick-xml for XML responses, calcard for vCard parsing, and keyring for secure credential storage. Keep the CardDAV protocol surface small -- only implement discovery + full-sync via PROPFIND + GET (no REPORT, no incremental sync).

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SYNC-01 | User can pull contacts from iCloud via CardDAV into the CRM | CardDAV discovery flow (PROPFIND chain), reqwest custom methods, iCloud endpoint at contacts.icloud.com |
| SYNC-02 | Pulled contacts are converted from vCard format to CRM markdown+YAML format | calcard crate parses vCard 3.0; field mapping table from vCard properties to Contact struct fields |
| SYNC-03 | Duplicate detection prevents re-importing contacts that already exist | Match on source_id (CardDAV UID from vCard), with fallback name matching for manually-created contacts |
| SYNC-04 | Sync metadata (source, source_id, ETag) is stored in contact frontmatter | Existing schema already has source/source_id fields; need to add etag field to schema and Contact struct |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| reqwest | 0.12 | HTTP client for CardDAV requests | De facto Rust HTTP client; supports custom methods (PROPFIND), basic auth, TLS |
| quick-xml | 0.37 | Parse WebDAV XML responses | Fast, serde-compatible XML parser; well-maintained |
| calcard | 0.1 | Parse vCard 3.0 data from iCloud | From Stalwart Labs (production mail server); supports vCard 3.0; robust/lenient parsing |
| keyring | 3.6 | Store iCloud app-specific password in macOS Keychain | Cross-platform credential storage; uses native macOS Keychain |
| tokio | 1 | Async runtime for reqwest | Required by reqwest async; use `rt-multi-thread` feature |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| serde | 1 (already present) | Deserialize XML responses | Pair with quick-xml serde feature |
| url | 2 | URL manipulation for CardDAV paths | Joining base URLs with discovery paths |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| calcard | vcard_parser 0.2 | Only supports RFC 6350 (vCard 4.0); iCloud returns 3.0 -- would fail |
| calcard | vobject 0.8 | Incomplete RFC support, API unstable, not production-ready |
| reqwest | libdav | libdav is a full CalDAV/CardDAV client but heavy dependency; we only need a few HTTP calls |
| keyring | Environment variable | Less secure; keyring is more user-friendly and uses native Keychain |
| quick-xml | roxmltree | roxmltree is read-only (fine for us) but quick-xml has better serde integration |

**Installation:**
```bash
cargo add reqwest --features rustls-tls,json
cargo add quick-xml --features serialize
cargo add calcard
cargo add keyring --features apple-native
cargo add tokio --features rt-multi-thread,macros
cargo add url
```

## Architecture Patterns

### Recommended Project Structure
```
src/
├── commands/
│   └── sync.rs          # CLI command: `acrm sync`
├── sync/
│   ├── mod.rs           # Module exports
│   ├── carddav.rs       # CardDAV client (discovery, fetch)
│   ├── vcard_map.rs     # vCard-to-Contact field mapping
│   ├── dedup.rs         # Duplicate detection logic
│   └── config.rs        # Sync configuration & credential management
├── models/
│   └── contact.rs       # Add etag field to Contact struct
└── ...existing modules
```

### Pattern 1: CardDAV Discovery Chain
**What:** Three-step PROPFIND sequence to discover the user's address book URL
**When to use:** Every sync run (cache the discovered URL for the session)
**Flow:**
```
1. PROPFIND https://contacts.icloud.com/
   → Body: request current-user-principal
   → Response: /123456789/principal/

2. PROPFIND /123456789/principal/
   → Body: request addressbook-home-set
   → Response: /123456789/carddavhome/

3. PROPFIND /123456789/carddavhome/
   → Depth: 1
   → Body: request resourcetype, displayname, getctag
   → Response: list of address books with their URLs
```

### Pattern 2: Fetch-All-Then-Diff
**What:** Fetch all vCard URLs + ETags from the address book, compare against local sync state, download only new/changed
**When to use:** Each sync run
**Flow:**
```
1. PROPFIND address-book-URL with Depth:1 → get all vCard hrefs + ETags
2. Compare ETags against stored etag in local contact frontmatter
3. GET each new/changed vCard URL → raw vCard text
4. Parse vCard → map to Contact → write markdown file
```

### Pattern 3: Sync Command as Subcommand
**What:** Add `acrm sync` (or `acrm sync icloud`) as a new CLI subcommand
**When to use:** User-initiated pull sync
**Example:**
```rust
// In main.rs Commands enum:
/// Sync contacts from iCloud
Sync {
    /// Force re-download all contacts (ignore ETags)
    #[arg(long)]
    force: bool,
    /// Dry run - show what would change without writing
    #[arg(long)]
    dry_run: bool,
},
```

### Anti-Patterns to Avoid
- **Implementing full RFC 6352:** Only implement what iCloud needs. No need for REPORT/addressbook-multiget for a simple pull sync.
- **Storing vCard data in CRM files:** The CRM format is YAML frontmatter. Store only mapped fields + sync metadata. Do NOT try to round-trip vCard data.
- **Hardcoding iCloud URLs:** Use the discovery chain. iCloud may redirect to region-specific servers (p01-contacts.icloud.com, etc.).
- **Storing passwords in config files:** Use keyring for the macOS Keychain. Store only the Apple ID email in a config file.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| vCard parsing | Custom vCard text parser | calcard crate | vCard has complex folding, encoding, multi-value properties; edge cases everywhere |
| Credential storage | Custom encryption/config file | keyring crate | macOS Keychain is the right tool; handles encryption, access control |
| XML parsing | String manipulation of WebDAV XML | quick-xml + serde | XML namespaces, nested elements, attribute handling is error-prone |
| HTTP client | Raw TCP/TLS | reqwest | TLS, redirects, auth headers, connection pooling |
| URL joining | String concatenation | url crate | Relative URL resolution, encoding, edge cases with trailing slashes |

**Key insight:** The CardDAV protocol involves XML namespaces, HTTP extension methods, and multi-step discovery -- each layer has edge cases that are hard to get right manually.

## Common Pitfalls

### Pitfall 1: iCloud Returns vCard 3.0, Not 4.0
**What goes wrong:** Parser expects vCard 4.0 (RFC 6350), fails on 3.0 data
**Why it happens:** iCloud's CardDAV server returns vCard 3.0 by default
**How to avoid:** Use calcard which handles both versions. Do NOT use vcard_parser crate (4.0 only).
**Warning signs:** Parse errors on CHARSET parameters, ENCODING=QUOTED-PRINTABLE, TYPE=WORK;VOICE

### Pitfall 2: iCloud Requires App-Specific Passwords
**What goes wrong:** Authentication fails with 401/403 despite correct Apple ID password
**Why it happens:** iCloud requires app-specific passwords for third-party DAV access, even without 2FA
**How to avoid:** Document setup: user goes to appleid.apple.com, generates app-specific password, stores via `acrm sync setup`
**Warning signs:** HTTP 401 Unauthorized on first PROPFIND

### Pitfall 3: Contact Names May Be Empty or Structured Differently
**What goes wrong:** vCard FN field is empty, or N field has complex structure (prefix;first;middle;last;suffix)
**Why it happens:** Some contacts have only an organization or email, no name
**How to avoid:** Fall back chain: FN -> N (reconstructed) -> ORG -> EMAIL -> "Unknown Contact"
**Warning signs:** Empty `name` field causing validation failure

### Pitfall 4: Duplicate Detection False Positives
**What goes wrong:** Manually created contacts get duplicated on sync, or synced contacts match wrong manual contacts
**Why it happens:** Name matching is fuzzy; "John Smith" may match a different John Smith
**How to avoid:** Primary match on source_id (vCard UID). Only skip import if source_id matches. Never auto-merge with manual contacts -- flag them for user review instead.
**Warning signs:** Duplicate files appearing after sync

### Pitfall 5: Async Runtime Conflict
**What goes wrong:** Mixing sync main() with async reqwest calls
**Why it happens:** reqwest async requires tokio runtime; current main.rs is sync
**How to avoid:** Use `#[tokio::main]` on main, or use reqwest::blocking for simplicity. Recommend blocking client for a CLI tool.
**Warning signs:** "Cannot start a runtime from within a runtime" panic

### Pitfall 6: CardDAV Server Redirects
**What goes wrong:** Discovery returns relative URLs or redirects to a different host
**Why it happens:** iCloud routes to region-specific servers (p01-contacts.icloud.com, etc.)
**How to avoid:** Always resolve relative URLs against the base URL. Follow redirects. Use the url crate for URL resolution.
**Warning signs:** 301/302 responses during PROPFIND chain

## Code Examples

### CardDAV PROPFIND Request with reqwest
```rust
// Using reqwest blocking client for CLI simplicity
use reqwest::blocking::Client;
use reqwest::Method;

let client = Client::builder()
    .redirect(reqwest::redirect::Policy::limited(5))
    .build()?;

let propfind = Method::from_bytes(b"PROPFIND")?;

let response = client
    .request(propfind, "https://contacts.icloud.com/")
    .header("Depth", "0")
    .header("Content-Type", "application/xml; charset=utf-8")
    .basic_auth(&apple_id, Some(&app_password))
    .body(r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:current-user-principal/>
  </d:prop>
</d:propfind>"#)
    .send()?;
```

### vCard to Contact Mapping with calcard
```rust
use calcard::vcard::VCard;

fn map_vcard_to_contact(vcard_text: &str, uid: &str, etag: &str) -> anyhow::Result<Contact> {
    let vcard = VCard::parse(vcard_text)?;

    // Extract FN (formatted name) with fallback chain
    let name = vcard_fn(&vcard)
        .or_else(|| vcard_n_reconstructed(&vcard))
        .or_else(|| vcard_org(&vcard))
        .unwrap_or_else(|| "Unknown Contact".to_string());

    let contact = Contact {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        email: vcard_emails(&vcard),
        phone: vcard_phones(&vcard),
        company: vcard_org(&vcard).unwrap_or_default(),
        role: vcard_title(&vcard).unwrap_or_default(),
        source: "icloud".to_string(),
        source_id: uid.to_string(),
        // ... other fields default
    };

    Ok(contact)
}
```

### Duplicate Detection
```rust
fn find_existing_by_source_id(contacts: &[ContactFile], source_id: &str) -> Option<&ContactFile> {
    contacts.iter().find(|cf| cf.contact.source_id == source_id)
}

fn should_update(existing: &ContactFile, new_etag: &str) -> bool {
    // If etag field exists and matches, no update needed
    // Compare stored etag against server etag
    existing.contact.etag.as_deref() != Some(new_etag)
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| vcard_parser (4.0 only) | calcard (3.0 + 4.0) | 2024 | calcard handles iCloud's vCard 3.0 output |
| reqwest async only | reqwest::blocking available | Stable | Simplifies CLI tools, no tokio dependency if using blocking |
| Manual XML parsing | quick-xml + serde derive | Stable | Type-safe XML deserialization |

**Deprecated/outdated:**
- `carddav-rs` crate: Last updated 2018, abandoned
- `vobject` crate: API explicitly marked unstable, missing encodings

## Open Questions

1. **reqwest blocking vs async**
   - What we know: reqwest offers both blocking and async clients. CLI tools typically use blocking.
   - What's unclear: Whether calcard requires async. If so, tokio is needed regardless.
   - Recommendation: Use `reqwest::blocking` to avoid async complexity. Only switch to async if a dependency requires it.

2. **Schema changes for etag field**
   - What we know: The Contact struct and template need an `etag` field. The schema already has `source` and `source_id`.
   - What's unclear: Whether to add etag to the template (clutters it for manual contacts) or handle it only in the sync path.
   - Recommendation: Add `etag` to Contact struct with `#[serde(default)]`. Add to template under `# Source` section with empty default. Contacts created manually will just have empty etag.

3. **Handling contacts without names (org-only, email-only)**
   - What we know: Some iCloud contacts are companies or email-only entries.
   - What's unclear: Whether to skip these or import with a synthesized name.
   - Recommendation: Import with fallback name chain (FN -> N -> ORG -> EMAIL). Tag with `icloud-import` for easy filtering.

4. **First-run setup UX**
   - What we know: User needs Apple ID + app-specific password. keyring stores in Keychain.
   - What's unclear: Best UX for initial setup.
   - Recommendation: `acrm sync setup` interactive command that prompts for Apple ID and app-specific password, stores in keyring.

## vCard Property to CRM Field Mapping

| vCard Property | CRM Field | Notes |
|---------------|-----------|-------|
| FN | name | Formatted name; primary source |
| N | name (fallback) | Reconstruct as "Given Family" if FN missing |
| EMAIL | email[] | May have multiple; include TYPE parameter |
| TEL | phone[] | May have multiple; include TYPE parameter |
| ORG | company | First component of structured value |
| TITLE | role | Job title |
| BDAY | birthday | Parse date format (may be YYYY-MM-DD or YYYYMMDD) |
| URL | website | First URL property |
| NOTE | (body notes section) | Append to Notes section if present |
| UID | source_id | CardDAV unique identifier; critical for dedup |
| ADR | address[] | Reconstruct from structured address components |
| X-SOCIALPROFILE | linkedin/twitter/etc | Match by TYPE parameter |

## Sources

### Primary (HIGH confidence)
- [sabre.io CardDAV client guide](https://sabre.io/dav/building-a-carddav-client/) - Discovery flow, sync strategy, ETag/CTag usage
- [RFC 6352](https://datatracker.ietf.org/doc/html/rfc6352) - CardDAV protocol specification
- [vdirsyncer iCloud docs](https://vdirsyncer.pimutils.org/en/stable/tutorials/icloud.html) - iCloud endpoint URL, auth requirements
- [DAVx5 iCloud page](https://www.davx5.com/tested-with/icloud) - iCloud CardDAV compatibility

### Secondary (MEDIUM confidence)
- [calcard GitHub](https://github.com/stalwartlabs/calcard) - vCard parser API, version support
- [calcard docs.rs](https://docs.rs/calcard/latest/calcard/vcard/index.html) - API reference
- [keyring crate](https://docs.rs/keyring) - Credential storage API
- [quick-xml docs](https://docs.rs/quick-xml) - XML parsing with serde

### Tertiary (LOW confidence)
- [vcard_parser GitHub](https://github.com/kenianbei/vcard_parser) - Evaluated and rejected (4.0 only)
- [carddav-rs GitHub](https://github.com/gkbrk/carddav-rs) - Evaluated and rejected (abandoned 2018)

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM - calcard is relatively new (0.1.x), needs validation that it handles iCloud vCards well
- Architecture: HIGH - CardDAV discovery flow is well-documented and standard
- Pitfalls: HIGH - iCloud auth and vCard version issues are well-known in the DAV ecosystem
- Field mapping: MEDIUM - vCard 3.0 property names are standard but encoding/structure varies by source

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable protocols, crate versions may change)
