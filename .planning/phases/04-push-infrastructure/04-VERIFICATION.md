---
phase: 04-push-infrastructure
verified: 2026-03-07T16:15:00Z
status: gaps_found
score: 3/5 must-haves verified
gaps:
  - truth: "A new CRM contact with no iCloud history can be pushed to iCloud and appears as a contact in iCloud"
    status: partial
    reason: "compute_push_changeset correctly identifies creates, put_vcard exists, contact_to_vcard works, but execute_push is a stub that returns hardcoded zeros -- the pieces are not wired together"
    artifacts:
      - path: "src/sync/push.rs"
        issue: "execute_push (lines 158-176) returns hardcoded empty PushResult; never calls put_vcard, never updates frontmatter, never writes cache"
    missing:
      - "Implement execute_push to call contact_to_vcard + put_vcard for creates, update contact frontmatter (source, source_id, etag), and write cache"
  - truth: "An updated CRM contact can be pushed to iCloud and the changes appear in iCloud"
    status: partial
    reason: "Same root cause: execute_push is a stub -- merge_contact_to_vcard and put_vcard exist but are never called from execute_push"
    artifacts:
      - path: "src/sync/push.rs"
        issue: "execute_push never calls merge_contact_to_vcard or put_vcard for updates"
    missing:
      - "Implement execute_push update path: merge_contact_to_vcard with cached text, PUT with If-Match etag, update cache on success"
  - truth: "A deleted/archived CRM contact triggers removal of the corresponding iCloud contact"
    status: partial
    reason: "Same root cause: execute_push is a stub -- delete_vcard exists but is never called from execute_push"
    artifacts:
      - path: "src/sync/push.rs"
        issue: "execute_push never calls delete_vcard for deletes"
    missing:
      - "Implement execute_push delete path: DELETE with If-Match etag, remove cache on success"
---

# Phase 4: Push Infrastructure Verification Report

**Phase Goal:** CRM can serialize contacts to vCard 3.0 and write them to iCloud via CardDAV PUT/DELETE with lossless round-tripping
**Verified:** 2026-03-07T16:15:00Z
**Status:** gaps_found
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A new CRM contact can be pushed to iCloud and appears there | PARTIAL | compute_push_changeset identifies creates (tested), contact_to_vcard serializes (tested), put_vcard sends PUT (implemented), but execute_push is a stub returning hardcoded zeros |
| 2 | An updated CRM contact can be pushed to iCloud | PARTIAL | merge_contact_to_vcard works (tested), put_vcard with If-Match exists, but execute_push is a stub |
| 3 | A deleted/archived CRM contact triggers removal from iCloud | PARTIAL | compute_push_changeset identifies deletes (tested), delete_vcard sends DELETE (implemented), but execute_push is a stub |
| 4 | Pushing preserves iCloud-only data via vCard cache | VERIFIED | merge_contact_to_vcard preserves X-ABUID, X-ABLABEL, PHOTO (test_merge_preserves_non_crm_properties passes); pull caches raw vCards (sync.rs lines 102-107); round-trip tested |
| 5 | User sees conflict warning when iCloud has newer version | VERIFIED | compute_push_changeset detects ETag mismatches and populates conflicts list (test_icloud_contact_with_etag_mismatch_goes_to_conflicts passes); put_vcard returns Err on 412 |

**Score:** 3/5 truths verified (2 fully verified, 3 partial due to same root cause)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/sync/vcard_write.rs` | Contact-to-vCard serialization and cache | VERIFIED | 261 lines, 22 tests, exports contact_to_vcard, merge_contact_to_vcard, read/write/delete_cached_vcard, cache_dir |
| `src/sync/carddav.rs` | put_vcard and delete_vcard methods | VERIFIED | put_vcard with If-Match/If-None-Match (line 182), delete_vcard with If-Match and 404 idempotency (line 231), build_vcard_url (line 264), 3 new tests |
| `src/sync/push.rs` | Push changeset computation and execution | STUB (partial) | compute_push_changeset is fully implemented and tested (9 tests). execute_push is a stub: all params prefixed with underscore, returns hardcoded empty PushResult (lines 158-176) |
| `src/sync/mod.rs` | Module registration | VERIFIED | Contains `pub mod push;` and `pub mod vcard_write;` |
| `src/commands/sync.rs` | Pull caches vCards | VERIFIED | Lines 102-107 call vcard_write::write_cached_vcard during pull (skipped in dry_run) |
| `.gitignore` | .sync/ excluded | VERIFIED | Line 19: `.sync/` |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| vcard_write.rs | calcard VCard | VCard::parse and to_string | WIRED | Line 108: `VCard { entries }`, line 109: `vcard.to_string()`, line 115: `VCard::parse(cached_vcard_text)` |
| vcard_write.rs | Contact model | contact field mapping | WIRED | Lines 50-106 map contact.email, phone, company, role, website, birthday |
| push.rs | vcard_write.rs | contact_to_vcard and merge_contact_to_vcard | WIRED (in compute only) | Lines 113-115 call merge_contact_to_vcard and contact_to_vcard for change detection. Not called in execute_push. |
| push.rs | carddav.rs | CardDavClient::put_vcard and delete_vcard | NOT WIRED | execute_push has `_client: &CardDavClient` (underscore = unused). put_vcard and delete_vcard are never called. |
| push.rs | vcard_write.rs cache | read/write/delete_cached_vcard | PARTIAL | read_cached_vcard called in compute_push_changeset (line 111). write/delete_cached_vcard never called from execute_push. |
| commands/sync.rs | vcard_write.rs | write_cached_vcard during pull | WIRED | Line 104: `vcard_write::write_cached_vcard(&crm_root, &uid, &vcard_text)` |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| PUSH-01 | 04-02, 04-03 | Push new CRM contact to iCloud (creates vCard on server) | PARTIAL | contact_to_vcard + put_vcard exist, compute_push_changeset identifies creates, but execute_push is stub |
| PUSH-02 | 04-02, 04-03 | Push updated CRM contact to iCloud (replaces vCard on server) | PARTIAL | merge_contact_to_vcard + put_vcard with If-Match exist, compute_push_changeset identifies updates, but execute_push is stub |
| PUSH-03 | 04-02, 04-03 | Push CRM deletion/archive to iCloud (removes contact from server) | PARTIAL | delete_vcard with If-Match exists, compute_push_changeset identifies deletes, but execute_push is stub |
| PUSH-04 | 04-01, 04-03 | Push preserves iCloud data not mapped to CRM via vCard cache | SATISFIED | merge_contact_to_vcard preserves X-properties, PHOTO, TYPE params (tested). Pull caches raw vCards. |
| PUSH-05 | 04-02, 04-03 | User sees conflict warning when iCloud has newer version | SATISFIED | compute_push_changeset detects ETag mismatches. put_vcard/delete_vcard return Err on 412. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| src/sync/push.rs | 158-176 | execute_push is a stub: all params are `_prefixed`, returns hardcoded empty PushResult | BLOCKER | The function that wires compute+serialize+PUT/DELETE together does nothing. Creates/updates/deletes cannot actually execute. |
| src/sync/push.rs | 165 | Comment: "Implementation will be called by the CLI push command in Phase 5" | INFO | Intentional deferral per summary, but contradicts phase goal of building "push infrastructure" |

### Human Verification Required

### 1. CardDAV PUT creates contact on iCloud
**Test:** Run `acrm sync push` (once implemented) with a new CRM-only contact and check iCloud
**Expected:** Contact appears in iCloud Contacts
**Why human:** Requires live iCloud account and network access

### 2. CardDAV DELETE removes contact from iCloud
**Test:** Archive a synced contact and run push
**Expected:** Contact disappears from iCloud Contacts
**Why human:** Requires live iCloud account

### 3. Rate limiting behavior
**Test:** Push 10+ contacts in sequence
**Expected:** No 429 errors from iCloud, 200ms delay between requests
**Why human:** Requires live iCloud server interaction to validate rate limit defense

## Gaps Summary

All three gaps share the same root cause: `execute_push` in `src/sync/push.rs` is a stub function. The function signature, types (PushChangeset, PushResult, PushDetail), and changeset computation are all complete and well-tested (9 unit tests). The individual building blocks -- contact_to_vcard, merge_contact_to_vcard, put_vcard, delete_vcard, cache read/write/delete -- are all implemented and tested.

What is missing is the ~50-80 lines of code inside execute_push that:
1. Iterates over changeset.creates, serializes each via contact_to_vcard, calls put_vcard, updates contact frontmatter (source, source_id, etag), writes cache
2. Iterates over changeset.updates, serializes each via merge_contact_to_vcard, calls put_vcard with If-Match, updates cache
3. Iterates over changeset.deletes, calls delete_vcard, removes cache
4. Handles conflicts (skip or force), handles 412 errors gracefully, populates PushResult with counts and details

The summary explicitly states this was an intentional deferral to Phase 5. However, the ROADMAP success criteria describe end-to-end push behavior, and this function is the critical link that makes that behavior possible. Without it, the infrastructure components exist but cannot be used together.

---

_Verified: 2026-03-07T16:15:00Z_
_Verifier: Claude (gsd-verifier)_
