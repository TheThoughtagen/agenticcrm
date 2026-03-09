---
status: diagnosed
trigger: "acrm bulk status=active (query-only mode) lists all 985+ contacts without truncation"
created: 2026-03-09T00:00:00Z
updated: 2026-03-09T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED - query-only path delegates to format::output_list which has no truncation logic
test: Read both code paths
expecting: truncation missing in query-only path
next_action: report diagnosis

## Symptoms

expected: Query-only bulk command truncates at 20 contacts with "...and N more"
actual: All 985+ contacts listed without truncation
errors: none
reproduction: `acrm bulk status=active` (no action flags)
started: since implementation

## Eliminated

(none needed - root cause found on first hypothesis)

## Evidence

- timestamp: 2026-03-09
  checked: src/commands/bulk.rs lines 191-203 (query-only path)
  found: Query-only mode builds a Vec<SearchMatch> from ALL matched contacts and passes the full list to format::output_list()
  implication: No truncation applied before output

- timestamp: 2026-03-09
  checked: src/commands/bulk.rs lines 70-92 (print_preview function)
  found: print_preview() has correct truncation logic - displays first 20 and prints "...and N more" - but this function is ONLY called in the action-mode path (line 209), NOT in the query-only path
  implication: Truncation exists but is wired to the wrong code path

- timestamp: 2026-03-09
  checked: src/format.rs lines 30-48 (output_list function)
  found: output_list iterates ALL items with `for item in data { println!("{item}"); }` - no truncation logic whatsoever
  implication: The generic output_list function was never designed to truncate

## Resolution

root_cause: |
  Two code paths exist in run_bulk():
  1. Query-only (line 192-203): calls format::output_list() which prints every item
  2. Action mode (line 206-224): calls print_preview() which truncates at 20

  The query-only path at line 202 passes the full results vec to format::output_list(),
  and output_list (src/format.rs:36-41) simply iterates and prints all items.
  There is no truncation in either output_list or the query-only code path.

  The same issue exists in run_bulk_update() at line 341 for the stdin no-action path.

fix: ""
verification: ""
files_changed: []
