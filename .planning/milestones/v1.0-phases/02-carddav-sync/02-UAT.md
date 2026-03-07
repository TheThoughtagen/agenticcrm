---
status: complete
phase: 02-carddav-sync
source: [02-01-SUMMARY.md, 02-02-SUMMARY.md, 02-03-SUMMARY.md]
started: 2026-03-06T15:00:00Z
updated: 2026-03-06T15:08:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Build and tests pass
expected: Run `cargo build` and `cargo test` — both succeed with zero failures. All 62+ tests pass.
result: pass

### 2. Sync command exists in CLI
expected: Run `acrm sync --help` — shows sync subcommand with `setup` action, `--dry-run`, and `--force` flags.
result: pass

### 3. Sync setup configures credentials
expected: Run `acrm sync setup` — prompts for Apple ID and app-specific password interactively, stores password in macOS Keychain.
result: pass

### 4. Sync pulls contacts from iCloud (SYNC-01)
expected: Run `acrm sync` — connects to iCloud CardDAV, discovers addressbook, and creates new markdown contact files in `contacts/` for each iCloud contact.
result: pass

### 5. vCard fields mapped correctly (SYNC-02)
expected: Open a synced contact file — name, email, phone, and organization fields from the vCard are correctly mapped to YAML frontmatter fields.
result: pass

### 6. Sync metadata in frontmatter (SYNC-04)
expected: Open a synced contact file — frontmatter contains `source: "icloud"`, a `source_id` (CardDAV UID), and an `etag` value.
result: pass

### 7. Duplicate detection on re-sync (SYNC-03)
expected: Run `acrm sync` again — previously imported contacts are NOT duplicated. Output shows "skipped" or "unchanged" for existing contacts.
result: pass

### 8. Dry-run mode previews without writing
expected: Run `acrm sync --dry-run` — shows what would be synced but does NOT create or modify any contact files.
result: pass

### 9. Force mode re-downloads all
expected: Run `acrm sync --force` — re-downloads and updates all contacts regardless of ETag match.
result: pass

## Summary

total: 9
passed: 9
issues: 0
pending: 0
skipped: 0

## Gaps

[none]
