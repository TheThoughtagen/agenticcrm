# Feature Research

**Domain:** MCP server, CLI bulk operations, LinkedIn CSV automation for a Rust CLI CRM
**Researched:** 2026-03-08
**Confidence:** HIGH (MCP spec is stable; bulk ops follow well-established Unix patterns; LinkedIn export is a documented native feature)

## Feature Landscape

### Table Stakes (Users Expect These)

#### MCP Server

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Tool registration for all existing commands | MCP is pointless if it only exposes a subset; agents need full CRUD | MEDIUM | Map each `acrm` subcommand (add, list, search, show, edit, log, due, delete, archive) to an MCP tool with typed input schemas |
| JSON-Schema typed tool inputs | MCP spec requires `inputSchema` for each tool; agents cannot discover parameters otherwise | LOW | Already have clap definitions; translate to JSON Schema |
| Tool result as structured content | MCP tools return `content` array with text/JSON; agents need parseable output | LOW | Existing `--format json` logic provides the serialization; wrap in MCP content blocks |
| stdio transport | Primary MCP transport; how Claude Desktop, VS Code, Cursor all connect | MEDIUM | Requires async (tokio) for JSON-RPC message loop over stdin/stdout |
| `initialize` / `initialized` handshake | MCP spec requires capability negotiation before any tool calls | LOW | Standard protocol handshake; SDK handles this |
| `tools/list` and `tools/call` handlers | Core MCP server lifecycle; without these, nothing works | MEDIUM | Route tool names to existing command logic |
| Error responses with proper MCP error codes | Agents need structured errors to retry or report failures | LOW | Map anyhow errors to MCP error content blocks |
| Server metadata (name, version, capabilities) | Agents display this; MCP spec requires it in `initialize` response | LOW | Static metadata from Cargo.toml |

#### Bulk Operations

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Query syntax for filtering contacts | Bulk ops without filtering = dangerous; must select targets precisely | MEDIUM | Field-based predicates: `status=dormant`, `tag=linkedin-import`, `company="Acme Corp"` |
| Bulk edit (set fields on matched contacts) | Primary use case: `acrm bulk 'status=dormant' --set status=archived` | MEDIUM | Iterate matches, apply edits, report count |
| Dry-run mode for bulk operations | Bulk changes are destructive; users must preview before committing | LOW | Existing `--dry-run` pattern from sync commands |
| Operation count summary | "Updated 47 contacts" -- users need confirmation of what happened | LOW | Counter in loop, formatted output |
| JSON output for bulk results | Pipe results to jq or other tools; agent consumption | LOW | Existing `--format json` system |
| Confirmation prompt for destructive bulk ops | Bulk delete/archive without confirmation is reckless | LOW | Existing `dialoguer` confirm pattern from delete command |

#### LinkedIn CSV Import (Rust-native)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `acrm import linkedin <file.csv>` command | Replace shell script with proper Rust command; consistent UX, validation, JSON output | MEDIUM | CSV parsing with csv crate; map to Contact struct; dedup by name |
| Dedup against existing contacts | Must not create duplicates on re-import | LOW | Match by name slug (existing pattern) or source_id |
| Import summary (added/skipped/updated) | Users need to know what happened | LOW | Counters |
| Change detection on re-import | Show what fields differ when contact already exists | MEDIUM | Compare parsed CSV fields against existing contact frontmatter |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| MCP resources for contact files | Expose contacts as `resource://contacts/{slug}` URIs; agents can read full markdown including interaction logs without a tool call | MEDIUM | MCP resources are application-controlled (host decides when to fetch); gives agents richer context than tool results |
| MCP prompts for CRM workflows | Pre-built prompt templates: "review overdue follow-ups", "prepare for meeting with {name}", "summarize relationship with {name}" | LOW | Prompt templates with variable substitution; high value for agent UX |
| Query syntax with operators | Beyond `key=value`: support `!=`, `>`, `<`, `~` (contains), date comparisons (`last_contacted < 2025-01-01`) | HIGH | Requires expression parser; significantly more useful but complex |
| JSON stdin pipe for bulk operations | `acrm search "acme" --format json \| acrm bulk --stdin --set tag+=priority` -- Unix composability | MEDIUM | Read ContactFile JSON from stdin; enables arbitrary pipeline composition |
| Bulk tag add/remove (+=, -=) | Tags are arrays; `--set tags=work` replaces all tags; need `--set tags+=work` to append | LOW | Parse `+=` / `-=` operators in set syntax |
| HTTP/SSE transport for MCP | Remote agent connections; multiple simultaneous clients | HIGH | Requires axum + SSE streaming; official rust-mcp-sdk supports this via HyperServer but adds significant dependency weight |
| LinkedIn automation (Playwright CSV trigger) | Auto-trigger LinkedIn's native "Download your data" flow; no scraping, just clicking the export button | HIGH | Requires Playwright (Node.js subprocess or headless-chrome crate); fragile to LinkedIn UI changes; experimental by nature |
| Smart re-import with merge | On re-import, detect field changes and offer per-field merge: "Company changed from X to Y, update?" | HIGH | Requires field-level diff and interactive prompts; valuable for periodic LinkedIn re-exports |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| LinkedIn profile scraping via Playwright | "Get more data than CSV provides" | TOS violation; fragile DOM selectors; account ban risk; PROJECT.md explicitly lists this as out of scope | Use native CSV export only; enrich manually or via API services later |
| MCP sampling (agent-initiated LLM calls) | "Let the server call the LLM for enrichment" | Adds LLM dependency to a local-first tool; unpredictable costs; privacy concerns with contact data | Keep MCP server read/write only; let the client-side agent do reasoning |
| Full-text search via MCP | "Search interaction logs, not just frontmatter" | Requires indexing at scale; current grep-through-files works fine for personal CRM (<10K contacts) | Expose search tool that searches frontmatter fields; agents can read full files via resources for deep context |
| Bulk operations with regex field matching | "Match contacts where name matches /^J.*son$/" | Regex in CLI args is error-prone; escaping nightmares; most users want simple equality/contains | Support `~` (contains) operator; if regex is truly needed, pipe through `acrm list --format json \| jq` |
| WebSocket MCP transport | "Real-time bidirectional communication" | Over-engineered for a personal CRM; no concurrent multi-agent scenario | stdio for local agents; HTTP/SSE if remote is needed |
| Persistent MCP server daemon | "Keep server running for instant connections" | Daemon management complexity; process lifecycle; crash recovery | Start MCP server on-demand via stdio (standard pattern); HTTP mode can use systemd/launchd if needed |
| LinkedIn OAuth API integration | "Use the official API instead of CSV" | LinkedIn API requires approved app; restrictive rate limits; only available to partners; personal use not supported | CSV export is the only reliable personal-use path |

## Feature Dependencies

```
[Existing CLI commands] ──required-by──> [MCP tool handlers]
[Existing --format json] ──required-by──> [MCP tool result formatting]

[stdio transport + JSON-RPC loop]
    └──required-by──> [MCP server launch command: acrm mcp]
                           └──enhances──> [HTTP/SSE transport (optional)]

[MCP tool registration]
    └──required-by──> [MCP resources (contacts as URIs)]
    └──required-by──> [MCP prompts (workflow templates)]

[Query syntax parser]
    └──required-by──> [Bulk edit: acrm bulk 'query' --set ...]
    └──required-by──> [Bulk delete/archive]
    └──required-by──> [Bulk tag operations]
    └──enhances──> [MCP search tool (richer filtering)]

[Bulk edit core]
    └──enhances──> [JSON stdin pipe (acrm bulk --stdin)]

[CSV parser (Rust-native)]
    └──required-by──> [acrm import linkedin <file>]
    └──required-by──> [Change detection on re-import]
                           └──enhances──> [Smart merge on re-import]

[LinkedIn CSV import] ──independent-of──> [MCP server]
[LinkedIn CSV import] ──independent-of──> [Bulk operations]

[Playwright automation]
    └──required-by──> [acrm linkedin export (experimental)]
    └──independent-of──> [CSV import (different phase of workflow)]
```

### Dependency Notes

- **MCP tools require existing CLI commands:** Each MCP tool delegates to the same logic as `acrm add`, `acrm search`, etc. The MCP server is a new interface to existing functionality, not new business logic.
- **Query syntax is shared:** Both bulk CLI operations and MCP search benefit from the same query parser. Build it once, use it in both contexts.
- **LinkedIn automation is fully independent:** The Playwright-based CSV export and the Rust-native CSV import are separate concerns. Import can ship without automation; automation is an optional convenience layer.
- **HTTP/SSE transport enhances but doesn't block stdio:** Ship stdio first (covers Claude Desktop, VS Code, Cursor). Add HTTP/SSE later if remote agent access is needed.

## MVP Definition

### Phase 1: MCP Server (stdio)

- [x] `acrm mcp` command that starts stdio JSON-RPC server
- [x] Tool: `search_contacts` -- query by name, company, tag, status
- [x] Tool: `show_contact` -- full contact detail by name
- [x] Tool: `add_contact` -- create new contact
- [x] Tool: `edit_contact` -- update fields on existing contact
- [x] Tool: `log_interaction` -- log interaction with a contact
- [x] Tool: `list_due_followups` -- show overdue/upcoming follow-ups
- [x] Tool: `list_contacts` -- list all contacts with optional tag filter
- [x] Tool: `delete_contact` -- delete a contact
- [x] Tool: `archive_contact` -- archive a contact
- [x] Resource: `contacts://{slug}` -- read full contact markdown
- [x] Prompt: `review_followups` -- guide agent through follow-up review

### Phase 2: Bulk Operations

- [x] Query syntax parser: `field=value`, `field!=value`, `field~value` (contains)
- [x] `acrm bulk 'query' --set field=value` -- bulk edit
- [x] `acrm bulk 'query' --delete` -- bulk delete with confirmation
- [x] `acrm bulk 'query' --archive` -- bulk archive
- [x] `--dry-run` on all bulk operations
- [x] `--yes` to skip confirmation
- [x] `--format json` for bulk results
- [x] Tag append/remove: `--set tags+=value`, `--set tags-=value`

### Phase 3: LinkedIn Import + Automation

- [x] `acrm import linkedin <file.csv>` -- Rust-native CSV import replacing shell script
- [x] Dedup against existing contacts by name slug
- [x] Change detection on re-import (show diffs)
- [x] Import summary (added/skipped/updated counts)
- [x] `acrm linkedin export` -- Playwright-based CSV export trigger (experimental)

### Add After Validation (v1.x)

- [ ] HTTP/SSE MCP transport -- trigger: when remote agent access is needed
- [ ] Advanced query operators (`>`, `<`, date comparisons) -- trigger: when users hit limits of simple equality/contains
- [ ] JSON stdin pipe for bulk ops (`--stdin`) -- trigger: when pipeline composition demand emerges
- [ ] MCP resource subscriptions (notify agent when contacts change) -- trigger: MCP spec stabilizes subscriptions
- [ ] Smart merge on LinkedIn re-import (interactive field-level diff) -- trigger: users doing periodic re-exports

### Future Consideration (v2+)

- [ ] CalDAV integration for follow-up reminders as calendar events -- defer: complex OAuth, tangential to CRM core
- [ ] MCP sampling for AI-powered contact enrichment -- defer: privacy concerns, LLM dependency
- [ ] Multi-source import framework (Google Contacts CSV, Outlook CSV) -- defer: each format is different; build on demand

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| MCP stdio server with full tool set | HIGH | MEDIUM | P1 |
| MCP contact resources | MEDIUM | LOW | P1 |
| MCP workflow prompts | MEDIUM | LOW | P1 |
| Query syntax parser | HIGH | MEDIUM | P1 |
| Bulk edit with query | HIGH | MEDIUM | P1 |
| Bulk delete/archive | MEDIUM | LOW | P1 |
| Bulk tag add/remove | MEDIUM | LOW | P1 |
| Rust-native LinkedIn CSV import | HIGH | MEDIUM | P1 |
| Import dedup and change detection | MEDIUM | MEDIUM | P1 |
| Dry-run for bulk ops | HIGH | LOW | P1 |
| HTTP/SSE MCP transport | LOW | HIGH | P2 |
| JSON stdin pipe for bulk | MEDIUM | MEDIUM | P2 |
| Advanced query operators (dates, comparisons) | MEDIUM | HIGH | P2 |
| LinkedIn Playwright automation | LOW | HIGH | P3 |
| Smart merge on re-import | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for v1.2 launch
- P2: Should have, add when demand emerges
- P3: Nice to have, experimental / future consideration

## Competitor Feature Analysis

| Feature | HubSpot MCP Server | mcp-crm (community) | Our Approach |
|---------|-------------------|---------------------|--------------|
| Contact CRUD via MCP | Read-only (launched June 2025) | Full CRUD with SQLite backend | Full CRUD via tools; data stays in markdown files |
| Search | Natural language via ChatGPT deep research | SQLite queries | Field-based query syntax; agent does NL interpretation |
| Resources | Not exposed | Not exposed | Contacts as URI-addressable resources; full markdown including interaction logs |
| Prompts | None | None | Workflow templates (follow-up review, meeting prep) -- differentiator |
| Bulk operations | API-based batch ops | Not available | CLI query syntax + bulk set/delete/archive |
| Data portability | Vendor-locked SaaS | SQLite file | Markdown files in git -- maximum portability |
| Transport | HTTP API | stdio | stdio (launch); HTTP/SSE (future) |
| LinkedIn integration | Native via API partnership | None | CSV import (native export); Playwright automation (experimental) |

## Sources

- [MCP Specification - Architecture Overview](https://modelcontextprotocol.io/docs/learn/architecture) -- Protocol primitives: tools, resources, prompts (HIGH confidence)
- [MCP Features Guide - WorkOS](https://workos.com/blog/mcp-features-guide) -- Tools vs Resources vs Prompts distinction (HIGH confidence)
- [MCP Server Design Best Practices - Philipp Schmid](https://www.philschmid.de/mcp-best-practices) -- Tool naming, scope, deterministic behavior (HIGH confidence)
- [MCP Best Practices - Workato](https://docs.workato.com/mcp/mcp-server-design.html) -- Server scoping and tool design patterns (MEDIUM confidence)
- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk) -- v0.16.0; stdio + HTTP/SSE transport support (HIGH confidence)
- [rust-mcp-sdk on crates.io](https://crates.io/crates/rust-mcp-sdk) -- Alternative SDK with HyperServer for HTTP/SSE (MEDIUM confidence)
- [Command Line Interface Guidelines](https://clig.dev/) -- Unix philosophy for CLI composability (HIGH confidence)
- [LinkedIn Export Connections Help](https://www.linkedin.com/help/linkedin/answer/a566336/export-connections-from-linkedin) -- Official CSV export fields and limitations (HIGH confidence)
- [LinkedIn Data Download Help](https://www.linkedin.com/help/linkedin/answer/a1339364/downloading-your-account-data) -- Full data archive vs quick connections export (HIGH confidence)
- [CRM MCP Servers Overview - Merge.dev](https://www.merge.dev/blog/crm-mcp-server) -- Landscape of CRM MCP implementations (MEDIUM confidence)
- [HubSpot MCP Server](https://developers.hubspot.com/mcp) -- First major CRM with production MCP; read-only (MEDIUM confidence)
- [Learn by Building - CRM MCP Server](https://learnbybuilding.ai/tutorial/creating-a-mcp-server-to-run-a-crm/) -- Community tutorial for CRM MCP patterns (LOW confidence)
- Existing codebase: `src/main.rs`, `src/store.rs`, `scripts/import-linkedin.sh` -- Direct reading (HIGH confidence)
- PROJECT.md constraints and existing architecture (HIGH confidence)

---
*Feature research for: MCP server, bulk operations, LinkedIn automation (v1.2 milestone)*
*Researched: 2026-03-08*
