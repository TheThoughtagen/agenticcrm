---
status: diagnosed
phase: 10-linkedin-import
source: 10-01-SUMMARY.md, 10-02-SUMMARY.md
started: 2026-03-09T18:30:00Z
updated: 2026-03-09T18:35:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Import LinkedIn CSV - New Contacts Created
expected: Running `acrm import linkedin <file>` creates new contact files in contacts/ for each row. Output shows "Created" entries with names and fields set.
result: pass

### 2. Dry-Run Mode Preview
expected: Running `acrm import linkedin <file> --dry-run` shows what would happen without actually creating/modifying files. No new files appear in contacts/.
result: issue
reported: "dry-run creates the contact file via ops::contact::add() before the dry_run guard at line 260. File is created with empty frontmatter but the display says 'Would import'. The skeleton file persists on disk."
severity: major

### 3. JSON Output Format
expected: Running `acrm import linkedin <file> --format json --dry-run` outputs valid JSON with created/updated/skipped/detected_changes and dry_run flag.
result: pass

### 4. Duplicate Detection on Re-Import
expected: Running import again on same CSV does NOT create duplicate contacts. Existing contacts matched by name/email shown as "Skipped".
result: issue
reported: "No duplicates created (good), but output shows '0 created, 0 updated, 0 skipped' - the 2 matched rows silently drop through without being counted in any category. Expected them in 'skipped' count for user visibility."
severity: minor

### 5. Fill-Empty Merge (Existing Contact Updated)
expected: If existing contact has empty company/role fields, re-importing CSV with those fields fills them without overwriting non-empty ones. Shows "Updated" with field list.
result: pass

### 6. Contact File Content Correct
expected: Newly created contact has correct YAML frontmatter: company, role, email array, source: "linkedin", met_date from Connected On, tags: ["linkedin"], relationship: colleague.
result: pass

### 7. Non-Existent File Error
expected: Running `acrm import linkedin /tmp/nonexistent.csv` shows clear error message, does not crash.
result: pass

## Summary

total: 7
passed: 5
issues: 2
pending: 0
skipped: 0

## Gaps

- truth: "Dry-run mode previews changes without writing any files to disk"
  status: failed
  reason: "User reported: dry-run creates the contact file via ops::contact::add() before the dry_run guard at line 260. File is created with empty frontmatter but the display says 'Would import'. The skeleton file persists on disk."
  severity: major
  test: 2
  root_cause: "In import_linkedin() Ok(None) branch (line 187-277), ops::contact::add() is called unconditionally at line 192. The if !dry_run guard at line 260 only protects the frontmatter write-back, not the initial file creation. The add() call creates the skeleton contact file on disk regardless of dry_run flag."
  artifacts:
    - path: "src/ops/import.rs"
      issue: "ops::contact::add() called before dry_run check in Ok(None) branch (line 192 vs guard at line 260)"
  missing:
    - "Wrap the entire Ok(None) create branch in if !dry_run, or skip the add() call and only build the ImportChange record when dry_run is true"
  debug_session: ""

- truth: "Re-importing existing contacts shows them in the skipped count for user visibility"
  status: failed
  reason: "User reported: No duplicates created (good), but output shows '0 created, 0 updated, 0 skipped' - the 2 matched rows silently drop through without being counted in any category. Expected them in 'skipped' count for user visibility."
  severity: minor
  test: 4
  root_cause: "In Ok(Some(existing_cf)) branch (line 279-407), when fields_changed is empty (all fields already populated), the code falls through without adding the contact to any result category. The if !fields_changed.is_empty() guard at line 386 only adds to 'updated' — there is no else branch to add to 'skipped'."
  artifacts:
    - path: "src/ops/import.rs"
      issue: "Missing else branch at line 386 — no-change matches not added to skipped"
  missing:
    - "Add else branch after line 406: push ImportSkip with reason 'no changes needed' when fields_changed is empty"
  debug_session: ""
