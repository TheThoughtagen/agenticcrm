# Research Summary: AgenticCRM Milestone 2

**Domain:** Personal CRM extensions -- CardDAV sync, TUI, MCP server, JSON output
**Researched:** 2026-03-05
**Overall confidence:** MEDIUM (web research tools unavailable; versions from May 2025 training data)

## Executive Summary

This milestone adds four capabilities to the existing Rust CLI CRM: JSON output mode, an interactive TUI, two-way iCloud CardDAV sync, and an MCP server for AI agent integration. The Rust ecosystem is well-positioned for three of these four; CardDAV sync requires the most custom work.

JSON output is trivial -- the Contact struct already derives serde Serialize, so adding `serde_json` and a `--json` flag is a half-day task. The TUI story is clear: ratatui is the undisputed Rust TUI framework with no serious competitors. MCP server has a less mature Rust SDK situation but the protocol (JSON-RPC 2.0 over stdio) is simple enough to implement directly if needed.

CardDAV sync is the most complex and risky feature. There is no mature Rust CardDAV client library. The project will need to build a thin CardDAV client on top of reqwest + quick-xml, implement vCard-to-Contact mapping, and handle iCloud's authentication and discovery flow. The protocol itself is well-documented (RFC 6352) but the implementation surface area is non-trivial -- expect this to be the largest phase.

The key architectural decision is introducing tokio as the async runtime. Both reqwest (for CardDAV) and MCP server (for async stdio) need it. This means the sync and MCP features pull in a significant dependency tree, but this is unavoidable for network I/O in Rust.

## Key Findings

**Stack:** ratatui for TUI, reqwest+quick-xml for CardDAV, tokio for async, serde_json for JSON output. MCP SDK needs verification.
**Architecture:** New features are additive modules -- no changes to existing CLI command architecture. TUI is a new entry point, sync is a new subcommand, MCP is a new binary target.
**Critical pitfall:** CardDAV/vCard has no mature Rust library; plan for custom implementation. MCP Rust SDK maturity is uncertain.

## Implications for Roadmap

Based on research, suggested phase structure:

1. **JSON Output** - Lowest risk, highest immediate value for agent integration
   - Addresses: `--json` flag on all commands
   - Avoids: No new dependencies beyond serde_json
   - Effort: Small (1-2 days)

2. **TUI with ratatui** - Well-understood problem, clear library choice
   - Addresses: Interactive dashboard and contact browser
   - Avoids: Can develop independently of sync/MCP
   - Effort: Medium (1-2 weeks for solid UX)

3. **CardDAV/iCloud Sync** - Highest complexity, most custom code
   - Addresses: Two-way iCloud contact sync
   - Avoids: Depends on JSON output (for testing/debugging sync)
   - Effort: Large (2-3 weeks including discovery, vCard mapping, conflict resolution)

4. **MCP Server** - Depends on JSON output working, benefits from stable data layer
   - Addresses: AI agent integration via MCP protocol
   - Avoids: Should come after sync so MCP can expose sync operations
   - Effort: Medium (1 week)

**Phase ordering rationale:**
- JSON output first because it is trivial and immediately useful for debugging everything else
- TUI second because it is self-contained and does not depend on network features
- CardDAV third because it is the hardest and benefits from JSON output for debugging
- MCP fourth because it wraps existing functionality and benefits from stable CLI/sync layer

**Research flags for phases:**
- Phase 3 (CardDAV): NEEDS deeper research -- verify vCard crate situation, test iCloud discovery flow
- Phase 4 (MCP): NEEDS verification -- check current state of Rust MCP SDK crates

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack (TUI) | HIGH | ratatui is unambiguously the right choice |
| Stack (CardDAV) | MEDIUM | reqwest/quick-xml are solid; vCard parsing crate situation unclear |
| Stack (MCP) | LOW | Rust MCP SDK maturity unknown; may need custom implementation |
| Stack (JSON) | HIGH | serde_json, trivial addition |
| Features | HIGH | Clear requirements from PROJECT.md |
| Architecture | HIGH | Additive modules, no existing code changes needed |
| Pitfalls | MEDIUM | CardDAV complexity is real; iCloud-specific quirks need testing |

## Gaps to Address

- Verify exact latest versions of ratatui, crossterm, reqwest, quick-xml on crates.io
- Identify canonical Rust MCP SDK crate (search crates.io for `mcp`, `rmcp`, `mcp-server`)
- Test iCloud CardDAV discovery flow with a real Apple ID + app-specific password
- Evaluate vCard parsing options: `vcard` crate vs `ical` crate vs manual parser
- Determine if iCloud returns vCard 3.0 or 4.0 format (likely 3.0)

---

*Research summary: 2026-03-05*
