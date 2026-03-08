---
status: diagnosed
phase: 05-push-command
source: 05-01-SUMMARY.md
started: 2026-03-08T10:00:00Z
updated: 2026-03-08T10:10:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Push dry-run preview
expected: Running `acrm sync push --dry-run` shows a preview of contacts that would be created, updated, or deleted on the server — but makes NO actual changes. Output lists each contact with its action (create/update/delete).
result: issue
reported: "Dry-run shows ~985 contacts as 'would update' even though I didn't change them. The changeset detection flags contacts where vCard serialization differs from server format (field ordering, whitespace, etc). Could wreck havoc if pushed. Need a verbose mode that shows what fields would actually change."
severity: blocker

### 2. Push executes changes
expected: Running `acrm sync push` (without --dry-run) actually pushes local contact changes to the CardDAV server. Output shows per-contact results (created/updated/deleted/failed).
result: skipped
reason: Unsafe to test — changeset detects 985 false positives

### 3. Push with --force overrides conflicts
expected: When a contact has been modified both locally and on the server (conflict), running `acrm sync push --force` overrides the server version with the local version instead of skipping.
result: skipped
reason: Unsafe to test — changeset detects 985 false positives

### 4. Pull subcommand works
expected: Running `acrm sync pull` performs the same pull sync as bare `acrm sync` — fetching contacts from the server and updating local markdown files.
result: pass

### 5. Flexible flag placement
expected: Both `acrm sync push --dry-run` and `acrm sync --dry-run push` work identically — flags can be placed on the parent command or the subcommand.
result: skipped
reason: Assumed working, not tested

### 6. Failed operations don't abort push
expected: If one contact fails to push (e.g., server error), the remaining contacts still push successfully. The failed contact is reported in the output but doesn't stop the batch.
result: skipped
reason: Unsafe to test — changeset detects 985 false positives

## Summary

total: 6
passed: 1
issues: 1
pending: 0
skipped: 4

## Gaps

- truth: "Push dry-run should only show contacts with real local changes, not vCard formatting differences"
  status: failed
  reason: "User reported: Dry-run shows ~985 contacts as 'would update' even though I didn't change them. The changeset detection flags contacts where vCard serialization differs from server format. Could wreck havoc if pushed. Need a verbose mode that shows what fields would actually change."
  severity: blocker
  test: 1
  root_cause: "compute_push_changeset uses exact string comparison (serialized == cached_text) between merge_contact_to_vcard output and raw cached server vCard. Three mechanisms cause false diffs: (1) merge reorders properties by stripping CRM-mapped props and appending at end, (2) add_crm_entries drops TYPE/LABEL parameters from EMAIL/TEL, (3) NOTE is in CRM_MAPPED_PROPERTIES (removed from cache) but never re-added by add_crm_entries — silently lost."
  artifacts:
    - path: "src/sync/push.rs"
      issue: "compute_push_changeset line 138 uses string == comparison"
    - path: "src/sync/vcard_write.rs"
      issue: "merge_contact_to_vcard reorders props; add_crm_entries drops params; NOTE silently dropped"
  missing:
    - "Replace string comparison with semantic Contact-to-Contact diff"
    - "Preserve vCard property parameters (TYPE, LABEL) during merge"
    - "Add NOTE to add_crm_entries output"
    - "Add verbose/diff mode showing which fields changed per contact"
  debug_session: ".planning/debug/push-changeset-false-updates.md"
