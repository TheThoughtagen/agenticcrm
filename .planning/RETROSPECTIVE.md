# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-03-06
**Phases:** 3 | **Plans:** 9 | **Tasks:** 18

### What Was Built
- Rust CLI (`acrm`) with full CRUD, JSON output, validation, and cadence-based follow-up calculation
- iCloud CardDAV sync with PROPFIND discovery chain, vCard mapping, dedup by source_id, and macOS Keychain credentials
- Interactive ratatui TUI with contact browser, split-pane detail view, real-time search, follow-up dashboard, and modal interaction logging

### What Worked
- **TEA pattern for TUI** — Screen/InputMode/Message enums made state transitions predictable and easy to extend across 3 plans
- **Raw frontmatter preservation** — storing raw YAML as String field meant round-trip editing never corrupted comments or field order
- **Phase independence** — Phase 02 (sync) and Phase 03 (TUI) both depended only on Phase 01, enabling clean parallel design
- **Auto-fix pattern** — 8 blocking issues were auto-fixed during execution without manual intervention or scope creep
- **Quick depth execution** — 38 minutes total execution across 9 plans, averaging 4 min/plan

### What Was Inefficient
- **Phase 02 missing VERIFICATION.md** — no formal verification document exists despite the phase being functionally complete and human-verified; created a gap that showed up in milestone audit
- **Duplicate decisions in STATE.md** — some decisions logged twice (with and without `[Phase XX]` prefix), indicating state accumulation could be cleaner
- **SyncConfig dead code** — struct was planned but never used; should have been removed during execution rather than carried as tech debt

### Patterns Established
- `Serialize+Display` dual output pattern for all CLI command types (enables `--format json` universally)
- `find_single_contact` shared helper for partial name matching across commands
- Standalone widget extraction (search bar) for TUI component reuse
- Stdout capture pattern for subprocess calls within TUI to prevent display corruption
- Direct file write pattern for existing contacts (bypasses store slug re-derivation)

### Key Lessons
1. **Verify every phase** — skipping VERIFICATION.md creates audit gaps even when code is functionally correct; the 5 minutes to verify saves rework later
2. **calcard + quick-xml API churn** — Rust crate APIs change between minor versions; pin versions and expect adaptation during execution
3. **reqwest blocking over async** — for CLI tools without concurrent I/O needs, blocking HTTP is simpler and the codebase stays smaller
4. **Dead code warnings are signals** — SyncConfig should have been removed immediately; compiler warnings left unaddressed become tech debt

### Cost Observations
- Model mix: 100% opus (balanced profile)
- Sessions: ~3 (project init, execution, audit/completion)
- Notable: 38 minutes of execution time for 4,721 LOC Rust across 3 phases — extremely efficient for a full CLI + sync + TUI stack

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Execution Time | Phases | Plans | Key Change |
|-----------|---------------|--------|-------|------------|
| v1.0 | 38 min | 3 | 9 | Baseline — established patterns for CLI, sync, TUI |

### Cumulative Quality

| Milestone | Tests | Auto-Fixes | Tech Debt Items |
|-----------|-------|------------|-----------------|
| v1.0 | 62 | 8 | 3 |

### Top Lessons (Verified Across Milestones)

1. Always create VERIFICATION.md for every phase — audit gaps are preventable
2. Direct file write pattern is reliable for existing contacts but bypasses validation; acceptable trade-off for known-good data
