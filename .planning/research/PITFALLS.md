# Pitfalls Research

**Domain:** Adding MCP server, bulk operations, and LinkedIn automation to a Rust CLI CRM with flat file storage
**Researched:** 2026-03-08
**Confidence:** HIGH (core async/sync and concurrency pitfalls), MEDIUM (LinkedIn automation specifics)

## Critical Pitfalls

### Pitfall 1: reqwest::blocking Panics Inside Tokio Async Runtime

**What goes wrong:**
The existing codebase uses `reqwest::blocking::Client` for CardDAV sync (`src/sync/carddav.rs`). The MCP server (rmcp crate) requires tokio async runtime. If MCP tool handlers call existing sync code that uses `reqwest::blocking`, tokio panics with "Cannot drop a runtime in a context where blocking is not allowed." This is a hard crash, not a subtle bug.

**Why it happens:**
`reqwest::blocking` internally creates its own tokio runtime via `block_on()`. Calling `block_on()` inside an existing tokio runtime is forbidden -- it is a nested runtime panic. The existing `CardDavClient` uses `reqwest::blocking::Client` throughout, and every store operation uses synchronous `std::fs` calls. When the MCP server invokes these from an async handler, it hits the nested-runtime panic.

**How to avoid:**
Use `tokio::task::spawn_blocking()` to wrap all existing synchronous code calls within MCP tool handlers. This offloads blocking work to a dedicated thread pool while requiring zero changes to the existing CLI code. The MCP server becomes a thin async shell around the existing sync core. Do NOT attempt to convert the entire codebase to async -- the CLI and TUI work fine synchronously and the project is 4,700+ LOC. A full async migration is a rewrite, not a feature addition.

**Warning signs:**
- Any `reqwest::blocking` import visible in code called from async context
- Test that invokes an MCP tool panics immediately with runtime error
- Store functions (`load_all_contacts`, `write_contact`) called directly in an `async fn`

**Phase to address:**
MCP Server phase -- must be the first architectural decision before writing any tool handlers.

---

### Pitfall 2: Concurrent File Writes from MCP Server Corrupt Contact Files

**What goes wrong:**
The CLI is single-threaded and single-invocation: one user runs one command at a time. The MCP server handles multiple concurrent requests from AI agents. Two simultaneous `edit` or `log` operations on the same contact file cause a read-modify-write race condition. Both read the file, both modify it in memory, one write overwrites the other's changes. With flat file storage and no database, there is no built-in concurrency control.

**Why it happens:**
`store::write_contact()` does a plain `std::fs::write()` with no locking. The `frontmatter::update_field()` pattern reads the full raw YAML, modifies it in memory, and writes back. Two concurrent operations on the same file silently lose one set of changes. No error, no crash, just lost data.

**How to avoid:**
Implement per-file locking in the MCP server layer using a `DashMap<PathBuf, Arc<Mutex<()>>>` or similar structure to serialize access per contact file. Hold the lock for the entire read-modify-write cycle, release after write completes. Combine with atomic writes (write to temp file, then `rename()`) to prevent partial writes. Do NOT use OS-level file locks (flock) -- they are advisory on most Unix systems, unreliable across platforms, and add complexity without benefit for a single-process server.

**Warning signs:**
- MCP test with concurrent tool calls produces inconsistent results
- Interaction log entries disappear intermittently
- Contact field edits are silently reverted

**Phase to address:**
MCP Server phase -- implement locking layer before exposing any write tools.

---

### Pitfall 3: Using Deprecated HTTP+SSE Transport Instead of stdio or Streamable HTTP

**What goes wrong:**
The PROJECT.md specifies "MCP server (HTTP/SSE)" but the MCP specification deprecated HTTP+SSE transport in spec version 2025-03-26, replacing it with Streamable HTTP. Building on SSE means building on a deprecated standard that clients will stop supporting.

**Why it happens:**
Most MCP tutorials from 2024-early 2025 show SSE transport. The deprecation happened mid-2025 and many guides have not been updated. The rmcp crate supports both, so it compiles either way.

**How to avoid:**
Use stdio transport for the initial implementation. This is a local, single-user tool. stdio is the most common MCP transport, most interoperable with AI clients (Claude Desktop, Cursor, etc.), and recommended by the MCP specification for local tools. No HTTP server complexity, no port management, no CORS. If HTTP is needed later, add Streamable HTTP (not SSE) as a second transport option.

**Warning signs:**
- Importing SSE-specific transport types from rmcp
- Setting up HTTP server with `/sse` endpoint
- Configuring CORS headers (unnecessary for stdio)

**Phase to address:**
MCP Server phase -- transport decision at the start.

---

### Pitfall 4: LinkedIn Automation Gets Account Restricted or Banned

**What goes wrong:**
LinkedIn actively detects and restricts accounts using automation tools. Restrictions range from temporary rate limits to permanent account bans. Using Playwright to navigate LinkedIn's UI triggers bot detection via browser fingerprinting, navigation patterns, and request timing analysis.

**Why it happens:**
LinkedIn's bot detection checks: `navigator.webdriver` flag, browser plugin fingerprints, request timing (too fast = bot), navigation flow (skipping intermediate pages), fresh session patterns, and excessive action creation. Playwright's default configuration exposes all of these signals. LinkedIn has stated that in 2025-2026 they will limit visibility of content created via automation tools.

**How to avoid:**
Do NOT automate LinkedIn's web UI for data export. Instead, automate the import of LinkedIn's official GDPR data export (Settings > Data Privacy > Get a copy of your data > Connections). LinkedIn emails a CSV within 10 minutes to 24 hours. The `acrm` tool should automate the CSV import/dedup/change-detection side, not the export side.

If Playwright automation is still desired (marked experimental in PROJECT.md for good reason):
- Target only the GDPR export request page (Settings > Data Privacy), not general browsing
- Use headed mode (not headless) -- LinkedIn detects headless browsers
- Add realistic delays (2-5 seconds between actions, random jitter)
- Implement circuit breaker: if any request returns 429 or a challenge page, stop immediately
- Never run against a primary LinkedIn account for development/testing

**Warning signs:**
- LinkedIn showing CAPTCHA challenges during automation
- "Unusual activity" emails from LinkedIn
- Automation working initially then failing after a few runs

**Phase to address:**
LinkedIn Automation phase -- this should be the LAST phase and marked experimental throughout.

---

### Pitfall 5: playwright-rust Crate Is Immature and Adds Node.js Runtime Dependency

**What goes wrong:**
The `playwright-rust` crate (v0.0.20) is a wrapper around the Node.js Playwright library. It requires Node.js installed at runtime, downloads browser binaries on first use, and has no stable releases. This violates the project constraint of "no runtime dependencies beyond the compiled binary."

**Why it happens:**
Playwright is fundamentally a Node.js tool. The Rust crate is a binding layer, not a native implementation. Every Playwright operation crosses the Rust-to-Node.js FFI boundary.

**How to avoid:**
Write a small standalone Node.js/TypeScript script for the LinkedIn CSV export automation. Call it from Rust via `std::process::Command` only when the user explicitly requests LinkedIn export. Clean separation: the Node.js script can be tested independently, and the core `acrm` binary remains pure Rust with no Node.js dependency. The script is a companion tool in the repo, not a cargo dependency.

Do NOT embed `playwright` or `playwright-rust` as a cargo dependency.

**Warning signs:**
- `playwright` appearing in Cargo.toml dependencies
- Build process requiring `npx playwright install`
- CI needing Node.js installed alongside Rust toolchain for core functionality

**Phase to address:**
LinkedIn Automation phase -- architecture decision at phase start.

---

### Pitfall 6: Bulk Operations Pipe Chains Re-scan All Contacts Per Stage

**What goes wrong:**
`store::load_all_contacts()` reads and parses every `.md` file in `contacts/`. With ~800 contacts currently, this takes perhaps 100ms. Running a pipe chain like `acrm bulk 'status=dormant' --format json | acrm bulk --stdin --set status=archived` re-loads ALL contacts from disk for each pipe stage, multiplying I/O linearly.

**Why it happens:**
Flat file storage has no index. Every query is a full directory scan with YAML parsing. The CLI architecture assumes one-shot execution. Pipe chains multiply the cost by the number of stages.

**How to avoid:**
Design the pipe protocol to pass contact file paths (or IDs) through the pipe rather than full contact data. The receiving command loads only the specific files referenced in stdin. For single commands without piping, accept the full scan -- at <10K contacts (stated out-of-scope threshold for indexing), full scans complete in under 500ms. Do NOT add SQLite or any indexing layer; the project explicitly rules this out.

**Warning signs:**
- Bulk command taking >2 seconds with current contact count
- Pipe chains noticeably slower than equivalent single commands
- Temptation to add an indexing database "just for performance"

**Phase to address:**
Bulk Operations phase -- design the pipe protocol (IDs vs full data) before implementing.

---

### Pitfall 7: MCP Tool Handlers Exposing Unsafe Write Operations Without Guardrails

**What goes wrong:**
An AI agent calling MCP tools has no inherent judgment about destructive operations. A tool like `delete_contact` or `bulk_edit` exposed without guardrails lets an agent delete or modify contacts based on a misunderstood prompt. Unlike CLI usage where a human reviews each command, MCP tool calls happen programmatically and at speed.

**Why it happens:**
MCP tool implementations focus on functionality, not safety constraints. The developer exposes the same capabilities as the CLI without considering that the caller is an AI model that may misinterpret ambiguous instructions.

**How to avoid:**
- Make write tools require explicit confirmation fields: `bulk_edit` should require a `confirm: true` parameter, and the tool description should instruct the agent to confirm with the user first.
- Add a `dry_run` parameter to all write tools. Agents should be encouraged (via tool descriptions) to dry-run first.
- Implement a hard limit on bulk operations via MCP: no more than 50 contacts modified per single tool call.
- Never expose `delete` as a tool that silently succeeds. Return a confirmation prompt that the agent must relay to the user.
- Log all MCP write operations to a dedicated log file for audit.

**Warning signs:**
- MCP tool that modifies contacts without returning what was changed
- No `dry_run` option on write tools
- Agent able to delete contacts without any user confirmation step

**Phase to address:**
MCP Server phase -- bake safety into tool design from the start, not retrofitted.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `spawn_blocking` for all sync code in MCP | Zero refactoring of existing 4.7K LOC | Thread pool overhead, cannot leverage async I/O benefits | Always -- this codebase should stay sync for CLI/TUI |
| No file locking for CLI commands | Simpler code, CLI is single-user | Does not catch race conditions in testing | Always -- CLI is single-process, locking is MCP-only concern |
| Shelling out to Node.js for Playwright | Clean boundary, independent testing | Extra runtime dependency for LinkedIn feature only | Always -- better than embedding playwright-rust |
| Full contact scan for bulk queries | Simple, no index maintenance | O(n) on every query | Until contacts exceed ~10K (explicitly out of scope) |
| Storing MCP server state in memory only | No persistence layer needed | State lost on restart | Always -- MCP server is stateless between sessions by design |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| rmcp (MCP SDK) | Using SSE transport for local tool | Use stdio transport -- simpler, more compatible, no HTTP overhead |
| rmcp tool handlers | Calling `std::fs::read_to_string` directly in async fn | Wrap in `tokio::task::spawn_blocking()` to avoid blocking the async runtime |
| LinkedIn GDPR export | Automating the full LinkedIn UI navigation | Automate only the CSV import; let user trigger export manually or automate just the settings/data-privacy page |
| reqwest in MCP context | Using `reqwest::blocking` inside async handler | `spawn_blocking` the entire sync operation; do not mix blocking and async reqwest in same task |
| Existing `store.rs` from MCP | Assuming store functions are thread-safe | They are not -- add per-file mutex in MCP layer, not in store itself |
| JSON pipe output | Outputting full contact YAML through pipe | Output contact IDs/paths for pipe efficiency; receiver loads only what it needs |
| rmcp tool schemas | Hand-writing JSON schemas for tool parameters | Use rmcp `#[tool]` macro to derive schemas from Rust struct definitions |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Full disk scan per MCP tool call | MCP responses >200ms for simple queries | Cache contact list in memory with file watcher invalidation | >5K contacts or high MCP request rate |
| Parsing YAML frontmatter for every contact on every request | CPU spike on bulk operations | Load file list first, parse only matching files based on filename filter when possible | >2K contacts with complex queries |
| Spawning too many blocking tasks from MCP | Thread pool exhaustion under concurrent requests | Limit concurrent write operations via semaphore (e.g., max 4) | >10 concurrent MCP tool calls |
| LinkedIn automation retry loops | Account gets rate-limited, script loops forever | Exponential backoff with max 3 retries, then fail loudly | Any automated LinkedIn interaction |
| Pipe chains doing redundant I/O | 3-stage pipe takes 3x as long as single command | Pass file paths through pipe, not full contact data | >500 contacts with multi-stage pipes |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| MCP server listening on 0.0.0.0 via HTTP transport | Remote code execution -- any network user can call edit/delete tools | Use stdio transport (no network exposure). If HTTP needed, bind to 127.0.0.1 only |
| LinkedIn credentials stored in plaintext config | Credential theft if repo is public or shared | Use system keyring (same pattern as existing iCloud via `keyring` crate) |
| No input validation on MCP tool arguments | Path traversal via crafted file names in add/edit tools | Reuse existing `validation::validate_contact()`, reject names with `..`, `/`, or null bytes |
| Bulk `--set` accepting arbitrary field names | Could overwrite `id`, `source_id`, or `etag`, breaking sync integrity | Whitelist editable fields in bulk mode; reject system/sync fields |
| MCP tool returning raw file system paths | Leaks local directory structure to AI agent | Return contact names and IDs only; resolve paths internally |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Bulk operation with no preview | User archives 200 contacts accidentally | Show count and sample before executing; require `--yes` flag for >10 contacts |
| MCP tool errors as opaque strings | AI agent cannot recover or explain failure to user | Return structured error with `code`, `message`, and `suggestion` fields |
| LinkedIn export taking 24 hours with no feedback | User thinks tool is broken | Show status message explaining LinkedIn's processing delay; log when request was submitted |
| Bulk query syntax unrelated to existing `search` syntax | User must learn two query languages | Reuse filter logic from `search` command, extend with `field=value` syntax |
| MCP server requiring manual startup | User must remember to start server before AI agent can use it | Document stdio integration for Claude Desktop (just a config entry, no manual start) |

## "Looks Done But Isn't" Checklist

- [ ] **MCP Server:** Tool handlers work in isolation but panic under concurrent calls -- verify with parallel tool invocations
- [ ] **MCP Server:** Read tools work but write tools silently corrupt -- verify with concurrent edit + log on same contact
- [ ] **MCP Server:** Server starts but AI client cannot discover tools -- verify tool listing returns correct JSON schema for all parameters
- [ ] **MCP Server:** Tools work in tests but `reqwest::blocking` code panics in real async context -- verify MCP tool that triggers CardDAV sync completes without panic
- [ ] **Bulk Operations:** Single bulk command works but pipe chains lose data -- verify `acrm bulk ... | acrm bulk ...` produces correct results
- [ ] **Bulk Operations:** `--dry-run` shows correct preview but actual execution differs -- verify dry-run and execute produce same change set
- [ ] **Bulk Operations:** Query syntax handles edge cases -- verify empty string matches, special characters in values, and missing fields
- [ ] **LinkedIn Import:** CSV import works with fresh export but fails on re-import -- verify dedup handles re-imported contacts (match on LinkedIn profile URL or email, not name)
- [ ] **LinkedIn Import:** Import works but overwrites manual CRM edits -- verify CRM-wins conflict resolution applies to LinkedIn re-imports

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| reqwest panic in async context | LOW | Wrap offending call in `spawn_blocking`; no data loss, just a code fix |
| Concurrent write corruption | MEDIUM | Git history preserves all versions; `git diff` to find lost changes; add mutex and replay lost edits |
| LinkedIn account restricted | HIGH | Wait 24-72 hours for restriction lift; reduce automation frequency; may require manual LinkedIn appeal |
| Wrong transport (SSE instead of stdio) | LOW | Swap transport type in server setup code; tool handlers remain unchanged |
| Bulk operation modifies wrong contacts | LOW | `git checkout contacts/` restores all files instantly; add confirmation prompt to prevent recurrence |
| Playwright-rust dependency bloat | MEDIUM | Remove from Cargo.toml, rewrite as standalone Node.js script, update build/CI process |
| MCP tool exposes unsafe delete | MEDIUM | Add dry_run and confirm parameters to tool; audit MCP write log for unintended operations |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| reqwest::blocking panic (P1) | MCP Server | MCP tool that triggers CardDAV sync completes without panic |
| Concurrent file corruption (P2) | MCP Server | 10 parallel write tool calls produce correct, non-corrupted results |
| Deprecated SSE transport (P3) | MCP Server | Server works with Claude Desktop via stdio config in `claude_desktop_config.json` |
| LinkedIn account restriction (P4) | LinkedIn Automation | Automation includes rate limiting, circuit breaker, and user-facing warnings |
| Playwright-rust dependency (P5) | LinkedIn Automation | `acrm` binary has no Node.js runtime dependency; LinkedIn script is standalone |
| Pipe chain performance (P6) | Bulk Operations | Three-stage pipe chain produces same result as equivalent single command, in <2x time |
| Unsafe MCP write tools (P7) | MCP Server | All write tools support `dry_run`; bulk writes require explicit `confirm` parameter |
| Bulk without preview | Bulk Operations | Bulk modify >10 contacts requires explicit `--yes` flag |

## Sources

- [reqwest::blocking should advertise tokio incompatibility (GitHub #1233)](https://github.com/seanmonstar/reqwest/issues/1233) -- HIGH confidence
- [Async -> blocking -> async sandwich causes tokio panics (Rust Forum)](https://users.rust-lang.org/t/async-blocking-async-sandwich-causes-tokio-panics/134538) -- HIGH confidence
- [rmcp -- Official Rust MCP SDK (GitHub)](https://github.com/modelcontextprotocol/rust-sdk) -- HIGH confidence
- [MCP Transports Specification (2025-11-25)](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) -- HIGH confidence
- [Why MCP Deprecated SSE for Streamable HTTP](https://blog.fka.dev/blog/2025-06-06-why-mcp-deprecated-sse-and-go-with-streamable-http/) -- MEDIUM confidence
- [LinkedIn Automation Safety Guide 2026 (Dux-Soup)](https://www.dux-soup.com/blog/linkedin-automation-safety-guide-how-to-avoid-account-restrictions-in-2026) -- MEDIUM confidence
- [Playwright Bot Detection Avoidance (BrowserStack)](https://www.browserstack.com/guide/playwright-bot-detection) -- MEDIUM confidence
- [playwright-rust crate (GitHub)](https://github.com/octaltree/playwright-rust) -- HIGH confidence (crate status verified)
- [Tokio Shared State Tutorial](https://tokio.rs/tokio/tutorial/shared-state) -- HIGH confidence
- [How to Build an MCP Server in Rust](https://oneuptime.com/blog/post/2026-01-07-rust-mcp-server/view) -- MEDIUM confidence
- Existing codebase inspection: `src/store.rs`, `src/sync/carddav.rs`, `src/main.rs`, `Cargo.toml` -- HIGH confidence

---
*Pitfalls research for: Adding MCP server, bulk operations, and LinkedIn automation to AgenticCRM v1.2*
*Researched: 2026-03-08*
