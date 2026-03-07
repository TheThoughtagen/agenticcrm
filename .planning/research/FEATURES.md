# Feature Landscape

**Domain:** Two-way iCloud CardDAV sync (push, conflict detection, selective filtering, auto-push)
**Researched:** 2026-03-07
**Overall confidence:** HIGH (RFC 6352 is stable standard; existing pull sync validates protocol understanding)

## Existing Foundation (Already Built in v1.0)

These are the features this milestone builds on top of. All have been validated and are working.

| Feature | Implementation | Relevant Code |
|---------|---------------|---------------|
| CardDAV PROPFIND discovery | 3-step principal/home/collection chain | `sync/carddav.rs` |
| vCard list fetch (PROPFIND depth 1) | Returns href + ETag per contact | `CardDavClient::fetch_vcard_list()` |
| vCard download (GET) | Fetches raw vCard text | `CardDavClient::fetch_vcard()` |
| vCard-to-Contact mapping | calcard parsing, name fallback chain | `sync/vcard_map.rs` |
| Contact dedup by source_id | Matches by CardDAV UID stored in frontmatter | `sync/dedup.rs` |
| ETag storage in frontmatter | Stored as `etag` field on each contact | schema + frontmatter preservation |
| Keychain credential storage | apple_id in config, password in macOS Keychain | `sync/config.rs` |
| `acrm sync` pull command | Full pull-sync with --force and --dry-run | `commands/sync.rs` |
| Raw frontmatter preservation | YAML comments and field order survive edits | `frontmatter.rs` |

## Table Stakes

Features that any two-way sync implementation must have. Without these, push sync is either broken or dangerous.

### Push: Create New Contacts on iCloud

| Feature | Why Expected | Complexity | Dependencies |
|---------|--------------|------------|-------------|
| Contact-to-vCard generation | Must produce valid vCard 3.0 for iCloud to accept | Med | calcard builder module, vcard_map reverse mapping |
| PUT with If-None-Match: * | Prevents overwriting existing server resource on create | Low | New method on CardDavClient |
| UUID generation for href | Client determines both URL path and UID in vCard | Low | uuid crate (already a dependency) |
| Store source_id on newly pushed contacts | Track the server-side UID for future sync | Low | Existing frontmatter update logic |
| Store returned ETag after PUT | Server returns new ETag; must save for future conflict checks | Low | Parse ETag from 201 response header |

**Expected behavior:** When a contact has `source: manual` (or no source_id), `acrm sync push` generates a vCard, PUTs it to `{addressbook_url}/{uuid}.vcf` with `If-None-Match: *`, saves the returned ETag and sets `source: icloud` + `source_id: {uuid}`.

### Push: Update Existing Contacts on iCloud

| Feature | Why Expected | Complexity | Dependencies |
|---------|--------------|------------|-------------|
| PUT with If-Match: "{etag}" | Conditional update prevents overwriting server changes | Low | Existing ETag from frontmatter |
| Full vCard replacement on PUT | CardDAV requires replacing the entire vCard, not field-level patches | Med | Contact-to-vCard generation |
| Handle 412 Precondition Failed | Server rejects if ETag mismatch (someone edited on phone) | Med | Conflict detection flow |
| Update stored ETag after successful PUT | Track the new server version | Low | Parse ETag from 204 response header |

**Expected behavior:** For contacts with `source: icloud` and a `source_id`, PUT the full vCard to `{addressbook_url}/{source_id}.vcf` with `If-Match: "{etag}"`. On 204, update local ETag. On 412, enter conflict detection flow.

### Push: Delete Contacts on iCloud

| Feature | Why Expected | Complexity | Dependencies |
|---------|--------------|------------|-------------|
| DELETE with If-Match: "{etag}" | Prevents deleting a contact someone else modified | Low | Existing ETag + source_id |
| Track deletion intent | Must know which contacts were deleted/archived locally since last sync | Med | Needs a deletion log or tombstone mechanism |
| Handle 404 on delete (already gone) | Server contact may already be deleted; this is not an error | Low | HTTP status handling |

**Expected behavior:** When a contact with `source: icloud` is deleted or archived locally, `acrm sync push` sends DELETE to `{addressbook_url}/{source_id}.vcf` with `If-Match`. On success (204) or already-gone (404), clean up local tracking. On 412, warn user.

### ETag-Based Conflict Detection

| Feature | Why Expected | Complexity | Dependencies |
|---------|--------------|------------|-------------|
| Pre-push ETag check | Before pushing, compare local ETag with server's current ETag | Med | PROPFIND to get current server ETags |
| CRM-wins conflict resolution (default) | PROJECT.md mandates: "CRM is source of truth" | Low | Policy decision, already documented |
| Conflict warning output | Show user which contacts had server-side changes that will be overwritten | Low | CLI output formatting |
| --force flag to skip conflict check | Power users can override without prompt | Low | Existing pattern from pull sync |

**Expected behavior:** Before pushing, fetch current ETags from server via PROPFIND. If server ETag differs from stored ETag AND local data has changed, warn user: "Contact X was modified on iCloud. CRM version will overwrite. Use --force to skip." Default behavior: CRM wins, but warn. This matches PROJECT.md constraint.

### `acrm sync push` Command

| Feature | Why Expected | Complexity | Dependencies |
|---------|--------------|------------|-------------|
| Manual push trigger | Users must be able to explicitly push changes | Low | New SyncAction variant |
| --dry-run flag | Show what would be pushed without actually pushing | Low | Existing pattern from pull |
| --force flag | Skip conflict checks, push everything | Low | Existing pattern from pull |
| Push result summary | "X created, Y updated, Z deleted, W conflicts" | Low | Existing SyncResult pattern |
| JSON output support | `--format json` for agent consumption | Low | Existing OutputFormat system |

**Expected behavior:** `acrm sync push [--dry-run] [--force]` discovers address book, fetches current server ETags, compares with local state, pushes changes (create/update/delete), reports results. Follows same UX patterns as existing pull sync.

## Differentiators

Features that go beyond basic push sync. Valuable but not strictly required for correctness.

### Selective Sync Filtering

| Feature | Value Proposition | Complexity | Dependencies |
|---------|-------------------|------------|-------------|
| Filter push by tags | Only push contacts tagged "sync-to-icloud" | Low | Tag matching on Contact struct |
| Filter push by status | Only push active contacts (not archived/dormant) | Low | Status matching |
| Filter pull by existing config | Apply same filters to pull direction | Low | Config integration |
| Sync filter config in sync.toml | Persist filter preferences | Low | Extend existing config parser |
| --tag and --status CLI flags | Override config filters per-run | Low | clap argument additions |

**Expected behavior:** Config file (`~/.config/acrm/sync.toml`) gains optional filter fields:

```toml
apple_id = "user@icloud.com"

[filters]
push_tags = ["sync-to-icloud"]    # only push contacts with these tags
push_status = ["active"]           # only push contacts with these statuses
pull_tags = []                     # empty = pull all
pull_status = []                   # empty = pull all
```

CLI flags override config: `acrm sync push --tag work --status active`. Default (no filters) = sync everything, matching current pull behavior.

### Auto-Push on Save

| Feature | Value Proposition | Complexity | Dependencies |
|---------|-------------------|------------|-------------|
| File watcher for contacts/ directory | Detect when contact files change on disk | Med | notify crate + debouncing |
| Auto-push on file modification | Changes propagate to iCloud without manual `sync push` | Med | File watcher + push logic |
| `acrm sync watch` daemon command | Long-running process that watches and pushes | Med | New CLI subcommand |
| Debounced push (batch changes) | Avoid pushing on every keystroke during bulk edits | Med | notify-debouncer-mini |
| Config toggle for auto-push | Enable/disable in sync.toml | Low | Config extension |

**Expected behavior:** `acrm sync watch` starts a long-running process that watches the `contacts/` directory. When files change (create, modify, delete), it waits a configurable debounce period (default 5 seconds), then pushes changed contacts to iCloud. Only pushes contacts that match sync filters.

**Alternative (simpler):** Instead of a file watcher daemon, auto-push could be a flag on write commands: `acrm edit --push`, `acrm log --push`, `acrm delete --push`. This avoids the complexity of a daemon process and the notify dependency. The PROJECT.md says "optional auto-push on save config" which is ambiguous -- either approach satisfies it.

### Bidirectional Sync in One Command

| Feature | Value Proposition | Complexity | Dependencies |
|---------|-------------------|------------|-------------|
| `acrm sync` does pull then push | Single command for full sync | Low | Compose existing pull + new push |
| CTag-based quick check | Skip full sync if nothing changed on server | Low | Parse CTag from PROPFIND |
| Sync direction flags | `acrm sync --pull-only` / `--push-only` | Low | CLI flags |

**Expected behavior:** Default `acrm sync` (with no subcommand) changes from pull-only to pull-then-push. First pull new/changed contacts from iCloud, then push local changes to iCloud. CTag check avoids unnecessary pull when server hasn't changed. `acrm sync push` and `acrm sync pull` remain available for directional sync.

## Anti-Features

Features to explicitly NOT build for this milestone.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Field-level merge on conflict | Massively complex; which fields win from which side? | CRM wins entirely. User is warned, can re-pull after. |
| Server-wins conflict mode | Contradicts PROJECT.md constraint "CRM is source of truth" | Always CRM wins. If user wants server version, re-pull with --force. |
| Real-time bidirectional sync | Requires persistent connection, webhook support Apple doesn't offer | Use manual `acrm sync` or periodic `acrm sync watch` with polling. |
| Sync multiple address books | iCloud typically has one default address book; multi-book is rare | Sync the primary address book only. |
| Photo sync | vCard PHOTO property is large (base64), slow to transfer, markdown CRM has no photo concept | Skip PHOTO property in both directions. |
| vCard group/distribution list sync | X-ADDRESSBOOKSERVER-GROUP is Apple-proprietary; complex semantics | Map groups to tags on pull only; don't push tag-to-group. |
| WebDAV-Sync (RFC 6578) sync-token | More efficient than CTag+ETag but adds protocol complexity | Use CTag+ETag approach; sync-token is an optimization for large address books. At personal scale (<50K contacts), CTag+ETag is fast enough. |
| Async/tokio for HTTP requests | PROJECT.md records reqwest blocking as a good decision | Keep blocking reqwest. Push adds a few more HTTP calls per sync, not enough to justify async complexity. |

## Feature Dependencies

```
Contact-to-vCard generation ──> Push create (need vCard body for PUT)
Contact-to-vCard generation ──> Push update (need vCard body for PUT)

Push create ──> Bidirectional sync (sync = pull + push)
Push update ──> Bidirectional sync
Push delete ──> Bidirectional sync

ETag conflict detection ──> Push update (must check before overwriting)
ETag conflict detection ──> Push delete (must check before deleting)

Deletion tracking ──> Push delete (must know what was deleted locally)

Selective sync filters ──> Auto-push (watcher should respect filters)
Push logic ──> Auto-push (watcher triggers push)

Existing pull sync ──> Bidirectional sync (already built)
Existing ETag storage ──> Conflict detection (already built)
Existing source_id dedup ──> Push update/delete routing (already built)
```

## MVP Recommendation

Prioritize in this order based on dependencies and risk:

### Must Build (Core push sync)

1. **Contact-to-vCard generation** -- Reverse of existing vcard_map. Map CRM Contact fields back to vCard 3.0 properties. calcard has a builder module for this. Test with iCloud acceptance (Apple is strict about vCard validity -- FN, N, VERSION are mandatory).
2. **CardDavClient PUT/DELETE methods** -- Add `put_vcard()` and `delete_vcard()` to existing client. These are straightforward HTTP methods with If-Match/If-None-Match headers.
3. **ETag conflict detection** -- Pre-push PROPFIND to fetch current server ETags, compare with local, warn on mismatch. Simple diff logic.
4. **Deletion tracking** -- Lightweight approach: scan for contacts with `source: icloud` + `source_id` that no longer exist as files. Compare against server listing. No need for a separate tombstone file.
5. **`acrm sync push` command** -- Wire it all together. Create/update/delete flow with --dry-run and --force.

### Should Build (Selective filtering)

6. **Sync filter config** -- Extend sync.toml with tag/status filters. Simple config parsing.
7. **Filter application to push and pull** -- Apply filters before sync operations. Straightforward predicate matching.

### Nice to Have (Auto-push)

8. **`acrm sync watch` OR `--push` flag on commands** -- The simpler `--push` flag approach is recommended over a file watcher daemon. Less complexity, fewer dependencies, no daemon management.

**Defer:**
- **Bidirectional `acrm sync`** (pull+push in one command): Low risk but changes existing behavior. Add after push is stable.
- **CTag quick-check optimization**: Useful but not required. Can be added later without breaking changes.
- **WebDAV-Sync tokens**: Over-optimization for personal scale.

## Sources

- [RFC 6352 - CardDAV Specification](https://www.rfc-editor.org/rfc/rfc6352) -- Authoritative standard for PUT/DELETE/If-Match semantics (HIGH confidence)
- [Building a CardDAV Client - sabre.io](https://sabre.io/dav/building-a-carddav-client/) -- Practical implementation guide covering create/update/delete/sync algorithm (HIGH confidence)
- [Building a CardDAV Client - CalConnect DevGuide](https://devguide.calconnect.org/CardDAV/building-a-carddav-client/) -- Community implementation guide (HIGH confidence)
- [DAVx5 Technical Information](https://manual.davx5.com/technical_information.html) -- Real-world sync client behavior, ETag handling (MEDIUM confidence)
- [Google CardDAV API](https://developers.google.com/people/carddav) -- Cross-reference for CardDAV PUT/DELETE patterns (MEDIUM confidence)
- [calcard crate](https://docs.rs/calcard/latest/calcard/) -- Rust library for vCard parsing AND building; already used for pull sync (HIGH confidence)
- [notify crate](https://github.com/notify-rs/notify) -- File system watcher for auto-push feature (HIGH confidence)
- Existing codebase: `src/sync/carddav.rs`, `src/sync/vcard_map.rs`, `src/commands/sync.rs` -- Direct reading (HIGH confidence)
- PROJECT.md constraints: "CRM is source of truth", "reqwest blocking", existing CLI patterns (HIGH confidence)
