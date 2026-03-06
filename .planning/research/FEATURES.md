# Feature Landscape

**Domain:** Plain-text personal CRM with CLI/TUI, CardDAV sync, and MCP agent integration
**Researched:** 2026-03-05
**Overall confidence:** MEDIUM (based on training data, ecosystem knowledge, and codebase analysis; web search unavailable for verification)

## Table Stakes

Features users expect from a personal CRM tool. Missing any of these and users will look elsewhere or revert to a spreadsheet.

### Core Contact Management (Existing)

| Feature | Why Expected | Complexity | Status | Notes |
|---------|--------------|------------|--------|-------|
| Add/edit/delete contacts | Fundamental CRUD | Low | Partial (no edit/delete CLI) | Edit command is in PROJECT.md active list |
| Search contacts | Users need to find people fast | Low | Exists | Full-text across name, company, tags, notes |
| List with filters | Browse and narrow down | Low | Exists | Currently only tag filter; needs status, company, relationship filters |
| View contact details | See everything about a person | Low | Exists | `acrm show` works |
| Interaction logging | Track when you last talked | Low | Exists | `acrm log` appends to markdown body |
| Follow-up reminders | The core CRM value prop | Low | Exists | `acrm due` shows overdue contacts |
| Import contacts | Bootstrap from existing data | Med | Partial | LinkedIn CSV import via shell script only |

### CLI Completeness (Needed)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Edit contact fields from CLI | Cannot currently modify frontmatter via CLI | Med | Must parse YAML, update specific field, re-serialize without data loss |
| Delete/archive contacts | No way to remove contacts via CLI | Low | Archive (status change) preferred over delete (file removal) |
| JSON output mode (`--json`) | Agent-friendliness is a core promise | Med | Every command needs a `--format json` flag; output should be structured, stable |
| Bulk tag/untag operations | Managing hundreds of contacts needs batch ops | Med | `acrm tag add --filter "company:Acme" networking` pattern |
| Sort options for list | Users need different views | Low | Sort by name, last_contacted, next_follow_up, priority |
| Filter by multiple criteria | Tag-only filtering is too limited | Low | Combine status, relationship, priority, company filters |
| Contact merge/dedup | Imports create duplicates | High | Matching heuristics on name/email, field merge strategy needed |

### Data Integrity (Needed)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Schema validation on write | Prevent corrupt/invalid contact files | Med | Validate required fields, enum values, date formats before writing |
| Safe serialization round-trip | Editing must not lose unknown fields or reorder YAML | High | Current `serde_yaml::to_string` may drop comments and reorder; needs custom serializer or operate on raw YAML |
| Automatic next_follow_up calculation | Logging an interaction should compute next date from cadence | Low | Parse cadence string ("monthly" -> +30 days), update on `acrm log` |

## Differentiators

Features that set AgenticCRM apart from typical personal CRMs. Not expected, but deliver outsized value.

### CardDAV Sync (iCloud/Apple Contacts)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Pull contacts from iCloud | Bootstrap CRM from existing phone contacts | High | CardDAV GET/REPORT on Apple's CardDAV endpoint; vCard 3.0/4.0 parsing required |
| Push contacts to iCloud | Changes in CRM appear on phone | High | CardDAV PUT with ETag-based conflict detection; must generate valid vCard |
| Two-way sync with CRM-wins conflict resolution | Keep both sources updated | Very High | Requires sync state tracking (ETags, CTag, sync tokens), change detection, field-level merge |
| Sync status/metadata tracking | Know when contacts were last synced, detect drift | Med | Store sync metadata (ETag, last-synced timestamp, source_id) in frontmatter |
| Selective sync (filter which contacts sync) | Not every CRM contact should go to phone | Low | Tag-based or status-based filter for sync scope |

**CardDAV implementation details worth noting:**
- Apple's CardDAV server lives at `https://contacts.icloud.com` and requires app-specific passwords (2FA)
- The protocol uses WebDAV with PROPFIND/REPORT/PUT/DELETE methods
- vCard parsing/generation is the real work: vCard 3.0 (Apple default) has quirks around encoding, photo handling, custom properties
- Sync tokens (RFC 6578) enable efficient incremental sync instead of full re-download
- Field mapping between vCard properties and the CRM's YAML schema needs explicit definition (e.g., vCard `ORG` -> `company`, `TITLE` -> `role`, `TEL` -> `phone[]`)
- CRM has richer fields (tags, relationship, status, priority, follow_up_cadence) that have no vCard equivalent -- these are CRM-only and must survive round-trips

### TUI Dashboard (ratatui)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Contact browser with table view | Visual overview of all contacts | Med | Scrollable table with columns: name, company, status, last_contacted, next_follow_up |
| Contact detail pane | See full details without switching context | Med | Split-pane layout: list on left, detail on right |
| Keyboard-driven navigation | Fast workflow for power users | Low | vim-style keys (j/k/gg/G), tab between panes, / for search |
| Inline search/filter | Narrow list in real-time | Med | Fuzzy matching as you type, filter by tag/status with keybindings |
| Follow-up dashboard | At-a-glance view of who needs attention | Med | Overdue contacts highlighted, sorted by urgency, quick-log action |
| Quick interaction logging | Log from TUI without leaving context | Med | Modal dialog: select type, enter summary, auto-updates contact |
| Status bar with stats | Contextual information | Low | Total contacts, overdue count, active filters displayed |
| Color-coded priority/status | Visual hierarchy | Low | Red for overdue, yellow for due-today, green for active, dim for archived |

**TUI architecture notes:**
- ratatui uses an immediate-mode rendering model: rebuild the entire UI each frame
- Standard pattern: App struct holds state, `update()` handles events, `draw()` renders
- Should use crossterm backend (most portable, works on macOS/Linux/Windows)
- Event loop should be async or use a separate thread for input handling
- Consider tui-textarea for text input widgets and tui-input for single-line inputs

### MCP Tool Server

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Search contacts tool | AI agents can find people in your network | Med | `search_contacts(query, filters)` -> JSON results |
| Get contact details tool | Agent reads full contact info | Low | `get_contact(name_or_id)` -> full contact JSON |
| Log interaction tool | Agent records conversations/meetings | Low | `log_interaction(contact, type, summary, notes)` |
| List due follow-ups tool | Agent proactively reminds you | Low | `get_due_followups()` -> list of overdue contacts |
| Add contact tool | Agent creates contacts from context | Low | `add_contact(name, fields)` -> created contact |
| Update contact tool | Agent enriches contact data | Med | `update_contact(name_or_id, fields)` -> updated contact |
| Relationship graph query | Agent understands your network topology | High | `find_connections(between, through)` -> paths, shared tags |
| Bulk query with filters | Agent does complex lookups | Med | `query_contacts(filters: {tags, status, company, relationship})` |

**MCP implementation details:**
- MCP uses JSON-RPC 2.0 over stdio (primary) or SSE transport
- Server declares tools via `tools/list` method, each tool has a JSON Schema for its parameters
- The server binary should be separate from the CLI (`acrm-mcp` or `acrm mcp serve`)
- Tools should return structured JSON, not human-formatted text
- Resources (read-only data exposure) could expose the contact list and individual contacts as URIs
- Consider also exposing prompts for common CRM workflows (e.g., "draft follow-up email to X")
- The Rust `rmcp` crate or `mcp-server` crate can handle the protocol layer; alternatively, implement the thin JSON-RPC layer directly since the protocol is straightforward

### Plain-Text Power Features

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Git integration (auto-commit on changes) | Full history of every contact change for free | Low | Optional `--git-commit` flag or config option; run `git add + commit` after writes |
| Grep-friendly output | Unix philosophy; pipe to other tools | Low | `--format tsv` for shell pipeline integration |
| Export to vCard/CSV | Get data out for other tools | Med | Generate vCard 3.0 or CSV from contact files |
| Contact templates | Quickly add contacts with pre-filled fields for specific contexts | Low | Template inheritance: `acrm add --template conference-lead "Name"` |

## Anti-Features

Features to explicitly NOT build. These would undermine the core value proposition.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Web UI | Scope creep; violates CLI/TUI-first philosophy; would need a web framework, auth, hosting | Use TUI for visual interface, MCP for agent access |
| Cloud hosting / multi-user | Personal CRM by design; cloud adds complexity, cost, privacy concerns | Stay local-first; git remote handles backup/sync between devices |
| Built-in email client | Massive scope; many better email tools exist | Log interactions manually or via agent; link to email threads |
| AI-powered contact enrichment | Privacy concern; requires API keys and external services; data quality is unreliable | Let the user or their AI agent enrich contacts deliberately via MCP tools |
| Calendar integration | Complex (CalDAV/OAuth), tangential to CRM core | Log meetings as interactions; let agents handle calendar via separate tools |
| Social media auto-scraping | Privacy/TOS violations; brittle scrapers; maintenance burden | Support manual LinkedIn CSV import; let users paste info |
| Mobile app | Massive scope; terminal apps work via SSH/Termux | Access via terminal on mobile, or via AI agent integration |
| Notification system | A CRM should not be a notification source; that is the agent's job | `acrm due` is pull-based; agents poll via MCP and notify through their own channels |
| Contact scoring / lead scoring | Enterprise CRM feature; over-engineers a personal tool | Use priority (high/med/low) and status (active/dormant/lost-touch/archived) |
| Full-text indexing / search engine | Over-engineering; grep on flat files is fast enough for personal scale (<10K contacts) | Linear scan with in-memory filtering; optimize only if profiling shows need |

## Feature Dependencies

```
Schema validation ─> Edit command (validate before writing)
Edit command ─> TUI inline editing (TUI calls edit logic)
JSON output mode ─> MCP server (MCP tools reuse JSON serialization)
JSON output mode ─> TUI (TUI uses same data layer)

CardDAV vCard parsing ─> CardDAV pull (need to parse vCards)
CardDAV vCard generation ─> CardDAV push (need to generate vCards)
CardDAV pull ─> Two-way sync (pull is prerequisite)
CardDAV push ─> Two-way sync (push is prerequisite)
Sync metadata in frontmatter ─> Two-way sync (need ETags/timestamps)

Contact CRUD completeness ─> MCP server (all tools need working CRUD)
Contact CRUD completeness ─> TUI (TUI needs full read/write access)

Safe YAML round-trip ─> Edit command (editing must not corrupt files)
Safe YAML round-trip ─> CardDAV sync (sync writes must preserve CRM-only fields)
```

## MVP Recommendation

For the next milestone (CardDAV + TUI + MCP), prioritize in this order:

### Phase 1: CLI Foundation (prerequisite for everything)

1. **JSON output mode** -- Every downstream feature (MCP, TUI, scripting) needs structured output. Add `--format json` to all commands. This is the single highest-leverage feature.
2. **Edit command** -- Cannot build TUI editing or MCP update tools without programmatic field editing.
3. **Safe YAML round-trip** -- Must solve before any feature that writes contact files (edit, sync, MCP updates). Consider using `yaml-rust2` for AST-preserving edits instead of serialize/deserialize.
4. **Schema validation** -- Catch bad data before it hits disk, especially important when sync or agents write contacts.

### Phase 2: MCP Server

5. **MCP server with read-only tools** -- search, get, list, due. Low risk since it only reads data. Immediately useful with AI agents.
6. **MCP write tools** -- add, update, log. Higher risk but completes the integration.

### Phase 3: CardDAV Sync

7. **vCard parsing/generation** -- The hard technical work; can be developed and tested independently.
8. **CardDAV pull (one-way import)** -- Get contacts from iCloud. Lower risk than two-way sync. Immediately useful.
9. **Two-way sync with conflict resolution** -- The full feature. Needs sync state tracking, CRM-wins merge logic.

### Phase 4: TUI

10. **TUI contact browser** -- Read-only table view with search/filter. Uses same data layer as CLI.
11. **TUI detail pane and interaction logging** -- Full interactive experience.

**Defer:**
- Relationship graph queries: interesting but not core; add after MCP basics work
- Bulk operations: useful but can be done with shell scripting on flat files for now
- Contact merge/dedup: complex heuristics, defer until import volume justifies it
- Export to vCard/CSV: nice-to-have, not blocking anything

## Sources

- Codebase analysis: `/Users/pmannion/repos/agenticcrm/src/` (direct reading of all Rust source files)
- Project requirements: `/Users/pmannion/repos/agenticcrm/.planning/PROJECT.md`
- Contact schema: `/Users/pmannion/repos/agenticcrm/.schemas/contact.yaml`
- CardDAV protocol: RFC 6352 (CardDAV), RFC 6578 (WebDAV Sync), RFC 6350 (vCard 4.0) -- from training data, MEDIUM confidence
- MCP protocol: Model Context Protocol specification (JSON-RPC 2.0 over stdio) -- from training data, MEDIUM confidence
- ratatui patterns: immediate-mode rendering, crossterm backend -- from training data, HIGH confidence (well-established library)

**Confidence notes:**
- Web search was unavailable during this research session. Feature landscape is based on domain expertise and codebase analysis rather than competitive product analysis.
- CardDAV details are from RFC knowledge in training data; Apple-specific endpoint behavior should be verified against current iCloud documentation.
- MCP protocol details reflect the spec as of early 2025; verify current tool/resource schema format against latest spec before implementation.
