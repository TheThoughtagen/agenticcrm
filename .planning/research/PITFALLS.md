# Pitfalls Research: Adding Two-Way CardDAV Push Sync

**Domain:** Adding CardDAV PUT/DELETE push, ETag conflict detection, and selective sync to existing pull-only iCloud sync
**Researched:** 2026-03-07
**Confidence:** HIGH (verified against RFC 6352, sabre/dav official guide, existing codebase inspection, and web-verified iCloud-specific behaviors)

## Critical Pitfalls

### Pitfall 1: Lossy vCard Reconstruction Destroys iCloud Data on Push

**What goes wrong:**
The existing pull sync maps vCard fields to our Contact model via `vcard_map.rs`, but discards all unmapped vCard properties (photos, X-properties, custom fields, IMPP, RELATED, ADR structure, TEL/EMAIL TYPE parameters). When pushing back via PUT, the reconstructed vCard lacks these properties. iCloud replaces the server copy with our stripped-down version, destroying data the user added via their phone's Contacts app (contact photos, Siri suggestions, linked contacts, address labels like "home"/"work").

**Why it happens:**
The v1.0 pull sync was one-directional -- it only read vCards and mapped them to our model. There was no need to preserve unmapped properties because we never wrote back. Adding push changes this fundamentally: every PUT replaces the server's vCard wholesale.

**How to avoid:**
- Store the original raw vCard text alongside each synced contact. Add a `.sync/vcards/{source_id}.vcf` cache directory (gitignored) that retains the full vCard fetched during pull.
- On push, start from the cached original vCard, then merge only the fields our CRM tracks (name, email, phone, company, role, website, birthday, notes) back into it. This preserves photos, TYPE parameters, X-properties, and everything else.
- If no cached vCard exists (CRM-created contact, never pulled), construct a minimal but valid vCard 3.0 from scratch.
- The sabre/dav guide explicitly warns: "Retain the entire vCard... mapping back and forward tends to be a lossy process."

**Warning signs:**
- Building a `contact_to_vcard()` function that constructs vCards from scratch for existing synced contacts.
- No `.vcf` cache directory in the design.
- The vcard_map module only has `map_vcard_to_contact` with no reverse path that considers the original.

**Phase to address:**
Phase 1 (Push infrastructure). The vCard cache must be implemented before any PUT operations. Retrofitting the cache after push is already shipping means contacts synced in the gap lose data.

---

### Pitfall 2: Stale ETag on Push Causes 412 Failures or Silent Overwrites

**What goes wrong:**
The existing pull sync stores the ETag in contact frontmatter (`etag` field) at pull time. Between pulls, the contact may be edited on iCloud (phone, web, another device). When push sends a PUT with `If-Match: <stale-etag>`, iCloud returns 412 Precondition Failed. If the developer "fixes" this by dropping `If-Match` entirely, the PUT silently overwrites the server version.

**Why it happens:**
The v1.0 system stores ETags but only uses them for pull-side change detection (`dedup::should_update`). Developers assume the stored ETag is still valid for writes, but any server-side change between sync cycles invalidates it.

**How to avoid:**
- Before any PUT, fetch the current ETag from iCloud via a targeted PROPFIND on that specific resource. Compare with our stored ETag.
- If ETags match: push with `If-Match: <etag>`. The contact has not changed server-side.
- If ETags differ: the contact was modified on iCloud since our last pull. This is a conflict. Apply "CRM wins" policy by: (1) fetching the current server vCard, (2) logging the conflict, (3) pushing our version with `If-Match: <current-server-etag>`.
- On 412 response despite our checks (race condition), re-fetch ETag and retry up to 3 times.
- Always use `If-None-Match: *` when creating new contacts to prevent overwriting an existing resource at that URL.

**Warning signs:**
- PUT requests that use the frontmatter `etag` field directly without freshness check.
- No handling of HTTP 412 responses.
- PUT requests without any `If-Match` header.

**Phase to address:**
Phase 1 (Push infrastructure) for the ETag refresh mechanism. Phase 2 (Conflict detection) for the full conflict resolution flow.

---

### Pitfall 3: iCloud Rewrites vCards After PUT, Invalidating the Returned ETag

**What goes wrong:**
iCloud's CardDAV server normalizes vCards after accepting a PUT. It may reorder properties, canonicalize phone number formats, adjust line folding, add or modify `PRODID`, or strip properties it does not support. Per RFC 6352, when the stored vCard is not octet-identical to what was submitted, the server must NOT return a strong ETag in the PUT response. Many developers assume the PUT response always includes a usable ETag for subsequent operations, leading to sync state corruption.

**Why it happens:**
The RFC allows servers to modify vCards post-PUT. iCloud exercises this right aggressively. Developers test with simple vCards that happen to survive normalization unchanged, and miss the issue.

**How to avoid:**
- After every successful PUT (201 Created or 204 No Content), check if the response includes an ETag header.
- If ETag is present: store it.
- If ETag is absent: immediately issue a GET (or PROPFIND for just the ETag) on the same resource URL to retrieve the server's canonical version and its ETag. Update both the local vCard cache and the stored ETag.
- Update the contact's frontmatter `etag` field with the post-PUT ETag, not the pre-PUT one.
- This GET-after-PUT pattern is explicitly recommended by sabre/dav documentation.

**Warning signs:**
- Code that assumes `response.headers().get("etag")` always returns `Some`.
- No fallback GET after PUT.
- Tests that mock PUT responses with ETags but do not test the no-ETag path.

**Phase to address:**
Phase 1 (Push infrastructure). This must be part of the core PUT implementation, not a later fix.

---

### Pitfall 4: Accidental Mass Delete on iCloud When Push Encounters Unsynced Contacts

**What goes wrong:**
The CRM has contacts from multiple sources (manual entry, LinkedIn import, iCloud pull). When implementing push, if the sync logic interprets "contact exists in CRM but not in push queue" as "should be deleted from iCloud," or if a filter misconfiguration excludes most contacts from the push set, the system could DELETE hundreds of contacts from iCloud in one sync cycle.

**Why it happens:**
The most dangerous moment is the first push after implementing delete propagation. The sync logic must distinguish between: (a) contact was deleted from CRM and should be deleted from iCloud, (b) contact was never synced to iCloud and should be left alone, (c) contact does not match the current sync filter. Getting this wrong in any direction causes data loss.

**How to avoid:**
- Track sync state explicitly per contact. A contact should only be DELETE-eligible if it has `source: "icloud"` AND a valid `source_id` AND was previously successfully synced (has a stored ETag).
- Never delete contacts that were created locally and never pushed.
- Implement a hard safety limit: if more than N contacts (e.g., 10) would be deleted in a single sync cycle, abort and require `--force` confirmation. This catches filter misconfigurations.
- Always show a preview of deletes before executing: "Will delete 3 contacts from iCloud: [names]. Proceed? [y/N]"
- Log all deletes to a sync log file with timestamps for recovery.

**Warning signs:**
- Delete logic that iterates iCloud contacts and removes any without a local match.
- No confirmation prompt for deletes.
- No upper bound on batch deletes.

**Phase to address:**
Phase 1 (Push infrastructure) for the delete safeguards. Must be in place before any DELETE requests are sent.

---

### Pitfall 5: Push-Then-Pull Loop Creates Infinite Sync Cycles

**What goes wrong:**
Push modifies a contact on iCloud, which changes the server ETag. The next pull detects the ETag change and "updates" the local contact with the server version (which is actually the same data we just pushed, possibly reformatted by iCloud). This triggers another push because the local file was modified. The system oscillates between push and pull indefinitely, or at minimum does redundant work every cycle.

**Why it happens:**
The v1.0 pull logic uses `dedup::should_update()` which compares ETags. After a push, the ETag changes (because the server modified the vCard). Pull sees a new ETag and overwrites the local contact with the server version. If the server-normalized data differs from what we have locally (even just whitespace in the raw frontmatter), the contact file changes, triggering another push.

**How to avoid:**
- After a successful push, immediately update the local contact's `etag` field to the new server ETag (fetched via GET-after-PUT as described in Pitfall 3).
- The pull logic should compare ETags against the post-push ETag, not the pre-push ETag. If they match, skip the update.
- Consider a `last_push_etag` field or sync state entry that records "we pushed this ETag." During pull, if the server ETag matches `last_push_etag`, skip -- the change originated from us.
- Do not modify any CRM-only fields (tags, status, follow_up_cadence, interaction log) during pull updates. Only update vCard-mapped fields. This prevents pull from dirtying the file and triggering another push.

**Warning signs:**
- Sync logs showing the same contacts being "updated" on every sync cycle.
- Pull immediately after push shows contacts as "updated" instead of "unchanged."

**Phase to address:**
Phase 2 (Conflict detection). Requires coordinating push and pull ETags in a unified sync state model.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Skip vCard caching, reconstruct from Contact model | Simpler implementation, no cache directory | Destroys photos, TYPE params, X-properties on every push | Never -- data loss is unacceptable |
| PUT without If-Match header | Avoids complexity of ETag management | Silent data overwrites on iCloud, violates RFC 6352 | Never |
| Store sync state in frontmatter only (no external state file) | No new files to manage | Cannot track push-specific state (last_push_etag, pending deletes) without polluting contact files | Acceptable for basic ETag storage, but need external state for push metadata |
| Skip GET-after-PUT ETag refresh | One fewer HTTP request per push | Stale ETags cause 412 failures on next push, or pull detects false changes | Never -- iCloud normalizes aggressively |
| Push all contacts regardless of source | Simpler push logic | Pushes LinkedIn imports and manual contacts to iCloud unexpectedly | Never -- only push contacts with `source: "icloud"` or explicitly opted in |

## Integration Gotchas

Common mistakes when connecting to iCloud CardDAV for write operations.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| iCloud PUT create | Using the contact's UUID as the vCard filename | Generate a new UUID for the filename (`{uuid}.vcf`) and set the vCard UID property separately. The URL filename and vCard UID are independent identifiers. |
| iCloud PUT create | Missing `If-None-Match: *` header | Always include `If-None-Match: *` on creates to prevent overwriting an existing resource at that URL. Server returns 412 if resource already exists. |
| iCloud PUT update | Using the old ETag from frontmatter | Fetch current ETag via PROPFIND before PUT. Use fresh ETag in `If-Match`. |
| iCloud DELETE | Deleting without `If-Match` | Include `If-Match: <current-etag>` on DELETE to prevent deleting a contact that was modified since last sync. |
| iCloud vCard format | Sending vCard 4.0 format | iCloud uses vCard 3.0. Ensure `VERSION:3.0` in all pushed vCards. Key differences: vCard 3.0 requires both N and FN, uses `TYPE=` parameter syntax differently, and does not support all 4.0 properties. |
| iCloud Content-Type | Using `text/vcard` without charset | Use `Content-Type: text/vcard; charset=utf-8` on all PUT requests. |
| iCloud URL construction | Constructing PUT URL by appending to addressbook URL | The PUT URL for updates must match the exact `href` returned by PROPFIND. For creates, append `{uuid}.vcf` to the addressbook collection URL. |
| iCloud required fields | Sending vCard without N property | iCloud rejects vCards missing N (structured name). Always include both FN and N, even if N is just `LastName;FirstName;;;`. |

## Performance Traps

Patterns that work at small scale but fail with real contact lists.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Individual PUT per contact on first push | First push of 500+ contacts takes 10+ minutes | Batch pushes with delays. There is no multiput in CardDAV, but space requests 200ms apart to avoid rate limits | >100 contacts |
| PROPFIND Depth:1 to refresh all ETags before push | Works, but fetches ETags for all contacts when maybe only 3 changed | Track dirty contacts locally. Only refresh ETags for contacts that were modified since last sync | >500 contacts |
| No rate limit handling | iCloud returns 503 after ~50-100 rapid requests, sync fails midway | Implement exponential backoff on 429/503. Start with 1s delay, double on each retry, max 60s. Cap at 5 retries per request | >50 rapid requests |
| Fetching full vCard on every push for ETag check | Wastes bandwidth fetching vCard bodies when we only need ETags | Use targeted PROPFIND for `getetag` only, not full GET | >100 contacts |

## Security Mistakes

Domain-specific security issues for CardDAV write operations.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Logging vCard content (including phone/email) at debug level | PII in log files that may be committed or shared | Log contact names and source_ids only. Never log raw vCard content or frontmatter in production |
| Storing the app-specific password in sync state files | Credential leak via git or file sharing | Already mitigated -- password is in macOS Keychain via `keyring` crate. Ensure no refactoring moves it to config files |
| No TLS certificate validation on PUT/DELETE | MITM could intercept and modify contact data in transit | reqwest validates TLS by default. Do not add `.danger_accept_invalid_certs(true)` even for testing |
| Pushing contacts with `status: "archived"` to iCloud | Archived contacts reappear on user's phone | Filter out archived contacts from push. Only push `active` and `dormant` status contacts |

## UX Pitfalls

Common user experience mistakes when adding push sync.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Auto-push on every save without opt-in | User edits CRM-only fields (tags, notes), triggering unexpected iCloud writes | Auto-push must be opt-in via config flag. Default to manual `acrm sync push`. |
| No feedback during push | User thinks app froze during multi-contact push | Show progress: "Pushing 3/47 contacts... [contact name]" |
| Silent conflict resolution | User does not know CRM overwrote their phone edits | Always print conflicts: "Conflict: Jane Smith was modified on iCloud (ETag changed). CRM version pushed (CRM wins policy)." |
| Delete without undo | User deletes contact from CRM, push deletes from iCloud, no recovery | Archive instead of delete. Push only propagates actual deletes, not archives. Provide `acrm sync undo-delete` that re-pushes from archive |
| Pushing all contacts on first push | User with 500 CRM contacts floods iCloud with contacts they did not want synced | First push should require explicit opt-in per contact or per tag. Or only push contacts with `source: "icloud"` by default |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **PUT works:** Often missing If-Match header -- verify 412 response handling works
- [ ] **DELETE works:** Often missing confirmation prompt -- verify batch delete safety limit
- [ ] **ETag tracking:** Often missing GET-after-PUT fallback -- verify behavior when PUT response has no ETag header
- [ ] **Conflict detection:** Often missing "both sides changed" case -- verify behavior when local AND server both modified since last sync
- [ ] **Selective sync:** Often missing filter persistence -- verify filters are stored in config and survive restarts
- [ ] **New contact push:** Often missing vCard 3.0 N property -- verify iCloud accepts the generated vCard
- [ ] **Delete propagation:** Often missing source check -- verify only `source: "icloud"` contacts trigger iCloud deletes
- [ ] **Auto-push:** Often missing debouncing -- verify rapid saves do not trigger multiple concurrent pushes
- [ ] **Rate limiting:** Often missing backoff -- verify behavior when iCloud returns 503

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Data loss from lossy vCard push | HIGH | Restore from git history (`git log -- contacts/name.md`). Re-pull from iCloud to recover server-side data. Cannot recover iCloud-side data destroyed by push unless iCloud has its own backup/undo. |
| Mass accidental delete on iCloud | HIGH | If caught quickly, contacts may be in iCloud's "Recently Deleted" (recoverable for 30 days). Otherwise, restore from iPhone backup or Time Machine backup of `~/Library/Application Support/AddressBook/`. |
| Infinite sync loop | LOW | Stop sync. Clear sync state (delete `.sync/` directory). Re-run initial pull to re-establish baseline ETags. |
| 412 failures blocking all pushes | LOW | Re-pull to refresh all ETags. Then retry push. If persistent, force-push with fresh ETags from PROPFIND. |
| Stale ETag causing silent overwrite | MEDIUM | Check git history for the overwritten contact. Re-pull from iCloud to get current server state. Manually merge if needed. |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Lossy vCard reconstruction (P1) | Phase 1: Push infrastructure | Verify `.sync/vcards/` cache exists and is populated during pull. Round-trip test: pull contact, push unchanged, verify iCloud vCard is identical. |
| Stale ETag on push (P2) | Phase 1: Push infrastructure | Unit test: PUT with stale ETag returns 412. Integration test: modify contact on iCloud between pull and push, verify conflict detected. |
| iCloud rewrites vCard (P3) | Phase 1: Push infrastructure | Integration test: push a contact, check if PUT response has ETag. If not, verify GET-after-PUT retrieves the new ETag. |
| Accidental mass delete (P4) | Phase 1: Push infrastructure | Test: filter out all contacts, verify no deletes sent. Test: delete 15 contacts, verify safety limit triggers abort. |
| Infinite sync loop (P5) | Phase 2: Conflict detection | Test: push a contact, immediately pull, verify contact shows as "unchanged." Run 3 consecutive sync cycles and verify no oscillation. |
| vCard 3.0 format errors (Integration) | Phase 1: Push infrastructure | Test: create contact in CRM, push to iCloud, verify iCloud Contacts app displays it correctly with all fields. |
| Rate limiting (Performance) | Phase 1: Push infrastructure | Test: push 20+ contacts rapidly, verify backoff kicks in on 503. Verify sync completes despite throttling. |
| Auto-push without opt-in (UX) | Phase 3: Selective sync + auto-push | Verify auto-push config defaults to `false`. Verify toggling config requires explicit user action. |

## Sources

- [RFC 6352 - CardDAV Specification](https://www.rfc-editor.org/rfc/rfc6352) -- ETag requirements, If-Match semantics, PUT/DELETE behavior (HIGH confidence)
- [sabre/dav: Building a CardDAV Client](https://sabre.io/dav/building-a-carddav-client/) -- Sync algorithm, vCard preservation warning, GET-after-PUT pattern, UID/URL independence (HIGH confidence)
- [DAVx5 Technical Documentation](https://manual.davx5.com/technical_information.html) -- If-None-Match for creates, CTag-based change detection (HIGH confidence)
- [Apple Developer Forums: Rate Limit Exceeded for CardDAV](https://developer.apple.com/forums/thread/722170) -- iCloud rate limiting exists but limits are undocumented (MEDIUM confidence)
- [The Eclectic Light Company: iCloud Throttling](https://eclecticlight.co/2024/02/22/icloud-does-throttle-data-syncing-after-all/) -- iCloud throttles aggressively, stops responding entirely rather than slowing (MEDIUM confidence)
- [Apple Developer Forums: FN property empty](https://developer.apple.com/forums/thread/724626) -- iCloud strict vCard 3.0 validation, FN/N required (MEDIUM confidence)
- [DAVx5 iCloud compatibility page](https://www.davx5.com/tested-with/icloud) -- iCloud DNS/SRV issues, general interoperability notes (MEDIUM confidence)
- Existing codebase inspection: `src/sync/carddav.rs`, `src/sync/vcard_map.rs`, `src/commands/sync.rs`, `src/sync/dedup.rs` (HIGH confidence)

---
*Pitfalls research for: Adding two-way CardDAV push sync to AgenticCRM v1.1*
*Researched: 2026-03-07*
