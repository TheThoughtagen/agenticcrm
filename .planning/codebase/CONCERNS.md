# Codebase Concerns

**Analysis Date:** 2026-03-05

## Tech Debt

**No Input Validation on Interaction Types:**
- Issue: `acrm log` accepts any string as `interaction_type`. CLAUDE.md defines a fixed set (coffee, call, email, message, conference, meeting, lunch, intro) but the CLI does not enforce it.
- Files: `src/commands/log.rs` (line 6), `src/main.rs` (line 43)
- Impact: Inconsistent interaction types make querying and filtering unreliable. Typos go undetected.
- Fix approach: Create an `InteractionType` enum in `src/models/contact.rs` with serde deserialization, and validate the `--type` argument against it in the CLI or via `clap::ValueEnum`.

**No Validation on `follow_up_cadence` Values:**
- Issue: `follow_up_cadence` is a free-form `String` ("monthly", "quarterly", etc.) with no parsing or validation. The `log` command updates `last_contacted` but never computes or updates `next_follow_up` based on cadence.
- Files: `src/models/contact.rs` (line 97), `src/commands/log.rs` (lines 47-48)
- Impact: `acrm due` is only useful if `next_follow_up` is manually maintained. The cadence field is effectively dead data.
- Fix approach: Parse cadence into a duration enum, and auto-compute `next_follow_up = last_contacted + cadence` when logging interactions.

**Hardcoded Default CRM Root Path:**
- Issue: `find_crm_root()` falls back to `~/repos/agenticcrm` which is developer-specific.
- Files: `src/store.rs` (line 100)
- Impact: Other users will get confusing errors or silently use the wrong directory.
- Fix approach: Remove the hardcoded fallback. Rely only on `ACRM_ROOT` env var or current directory detection.

**Duplicate Code: Name-Matching Logic:**
- Issue: The partial name matching pattern (lowercase contains + bail on multiple matches) is duplicated across `show.rs`, `log.rs`. `search.rs` uses a different matching approach (no bail on multiple).
- Files: `src/commands/show.rs` (lines 9-22), `src/commands/log.rs` (lines 9-22)
- Impact: Inconsistent behavior risk if one is updated without the other. Maintenance burden.
- Fix approach: Extract a `resolve_single_contact()` function into `src/store.rs` that handles matching, ambiguity errors, and "not found" errors.

**`Cargo.lock` Not in `.gitignore` but Not Committed:**
- Issue: `Cargo.lock` exists locally but is untracked (shown in `git status`). For binary applications, `Cargo.lock` should be committed per Rust conventions.
- Files: `Cargo.lock`, `.gitignore`
- Impact: Builds are not reproducible across machines.
- Fix approach: Commit `Cargo.lock` to version control.

**Shell Scripts Duplicate Rust CLI Functionality:**
- Issue: Four shell scripts (`add-contact.sh`, `search.sh`, `due-followups.sh`, `import-linkedin.sh`) overlap with Rust CLI commands (`add`, `search`, `due`). The LinkedIn import script has no Rust equivalent.
- Files: `scripts/add-contact.sh`, `scripts/search.sh`, `scripts/due-followups.sh`, `scripts/import-linkedin.sh`
- Impact: Two codepaths for the same operations that can drift apart. The shell `add-contact.sh` uses the template file while the Rust `add` command constructs the contact programmatically, meaning field ordering and YAML comments differ.
- Fix approach: Deprecate shell scripts in favor of Rust CLI. Add an `acrm import linkedin` subcommand. Keep scripts only as thin wrappers if needed.

**Rust `add` Command Does Not Use Template:**
- Issue: `src/commands/add.rs` constructs a `Contact` struct with hardcoded defaults rather than reading from `templates/contact.md`. The template includes YAML comments (e.g., `# Contact`, `# Professional`) and an HTML comment in the interaction log, which the Rust output lacks.
- Files: `src/commands/add.rs`, `templates/contact.md`
- Impact: Contacts created via `acrm add` differ structurally from those created via the shell script or template. YAML output from `serde_yaml` strips comments and may reorder fields.
- Fix approach: Either read and populate the template file in the Rust `add` command, or accept the serde-generated format as canonical and update the template to match.

## Known Bugs

**`unwrap()` on `next_follow_up` in `due.rs`:**
- Symptoms: Panic if a contact somehow passes the filter with `None` for `next_follow_up`.
- Files: `src/commands/due.rs` (line 30)
- Trigger: Currently guarded by the `.is_some_and()` filter on line 15, so this is safe in practice. However, a future refactor could break the invariant.
- Workaround: The filter prevents `None` from reaching line 30.
- Fix: Replace with `if let Some(follow_up) = c.next_follow_up` or use `unwrap_or_default()`.

**LinkedIn Import Script Fragile CSV Parsing:**
- Symptoms: Malformed or quoted CSV fields (containing commas, newlines) will break the import.
- Files: `scripts/import-linkedin.sh` (line 23)
- Trigger: Any LinkedIn connection with a company name containing a comma (e.g., "Smith, Jones & Associates").
- Workaround: Manually clean CSV before importing.

**`serde_yaml` Crate Is Deprecated:**
- Symptoms: No bug yet, but `serde_yaml` 0.9 is unmaintained. The author has archived the crate.
- Files: `Cargo.toml` (line 10)
- Trigger: Future Rust edition or dependency updates may cause breakage.
- Workaround: None needed currently.
- Fix: Migrate to `serde_yml` (community fork) or another YAML library.

## Security Considerations

**No Input Sanitization on Contact Names:**
- Risk: Contact names are used directly to construct file paths via `slug()`. While `slug()` filters to alphanumeric and hyphens, the shell script `add-contact.sh` uses `tr` which could behave differently. Names with special characters could cause path traversal in the shell scripts.
- Files: `src/models/contact.rs` (lines 113-122), `scripts/add-contact.sh` (line 16)
- Current mitigation: The Rust `slug()` function filters characters safely. Shell script uses `tr -cd '[:alnum:]-'`.
- Recommendations: Ensure the shell and Rust slug functions produce identical output. Add explicit path traversal check (no `..` in slug).

**LinkedIn Import Injects Unescaped Data into YAML:**
- Risk: LinkedIn profile data (names, company names) containing YAML special characters (colons, quotes, brackets) could produce malformed or injectable YAML frontmatter.
- Files: `scripts/import-linkedin.sh` (lines 45-101)
- Current mitigation: Values are wrapped in double quotes in the heredoc, but nested quotes in the data are not escaped.
- Recommendations: Escape double quotes in input fields before writing to YAML, or rewrite the import in Rust using `serde_yaml` for proper serialization.

## Performance Bottlenecks

**Full Contact Scan on Every Command:**
- Problem: Every command (`list`, `search`, `show`, `log`, `due`) calls `load_all_contacts()` which reads and parses every `.md` file in `contacts/`.
- Files: `src/store.rs` (lines 54-71)
- Cause: No index or cache. File I/O + YAML parsing for every contact on every invocation.
- Improvement path: For < 1000 contacts this is fine (sub-second). For larger collections, consider a cached index (e.g., SQLite or a JSON manifest regenerated on change). Alternatively, for `show` and `log`, resolve the slug from the name first and read only the matching file.

## Fragile Areas

**Frontmatter Parser:**
- Files: `src/store.rs` (lines 25-45)
- Why fragile: Hand-rolled frontmatter parsing with string slicing. Relies on exact `---` delimiter placement. Does not handle edge cases like `---` appearing in the body content, or Windows line endings (`\r\n`).
- Safe modification: Add comprehensive tests before changing. Consider using a dedicated frontmatter parsing crate.
- Test coverage: Zero tests exist for this parser.

**YAML Serialization Round-Trip Fidelity:**
- Files: `src/store.rs` (lines 48-51)
- Why fragile: Reading a hand-written YAML file with comments and serializing it back with `serde_yaml` strips all YAML comments, may reorder fields, and changes formatting. Every `acrm log` operation rewrites the entire file, losing comments.
- Safe modification: Either accept comment loss as a design choice, or switch to a YAML library that preserves comments (e.g., `yaml-rust2` with manual manipulation).
- Test coverage: No tests.

## Scaling Limits

**File-Per-Contact Storage Model:**
- Current capacity: Works well for personal CRM use (hundreds of contacts).
- Limit: At ~10,000+ contacts, directory listing and full scans will become noticeably slow. Git performance may degrade with many small files.
- Scaling path: Add an index file or embedded database for queries while keeping markdown as the source of truth.

## Dependencies at Risk

**`serde_yaml` (0.9):**
- Risk: Crate is archived/deprecated by its author (dtolnay).
- Impact: No future bug fixes or compatibility updates.
- Migration plan: Switch to `serde_yml` (community fork maintaining API compatibility) or evaluate `yaml-rust2`.

## Missing Critical Features

**No Delete/Archive Command:**
- Problem: No `acrm delete` or `acrm archive` command exists.
- Blocks: Users must manually delete or move contact files.

**No Edit Command:**
- Problem: No `acrm edit` command to update contact fields from the CLI.
- Blocks: All field updates (except `last_contacted` via `log`) require manually editing markdown files.

**No Import Command in Rust CLI:**
- Problem: LinkedIn import only exists as a shell script with fragile CSV parsing.
- Blocks: Reliable bulk import workflow.

## Test Coverage Gaps

**Zero Test Coverage:**
- What's not tested: The entire codebase has no tests. No unit tests, no integration tests, no test files exist anywhere.
- Files: All files in `src/` -- `src/store.rs`, `src/models/contact.rs`, `src/commands/*.rs`
- Risk: Any refactoring (especially to the frontmatter parser, slug generation, or YAML round-tripping) could silently break functionality. The name-matching logic, date handling, and file I/O are all untested.
- Priority: **High** -- The frontmatter parser (`src/store.rs` lines 25-45) and slug generation (`src/models/contact.rs` lines 113-122) are the highest-priority targets for testing, as they handle data integrity.

---

*Concerns audit: 2026-03-05*
