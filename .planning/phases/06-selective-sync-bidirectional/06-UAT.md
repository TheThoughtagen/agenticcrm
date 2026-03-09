---
status: complete
phase: 06-selective-sync-bidirectional
source: 06-01-SUMMARY.md, 06-02-SUMMARY.md
started: 2026-03-08T20:10:00Z
updated: 2026-03-08T20:20:00Z
---

## Current Test

[testing complete]

## Tests

### 1. CLI --tag and --status flags on all sync commands
expected: All three sync commands (sync, sync pull, sync push) show --tag and --status flags in help output
result: pass

### 2. Independent subcommands still work
expected: `acrm sync pull --help` and `acrm sync push --help` each show single-direction descriptions with all expected flags
result: pass

### 3. Bare `acrm sync` describes bidirectional behavior
expected: Top-level `acrm sync --help` says "Sync contacts with iCloud (pull then push)" indicating bidirectional
result: pass

### 4. All unit tests pass
expected: `cargo test` passes all tests including SyncFilter (tag matching, status matching, config+CLI merge, empty filter passthrough)
result: pass

### 5. Live sync filtering (tag/status on pull and push)
expected: Pull/push with --tag/--status flags only syncs matching contacts
result: skipped
reason: No tags on contacts currently, can't observe filtering behavior

### 6. CLI flags override config filters
expected: CLI --tag overrides sync.toml push_filters.tags (replaces, not unions)
result: skipped
reason: Requires tagged contacts to observe override behavior; unit tests cover this logic

### 7. --dry-run propagates in bidirectional mode
expected: `acrm sync --dry-run` shows both Pull and Push phases executing without making changes
result: pass

## Summary

total: 7
passed: 5
issues: 0
pending: 0
skipped: 2

## Gaps

[none yet]
