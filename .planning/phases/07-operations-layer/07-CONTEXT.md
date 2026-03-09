# Phase 7: Operations Layer - Context

**Gathered:** 2026-03-09
**Status:** Ready for planning

<domain>
## Phase Boundary

Extract all CRM business logic from CLI command handlers into a shared `ops` module. Every CLI command (add, list, search, show, edit, log, due, delete, archive) and sync operations (pull, push, bidirectional) delegate to ops functions. CLI handlers become thin wrappers: parse args, call ops, format output. No user-facing behavior changes.

</domain>

<decisions>
## Implementation Decisions

### Ops API Surface
- Each ops function returns a typed result struct (e.g., `AddResult { path, contact }`, `SearchResult { matches }`, `SyncPullResult { created, updated, skipped }`)
- Callers (CLI, future MCP, future bulk) are responsible for formatting output from these structs
- CRM root path passed as argument to all ops functions — ops never calls `find_crm_root()` itself
- Contact lookup (name → ContactFile) handled inside ops — callers pass a name string, ops does fuzzy matching internally
- Ops module uses its own error enum (`OpsError::NotFound`, `OpsError::AmbiguousMatch`, `OpsError::ValidationFailed`, etc.) — not anyhow. MCP can map these to proper error codes, CLI can show user-friendly messages

### Sync Inclusion
- Sync operations (pull, push, bidirectional) extracted into ops alongside CRUD commands
- Sync setup (interactive credential prompting) stays CLI-only — not useful for MCP
- Credentials provided by caller: `ops::sync_pull(root, credentials, filter, opts)` — ops never loads from keyring
- Sync filter construction (merging config + CLI flags) stays in callers — ops receives a `SyncFilter`, doesn't know about config files

### Tech Debt Cleanup
- Fix `update_existing_contact` bypassing `store::serialize_contact_file` — route all writes through same path during extraction
- Remove unused `SyncConfig` struct (dead_code warning)
- Fix TUI dead_code warning
- Target: zero compiler warnings after Phase 7

### TUI Integration
- TUI calls ops functions for all operations: listing contacts, logging interactions, any future mutations
- TUI uses `ops::list()` for contact loading on startup and reload
- TUI's in-memory search filtering stays as-is (real-time keystroke filtering on loaded data — not a business logic concern, performance-critical)

### Claude's Discretion
- Module structure (single `ops.rs` vs `ops/` directory with submodules)
- Exact OpsError variant names and structure
- Internal refactoring approach (extract-and-test vs move-and-verify)
- How to handle the delete confirmation prompt (likely: ops returns what would be deleted, CLI prompts, then calls ops::confirm_delete)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Key principle: ops functions should be pure enough that MCP handlers (Phase 9) and bulk operations (Phase 8) can call them without any CLI coupling.

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `store.rs`: `parse_contact_file`, `serialize_contact_file`, `load_all_contacts`, `write_contact`, `find_single_contact`, `find_crm_root` — core I/O layer that ops will build on
- `frontmatter.rs`: `parse_raw_frontmatter`, `update_field`, `update_array_field` — field-level editing ops will use
- `validation.rs`: `validate_contact` — ops should validate before writing
- `sync/` module: `carddav.rs`, `push.rs`, `vcard_write.rs`, `vcard_map.rs`, `filter.rs` — sync infrastructure ops will orchestrate

### Established Patterns
- Commands in `src/commands/*.rs` each have a `pub fn run(...)` that takes an `OutputFormat` reference — this pattern will be preserved as thin wrappers
- `find_single_contact` does fuzzy matching and returns error on zero/ambiguous matches — this logic moves into ops
- Raw frontmatter preservation pattern (YAML comments survive editing) — must be maintained through ops layer

### Integration Points
- `main.rs` match arms: each command dispatches to `commands::*::run()` — after refactor, these call the same functions but handlers internally call ops
- TUI `app.rs` `update()` method: handles `Message::ConfirmLog` with inline interaction logging — will call ops::log_interaction
- TUI `App::new()`: loads contacts via `store::load_all_contacts` — will call ops::list

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 07-operations-layer*
*Context gathered: 2026-03-09*
