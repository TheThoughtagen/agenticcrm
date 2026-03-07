# Project Research Summary

**Project:** AgenticCRM v1.1 -- Two-Way iCloud CardDAV Sync
**Domain:** CardDAV protocol write operations (PUT/DELETE), ETag-based conflict detection, selective sync filtering
**Researched:** 2026-03-07
**Confidence:** HIGH

## Executive Summary

AgenticCRM v1.1 adds push sync to the existing pull-only iCloud CardDAV integration. The core task is well-scoped: add PUT and DELETE HTTP methods to the existing `CardDavClient`, build a reverse vCard mapping (Contact-to-vCard using the calcard crate already in use), and wire up a `sync push` CLI subcommand. Zero new Cargo dependencies are required -- the existing stack (reqwest blocking, calcard 0.3.2, quick-xml, uuid, clap) handles everything. RFC 6352 is a stable, well-documented standard, and the v1.0 pull sync already validates that our protocol understanding and iCloud authentication work correctly.

The recommended approach is to build push infrastructure first (vCard serialization, PUT/DELETE methods, vCard cache for lossless round-tripping), then layer on push orchestration with ETag conflict detection, and finally add selective filtering and convenience features. The architecture follows the existing codebase patterns closely: two new modules (`sync/push.rs` for orchestration, `sync/filter.rs` for predicates), modifications to four existing modules, and no structural changes to the flat-file contact storage model. The "CRM wins" conflict resolution policy (already documented in PROJECT.md) keeps conflict handling simple.

The primary risk is data loss from lossy vCard reconstruction. When pushing a contact back to iCloud, a naive implementation constructs a minimal vCard from CRM fields, destroying photos, TYPE parameters, and Apple-proprietary properties that iCloud stored. The mitigation is a vCard cache (`.sync/vcards/{source_id}.vcf`) that preserves the original server vCard and merges CRM field changes into it before pushing. Secondary risks include accidental mass deletion (mitigated by safety limits and confirmation prompts) and infinite sync loops from ETag churn (mitigated by GET-after-PUT to capture iCloud's normalized ETag). All critical pitfalls have clear, well-documented prevention strategies.

## Key Findings

### Recommended Stack

No changes to `Cargo.toml`. Every technology needed for v1.1 is already a dependency. This is the strongest possible position for a milestone -- zero dependency risk.

**Core technologies (all existing):**
- **reqwest 0.13.2 (blocking):** PUT/DELETE with If-Match/If-None-Match conditional headers. Already creates custom methods (PROPFIND); standard methods are simpler.
- **calcard 0.3.2:** VCard construction via `VCard { entries: Vec<VCardEntry> }` builder pattern. Serialization via `write_to()` with explicit vCard 3.0 version targeting. Source code verified locally.
- **quick-xml 0.39.2:** Parse XML error responses from PUT/DELETE (same role as existing PROPFIND response parsing).
- **clap 4 (derive):** New `sync push` subcommand with --force, --dry-run, --tag, --status flags.

**Explicitly rejected:** tokio/async (unnecessary complexity), toml crate (hand-parser sufficient for 4 config fields), notify (file watcher deferred), diff/merge crates (CRM-wins means no merge needed).

See: `.planning/research/STACK.md`

### Expected Features

**Must have (table stakes):**
- Contact-to-vCard 3.0 generation (reverse of existing vcard_map)
- PUT with If-None-Match for creating new contacts on iCloud
- PUT with If-Match for updating existing contacts on iCloud
- DELETE with If-Match for removing archived contacts from iCloud
- ETag-based conflict detection with CRM-wins resolution
- `acrm sync push` command with --dry-run and --force flags
- Deletion tracking via source/source_id/etag presence in frontmatter

**Should have (differentiators):**
- Selective sync filtering by tags and status (config + CLI override)
- Bidirectional `acrm sync` that does pull-then-push in one command

**Defer (v2+):**
- Auto-push via file watcher daemon (`acrm sync watch`) -- use simpler `--push` flag on commands if needed
- CTag-based quick-check optimization (skip pull when server unchanged)
- WebDAV-Sync token support (over-optimization at personal scale)
- Field-level merge on conflicts
- Photo sync, vCard group sync, multi-address-book sync

See: `.planning/research/FEATURES.md`

### Architecture Approach

The architecture extends the existing v1.0 sync module with two new files and modifications to four existing files. Component boundaries are clean: `CardDavClient` handles HTTP transport only, `vcard_map` handles bidirectional Contact-to-VCard mapping, `push` orchestrates the sync flow, and `filter` provides pure predicate functions. Data flows from local contact files through filtering, ETag comparison, vCard serialization, and HTTP PUT/DELETE to iCloud. The flat-file architecture is preserved -- sync state (source, source_id, etag) stays in contact frontmatter, with a gitignored vCard cache for lossless round-tripping.

**Major components:**
1. **sync/push.rs (NEW)** -- Push orchestration: load contacts, apply filters, compare ETags, determine create/update/delete actions, call CardDavClient, update frontmatter
2. **sync/filter.rs (NEW)** -- Pure predicate functions for tag/status filtering, constructed from config
3. **sync/vcard_map.rs (MODIFY)** -- Add `map_contact_to_vcard()` reverse mapping using calcard builder API
4. **sync/carddav.rs (MODIFY)** -- Add `put_vcard()` and `delete_vcard()` methods with conditional headers
5. **sync/config.rs (MODIFY)** -- Add auto_push, push_tags, push_statuses config parsing
6. **commands/sync.rs (MODIFY)** -- Add `run_push()` and wire up CLI subcommand

See: `.planning/research/ARCHITECTURE.md`

### Critical Pitfalls

1. **Lossy vCard reconstruction destroys iCloud data** -- Cache original vCards in `.sync/vcards/{source_id}.vcf` during pull. On push, merge CRM fields into the cached original rather than constructing from scratch. This preserves photos, TYPE params, and X-properties.
2. **iCloud rewrites vCards after PUT, invalidating ETags** -- After every PUT, check if response includes an ETag. If absent (common with iCloud), immediately GET the resource to retrieve the canonical ETag. Store the post-normalization ETag, not the pre-PUT one.
3. **Accidental mass delete on iCloud** -- Only DELETE contacts with `source: "icloud"` AND valid source_id AND stored ETag. Implement a hard safety limit (abort if >10 deletes without --force). Show delete preview before executing.
4. **Push-then-pull infinite sync loop** -- After push, update local ETag to match server's post-normalization ETag. Pull should skip contacts whose ETag matches the last-pushed ETag. Only update vCard-mapped fields during pull, never CRM-only fields.
5. **Stale ETag causes 412 failures** -- Fetch current server ETag via PROPFIND before each PUT. Compare with stored ETag to detect conflicts. On 412 (race condition), retry with fresh ETag up to 3 times.

See: `.planning/research/PITFALLS.md`

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Push Infrastructure (vCard Serialization + HTTP Methods + Cache)

**Rationale:** Foundation for all push operations. No push features work without vCard generation, PUT/DELETE transport, and the vCard cache for lossless round-tripping. These are independent, testable units. The vCard cache MUST be built here -- retrofitting after push ships means contacts synced in the gap lose iCloud data.
**Delivers:** `map_contact_to_vcard()` function, `put_vcard()` and `delete_vcard()` on CardDavClient, `.sync/vcards/` cache directory populated during pull, GET-after-PUT ETag refresh, vCard 3.0 compliance
**Addresses:** Contact-to-vCard generation, PUT with If-Match/If-None-Match, DELETE with If-Match, vCard caching
**Avoids:** Lossy vCard reconstruction (Pitfall 1), iCloud ETag rewriting (Pitfall 3), vCard 3.0 format errors

### Phase 2: Push Orchestration + Conflict Detection

**Rationale:** Depends on Phase 1 infrastructure. This is the integration phase that wires vCard generation and HTTP methods into a cohesive push flow with ETag-based conflict detection. Delivers the user-facing `acrm sync push` command.
**Delivers:** `sync/push.rs` orchestration module, `acrm sync push` command with --dry-run and --force, conflict detection and CRM-wins resolution, deletion tracking and safety limits, push result reporting
**Addresses:** ETag conflict detection, deletion propagation, `acrm sync push` command, push result summary
**Avoids:** Stale ETag failures (Pitfall 2), accidental mass delete (Pitfall 4), push-then-pull loops (Pitfall 5)

### Phase 3: Selective Sync + Bidirectional Command

**Rationale:** Filtering and bidirectional sync are refinements that layer on top of working push. Lower risk, lower complexity. Can ship push without these, but they significantly improve usability.
**Delivers:** `sync/filter.rs` module, sync.toml filter config, --tag and --status CLI flags, `acrm sync` as pull-then-push, filter application to both directions
**Addresses:** Selective sync filtering, bidirectional sync command, config persistence
**Avoids:** Pushing all contacts regardless of source (only push opted-in contacts), auto-push without opt-in

### Phase Ordering Rationale

- **Phase 1 before Phase 2:** Push orchestration depends on having working vCard serialization, HTTP methods, and the vCard cache. Building and testing these in isolation reduces integration risk.
- **Phase 2 before Phase 3:** Filtering is meaningless without a working push. Conflict detection must be proven before adding convenience features.
- **vCard cache in Phase 1, not later:** The PITFALLS research is emphatic -- retrofitting the cache after push ships means contacts synced in the gap lose iCloud data (photos, TYPE params). This must be day-one infrastructure.
- **Auto-push deferred entirely:** The simpler `--push` flag approach (on edit/log commands) is recommended over a file watcher daemon. This avoids the notify dependency and daemon complexity. Can be added as a follow-up milestone.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 1:** vCard cache merge strategy needs validation -- how to replace specific VCardEntry instances in a parsed VCard without disturbing others. Recommend a targeted spike during planning.
- **Phase 2:** iCloud rate limiting behavior is undocumented. Initial push of 700+ contacts needs throttling, but exact limits are unknown. Recommend conservative 200ms delays between PUTs and exponential backoff on 503.

Phases with standard patterns (skip research-phase):
- **Phase 3:** Selective filtering is pure predicate logic with no external dependencies. Config parsing extends an existing hand-parser. Well-documented, established patterns.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Zero new deps. calcard source verified locally. reqwest capabilities confirmed by existing code. |
| Features | HIGH | RFC 6352 is stable standard. Feature set derived from protocol spec + established CardDAV client guides. |
| Architecture | HIGH | Extension of existing patterns. Component boundaries follow established codebase conventions. |
| Pitfalls | HIGH | Cross-referenced RFC 6352, sabre/dav guide, DAVx5 docs, Apple developer forums. Multiple sources confirm each pitfall. |

**Overall confidence:** HIGH

### Gaps to Address

- **vCard cache merge strategy:** The approach of merging CRM fields into cached originals needs implementation validation. calcard's public `entries` field makes this feasible but the exact merge logic (find-and-replace VCardEntry by property type) needs a spike during Phase 1 planning.
- **iCloud rate limits:** Exact thresholds are undocumented. The 200ms delay + exponential backoff strategy is a best guess based on community reports. Monitor during initial bulk push and adjust.
- **iCloud vCard normalization extent:** The degree of iCloud's post-PUT vCard rewriting is not fully documented. Some properties may be stripped or reformatted. Needs empirical testing during Phase 1.
- **Deletion detection edge cases:** Scanning for missing files with known source_ids works but has edge cases (renamed files, moved files). May need a lightweight sync log if this proves fragile during Phase 2.
- **Name splitting edge cases:** Multi-word last names, mononyms, CJK names in the `split_name()` helper need careful handling and test cases.

## Sources

### Primary (HIGH confidence)
- [RFC 6352 - CardDAV](https://www.rfc-editor.org/rfc/rfc6352) -- PUT/DELETE semantics, ETag requirements, If-Match behavior
- [sabre/dav: Building a CardDAV Client](https://sabre.io/dav/building-a-carddav-client/) -- Sync algorithm, vCard preservation, GET-after-PUT
- [CalConnect DevGuide: Building a CardDAV Client](https://devguide.calconnect.org/CardDAV/building-a-carddav-client/) -- Implementation patterns
- calcard 0.3.2 local source (`builder.rs`, `writer.rs`) -- VCard construction and serialization API
- Existing AgenticCRM codebase -- `sync/carddav.rs`, `sync/vcard_map.rs`, `sync/dedup.rs`, `commands/sync.rs`

### Secondary (MEDIUM confidence)
- [DAVx5 Technical Documentation](https://manual.davx5.com/technical_information.html) -- If-None-Match, CTag patterns
- [Google CardDAV API](https://developers.google.com/people/carddav) -- PUT/DELETE cross-reference
- [Apple Developer Forums](https://developer.apple.com/forums/thread/722170) -- iCloud rate limiting, vCard validation
- [vdirsyncer iCloud issue #1145](https://github.com/pimutils/vdirsyncer/issues/1145) -- iCloud CardDAV write behavior

### Tertiary (LOW confidence)
- [The Eclectic Light Company: iCloud Throttling](https://eclecticlight.co/2024/02/22/icloud-does-throttle-data-syncing-after-all/) -- iCloud throttling behavior (needs empirical validation)

---
*Research completed: 2026-03-07*
*Ready for roadmap: yes*
