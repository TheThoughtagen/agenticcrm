# Domain Pitfalls

**Domain:** Personal CRM with CardDAV sync, ratatui TUI, and MCP server integration
**Researched:** 2026-03-05
**Overall confidence:** MEDIUM (no web sources available; based on training data for RFC 6352, ratatui patterns, and MCP spec knowledge up to May 2025)

---

## Critical Pitfalls

Mistakes that cause rewrites, data loss, or architectural dead ends.

### Pitfall 1: Lossy Markdown-to-vCard Round-Tripping

**What goes wrong:** The CRM stores contacts as Markdown+YAML with rich freeform fields (interaction logs, family notes, interests). vCard (RFC 6352 / RFC 6350) has a fixed property set. When syncing to CardDAV and back, CRM-specific fields (tags, follow_up_cadence, interaction log, relationship type, how_we_met) have nowhere to live in vCard. Developers either lose this data silently, or try to stuff it into vCard X-properties and discover that iCloud strips or ignores unknown X-properties on write-back.

**Why it happens:** Developers assume they can treat the sync as a simple serialization problem. In reality, the CRM data model is a superset of vCard, and the CardDAV server (especially iCloud) is opinionated about what it preserves.

**Consequences:** Data loss on sync cycles. Users lose interaction history or CRM metadata after a sync round-trip. Or the sync layer becomes impossibly complex trying to preserve everything.

**Prevention:**
- Design the sync as a **partial field mapping**, not a full serialization. Only sync the vCard-native fields: name, email, phone, address, birthday, company/role, social URLs, notes.
- Store a `carddav_etag` and `carddav_href` in the YAML frontmatter for sync tracking, but never expect the CardDAV side to store CRM-specific metadata.
- The interaction log and CRM fields (status, priority, follow_up_cadence, tags) are CRM-only. Document this boundary explicitly.
- Use a sync metadata file (e.g., `.sync/carddav-state.json`) to track the mapping between local contact IDs and remote CardDAV hrefs/etags.

**Detection:** If your sync design includes "store tags in vCard NOTE field" or "use X-ACRM-STATUS vCard extension" -- you are heading toward this pitfall.

**Phase:** CardDAV sync phase. Must be addressed in the initial sync architecture design, not bolted on later.

---

### Pitfall 2: CardDAV ETag/Conflict Mishandling Causes Data Overwrites

**What goes wrong:** CardDAV uses ETags for optimistic concurrency. A PUT request must include `If-Match: <etag>` to update a contact. If the ETag has changed (someone edited the contact on their phone), the server returns 412 Precondition Failed. Developers either ignore ETags (causing silent overwrites), or fail to handle 412 properly (causing sync failures that require manual intervention).

**Why it happens:** The project spec says "CRM wins on conflicts." Developers interpret this as "just PUT unconditionally." But CardDAV servers reject unconditional PUTs on existing resources (or worse, some servers silently merge). Apple's iCloud CardDAV is particularly strict about ETags.

**Consequences:** Contact data on the phone gets silently overwritten without the user realizing what was lost. Or sync gets stuck on 412 errors with no recovery path.

**Prevention:**
- "CRM wins" must be implemented as: (1) GET the current server version, (2) merge/compare, (3) PUT with the fresh ETag. If 412, re-fetch and retry.
- Never PUT without `If-Match` unless creating a new contact (use `If-None-Match: *` for creates).
- Store ETags locally per contact in the sync state file. Refresh ETags on every sync cycle via `PROPFIND` or individual GETs.
- Implement a simple retry loop (max 3 attempts) for ETag conflicts.

**Detection:** If your sync code does `PUT` without checking the response status or without an `If-Match` header, this pitfall is active.

**Phase:** CardDAV sync phase. Must be in the core sync loop implementation.

---

### Pitfall 3: iCloud CardDAV Authentication Is Not Standard Basic Auth

**What goes wrong:** Developers try to authenticate to iCloud CardDAV using Apple ID email + password. This fails because iCloud requires an **app-specific password** (generated at appleid.apple.com) and the CardDAV endpoint URL is not obvious -- it requires a discovery flow (well-known URL -> principal URL -> addressbook-home-set -> actual collection URL).

**Why it happens:** Most CardDAV tutorials show simple Basic Auth against a known URL. iCloud adds multiple layers: app-specific passwords, a DNS SRV/well-known discovery chain, and URL indirection.

**Consequences:** Authentication fails immediately. Developers spend days debugging auth when the real issue is using the wrong password type or wrong endpoint URL.

**Prevention:**
- Document clearly: users must generate an app-specific password at https://appleid.apple.com/account/manage (under "Sign-In and Security" > "App-Specific Passwords").
- Implement the full CardDAV discovery chain: `GET /.well-known/carddav` on `contacts.icloud.com` -> follow redirect -> `PROPFIND` for `current-user-principal` -> `PROPFIND` for `addressbook-home-set` -> `PROPFIND` to list collections.
- Store the discovered collection URL in config so discovery only runs once (but re-discover on 401/404).
- The iCloud CardDAV host is `contacts.icloud.com` with port 443 and TLS required.

**Detection:** If your config asks for "CardDAV URL" as a single field without a discovery mechanism, you will hit this.

**Phase:** CardDAV sync phase. Must be the very first thing implemented and tested -- before any CRUD operations.

---

### Pitfall 4: Ratatui Blocking I/O Freezes the TUI

**What goes wrong:** The TUI calls `load_all_contacts()` (which does synchronous filesystem I/O via `walkdir` + `read_to_string`) on the main thread. With hundreds of contacts, the TUI freezes for visible periods. With CardDAV sync running, it freezes for seconds.

**Why it happens:** The existing `store.rs` is entirely synchronous. Developers wire it directly into the ratatui render loop. Ratatui itself is immediate-mode and single-threaded -- it redraws every frame. Any blocking call in the event loop blocks the entire UI.

**Consequences:** The TUI feels broken. Keypresses are lost during I/O. Users think the app crashed.

**Prevention:**
- Use a **background thread** (or tokio task if you adopt async) for all I/O operations. The TUI event loop should only read from an in-memory state and send commands to a background worker via channels (`std::sync::mpsc` or `crossbeam`).
- Architecture pattern: `App` struct holds display state. Background thread loads data and sends `AppEvent::ContactsLoaded(Vec<ContactFile>)` over a channel. The event loop polls both terminal events AND the channel.
- Never call `std::fs::read_to_string` or any network I/O from within the `terminal.draw(|f| { ... })` closure or the main event loop.
- Show a loading indicator for any operation that might take >100ms.

**Detection:** If `load_all_contacts()` appears anywhere in the same function as `terminal.draw()`, this pitfall is active.

**Phase:** TUI phase. Must be the foundational architecture decision before building any TUI features.

---

### Pitfall 5: MCP Server stdio Transport Conflicts with TUI Terminal Control

**What goes wrong:** MCP servers typically use stdio (stdin/stdout) as their transport -- the AI client sends JSON-RPC over stdin, the server responds on stdout. But the ratatui TUI also takes control of the terminal (raw mode, alternate screen). These two uses of the terminal are fundamentally incompatible in the same process.

**Why it happens:** Developers try to build a single binary that is both TUI and MCP server simultaneously, or try to add MCP to the existing CLI binary without considering that MCP stdio transport hijacks stdin/stdout.

**Consequences:** The MCP server and TUI cannot run simultaneously in the same process. Attempting it corrupts terminal state, mangles JSON-RPC messages, or causes the TUI to display garbage.

**Prevention:**
- The MCP server MUST be a separate execution mode, not a concurrent feature of the TUI. Use a subcommand: `acrm serve-mcp` that runs in stdio mode (no TUI, no colored output, pure JSON-RPC on stdin/stdout).
- Alternatively, use an HTTP/SSE transport for MCP instead of stdio, which avoids the terminal conflict entirely. But stdio is the most widely supported MCP transport for local tools.
- The binary can share all the business logic (store.rs, models, etc.) but the entrypoints must be mutually exclusive: `acrm tui` starts the TUI, `acrm serve-mcp` starts the MCP server.
- Ensure `acrm serve-mcp` suppresses ALL non-JSON output (no `eprintln!`, no `colored` output, no progress indicators).

**Detection:** If your architecture diagram shows MCP and TUI as concurrent features of a single process, or if MCP uses stdout while the TUI is active, this pitfall is active.

**Phase:** MCP phase AND TUI phase. The binary structure must accommodate both from the start. Design the subcommand split early.

---

## Moderate Pitfalls

### Pitfall 6: vCard UID Generation Causes Duplicate Contacts

**What goes wrong:** When creating a new contact on the CardDAV server, the vCard must have a UID property. If you use the CRM's `id` field (a UUID) as the vCard UID, and later the same person is added independently on the phone, you get two vCards for the same person with different UIDs. The sync has no way to detect they are the same contact.

**Prevention:**
- During initial sync, match existing CardDAV contacts to CRM contacts by **name + email** (fuzzy matching), not just UID.
- Once matched, store the CardDAV UID in the CRM frontmatter (`carddav_uid` field) and the CRM UUID in the vCard as an `X-ACRM-ID` property (though iCloud may strip this).
- For ongoing sync, always use the stored UID mapping. Only fall back to fuzzy matching for initial bootstrap.
- Build a manual "link/unlink" command (`acrm sync link <contact> <carddav-uid>`) for cases where automatic matching fails.

**Detection:** If your sync creates a new vCard for every CRM contact on first run without checking for existing matches, duplicates will proliferate.

**Phase:** CardDAV sync phase. Must be addressed in the initial sync/bootstrap logic.

---

### Pitfall 7: Ratatui State Management Becomes Spaghetti Without a State Machine

**What goes wrong:** The TUI starts simple (a contact list), but quickly adds modals, search, detail views, editing, confirmation dialogs. Without a structured state machine, the event handler becomes a nest of `if current_view == X && modal_open && editing_field == Y` conditionals that is impossible to extend or debug.

**Prevention:**
- Use an explicit **state enum** from day one:
  ```rust
  enum AppView {
      Dashboard,
      ContactList { selected: usize, filter: Option<String> },
      ContactDetail { contact_id: String },
      Editing { contact_id: String, field: EditField },
      SyncStatus,
  }
  ```
- Each variant owns its view-specific state. The event handler pattern-matches on the current view.
- Use a **view stack** (Vec<AppView>) for navigation so "back" always works. Push on navigate, pop on back/escape.
- Keep the `App` struct flat: `current_view: AppView`, `contacts: Vec<ContactFile>`, `status_message: Option<String>`. Do not nest state deeply.

**Detection:** If your event handler has more than 3 levels of `if/else` nesting or checks multiple booleans to determine the current UI state, refactor to an enum.

**Phase:** TUI phase. Must be the first architectural decision before building views.

---

### Pitfall 8: MCP Tool Schema Mismatch Causes Silent AI Agent Failures

**What goes wrong:** MCP tools expose JSON schemas for their inputs. If the schema does not precisely match what the tool handler expects, AI agents send malformed requests that either error out or silently produce wrong results. Common issues: optional fields not marked as optional, enum values not matching the Rust enum variants, date format not specified.

**Prevention:**
- Define MCP tool schemas using `serde_json` generated from the actual Rust types. Do not hand-write JSON schemas separately -- they will drift.
- Include `description` fields on every property -- AI agents use these to understand what to pass.
- Use integration tests that send actual MCP requests (JSON-RPC over stdin/stdout) and verify responses. Test with malformed inputs to verify error messages are useful.
- Map the CRM's existing CLI commands directly to MCP tools: `search_contacts`, `show_contact`, `log_interaction`, `list_due`, `add_contact`. Keep the same parameter names.

**Detection:** If your MCP tool schemas are defined as string literals in the code rather than derived from types, drift is inevitable.

**Phase:** MCP phase. Schema design should happen alongside tool implementation, not after.

---

### Pitfall 9: CardDAV Sync Corrupts the Git History

**What goes wrong:** CardDAV sync modifies many contact files at once. If sync auto-commits after every file write (or worse, does not commit at all), the git history becomes either noisy (hundreds of "sync" commits) or dangerous (uncommitted changes that conflict with manual edits).

**Prevention:**
- Sync should be a single atomic operation that modifies all files, then creates ONE commit: `"sync: CardDAV sync at 2026-03-05T14:30:00"`.
- Run sync on a clean working tree only. Before sync, check `git status` -- if there are uncommitted changes, abort with a message telling the user to commit first.
- Store sync state (last sync timestamp, ETags, UID mappings) in `.sync/` directory, which should be gitignored. Only contact file changes go into git.
- Provide a `--dry-run` flag that shows what would change without writing.

**Detection:** If your sync writes files one-at-a-time and commits after each, or if sync state files are being committed to git, this pitfall is active.

**Phase:** CardDAV sync phase. Git integration strategy must be decided upfront.

---

### Pitfall 10: vCard Parsing Is Harder Than It Looks

**What goes wrong:** Developers assume vCard is a simple text format and try to parse it with string splitting. vCard 3.0 and 4.0 have subtle differences (iCloud uses vCard 3.0). Properties can span multiple lines (line folding at 75 chars), values can be escaped with backslashes, structured values use semicolons as delimiters, and character encoding varies. A hand-rolled parser will break on real-world vCards.

**Prevention:**
- Use an existing Rust vCard parsing crate. Candidates include `vcard` or `ical` (which handles vCard as well). Evaluate before the CardDAV phase begins.
- If no Rust crate is mature enough, consider shelling out to a Python tool or wrapping a C library. But this conflicts with the "no runtime dependencies" constraint, so verify Rust crate quality first.
- Test with real vCards exported from iCloud -- they contain surprising formatting choices (e.g., photo data as base64 inline, semicolons in names, multi-value fields).
- Handle gracefully: if a vCard property cannot be parsed, skip it with a warning rather than failing the entire sync.

**Detection:** If you find yourself writing `line.split(":")` to parse vCard properties, stop and use a library.

**Phase:** CardDAV sync phase. Crate evaluation is a prerequisite task before implementation begins.

---

## Minor Pitfalls

### Pitfall 11: Ratatui Scrolling and Viewport Bugs with Large Contact Lists

**What goes wrong:** With hundreds of contacts, the list widget needs proper scrolling. Developers implement scrolling manually and get off-by-one errors, fail to keep the selected item visible, or break scrolling when filters change the list length.

**Prevention:**
- Use ratatui's built-in `List` widget with `ListState`, which handles scrolling and selection natively. Do not implement manual offset tracking.
- When applying a filter, reset the selection index to 0 and reset the scroll offset. Forgetting this causes panics (index out of bounds) or invisible selection.
- Test with 0 contacts, 1 contact, and 500+ contacts.

**Phase:** TUI phase. Test edge cases during list view implementation.

---

### Pitfall 12: serde_yaml Field Ordering Is Not Preserved

**What goes wrong:** The project convention says "keep frontmatter fields in the order defined in the template." But `serde_yaml::to_string` serializes fields in struct definition order by default, and if fields are added for sync metadata, the frontmatter order changes, causing noisy git diffs.

**Prevention:**
- Keep the `Contact` struct field order matching the template order. When adding new fields (carddav_uid, carddav_etag), add them at the end in a clearly marked section.
- Consider using `serde_yaml`'s `#[serde(flatten)]` or a custom serializer if field ordering becomes problematic. But test first -- `serde_yaml` with `derive(Serialize)` preserves struct field order, which is usually sufficient.
- Any new sync metadata fields should be added to both the struct AND the template simultaneously.

**Phase:** CardDAV sync phase. Relevant when extending the Contact struct for sync metadata.

---

### Pitfall 13: MCP JSON Output Breaks Existing CLI Colored Output

**What goes wrong:** The existing CLI uses `colored` crate for terminal output. When adding `--json` output mode (needed for MCP and scripting), developers forget to disable colored output, and JSON responses contain ANSI escape codes that break JSON parsing.

**Prevention:**
- Check for `--json` flag (or `NO_COLOR` env var) at the top level and disable colored output globally.
- Better: structure commands to return a result type, then format at the output layer. Commands return `CommandResult`, the output layer either pretty-prints with color or serializes to JSON.
- The MCP server mode should always use the JSON path, never the colored path.

**Detection:** If `colored::control::set_override(false)` is not called in JSON/MCP mode, ANSI codes will leak into output.

**Phase:** JSON output phase (precedes MCP). Must be solved before MCP implementation.

---

### Pitfall 14: CardDAV Rate Limiting and Timeout on Large Initial Syncs

**What goes wrong:** First sync with hundreds of contacts sends hundreds of GET/PUT requests to iCloud. iCloud rate-limits aggressively (exact limits undocumented but real). The sync hangs or fails partway through, leaving the sync state inconsistent.

**Prevention:**
- Use `REPORT` with `addressbook-multiget` (RFC 6352 Section 8.7) to fetch multiple vCards in a single request instead of individual GETs.
- Implement exponential backoff on 429/503 responses.
- Use `PROPFIND` with `getctag` (collection tag) to detect whether the collection has changed at all before doing a full sync. If ctag is unchanged, skip the sync entirely.
- Make sync resumable: track which contacts have been synced and continue from where it left off.

**Phase:** CardDAV sync phase. Critical for initial bootstrap sync with existing iCloud contacts.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| CardDAV sync design | Lossy round-tripping (Pitfall 1) | Define the field mapping boundary first. Document which fields sync and which do not. |
| CardDAV auth | iCloud auth complexity (Pitfall 3) | Implement discovery chain first. Test with real iCloud account before building CRUD. |
| CardDAV initial sync | Duplicate contacts (Pitfall 6), Rate limiting (Pitfall 14) | Build fuzzy matching for bootstrap. Use multiget requests. |
| CardDAV conflict handling | ETag mishandling (Pitfall 2) | Always use If-Match. Implement retry on 412. |
| CardDAV vCard parsing | Format complexity (Pitfall 10) | Evaluate Rust vCard crates before starting. Test with real iCloud exports. |
| CardDAV git integration | History corruption (Pitfall 9) | Single commit per sync. Clean working tree check. Gitignore sync state. |
| TUI architecture | Blocking I/O (Pitfall 4) | Background thread + channel pattern from day one. |
| TUI state management | Spaghetti state (Pitfall 7) | Enum-based view state with view stack. |
| TUI + MCP coexistence | stdio conflict (Pitfall 5) | Separate subcommands. Plan binary structure early. |
| MCP tool design | Schema drift (Pitfall 8) | Derive schemas from Rust types. Integration tests. |
| JSON output mode | ANSI in JSON (Pitfall 13) | Output layer abstraction. Solve before MCP phase. |
| Contact struct changes | YAML field ordering (Pitfall 12) | Add new fields at end. Keep struct order matching template. |

## Sources

- RFC 6352 (CardDAV specification) -- training data, MEDIUM confidence
- RFC 6350 (vCard 4.0 specification) -- training data, MEDIUM confidence
- ratatui architecture patterns -- training data from ratatui docs and examples, MEDIUM confidence
- MCP specification (modelcontextprotocol.io) -- training data up to May 2025, MEDIUM confidence
- iCloud CardDAV behavior -- training data from community reports, LOW-MEDIUM confidence (Apple does not document rate limits or X-property handling officially)
- Existing codebase analysis -- HIGH confidence (directly inspected `store.rs`, `contact.rs`, `Cargo.toml`)

**Note:** Web search and web fetch were unavailable during this research. All findings are based on training data (cutoff May 2025). iCloud-specific behaviors (Pitfalls 3, 14) should be validated against current iCloud behavior during implementation, as Apple may have changed endpoints or policies.
