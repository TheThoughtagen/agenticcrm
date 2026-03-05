# External Integrations

**Analysis Date:** 2026-03-05

## APIs & External Services

**None currently integrated.** This is a fully offline, local-only application. No external API calls are made by any code in the codebase.

**Planned (from `README.md`, not yet implemented):**
- Apple/iCloud Contacts via CardDAV (`vdirsyncer`)
- LinkedIn (CSV export import only - no API)
- Outlook/Exchange via Microsoft Graph API
- Facebook (data export import)
- X/Twitter (API or data export)

## Data Storage

**Databases:**
- None. All data is stored as plain-text markdown files with YAML frontmatter in `contacts/`.
- Contact files: `contacts/{firstname-lastname}.md`
- Version history managed by Git (not the application itself)

**File Storage:**
- Local filesystem only
- CRM root resolved by `src/store.rs:find_crm_root()`:
  1. `ACRM_ROOT` environment variable
  2. Current working directory (if `contacts/` and `templates/` exist)
  3. `~/repos/agenticcrm` (hardcoded fallback)

**Caching:**
- None. Contacts are read from disk on every command invocation.

## Authentication & Identity

**Auth Provider:**
- None. No authentication. Local single-user tool.

## Monitoring & Observability

**Error Tracking:**
- None. Errors printed to stderr via `anyhow` error chains and `eprintln!` warnings.

**Logs:**
- No structured logging. CLI output goes to stdout, warnings/errors to stderr.

## CI/CD & Deployment

**Hosting:**
- Local machine only. No deployment pipeline.

**CI Pipeline:**
- None detected. No GitHub Actions, no CI configuration files.

## Environment Configuration

**Required env vars:**
- None required

**Optional env vars:**
- `ACRM_ROOT` - Override CRM data directory location (checked in `src/store.rs`)

**Secrets location:**
- No secrets required. `.gitignore` excludes `.env` and `credentials/` preemptively.

## Import Sources

**LinkedIn CSV:**
- Script: `scripts/import-linkedin.sh`
- Input: LinkedIn `Connections.csv` export file
- Process: Parses CSV, creates one markdown contact file per row
- Fields mapped: first name, last name, email, company, position, connected date
- Tags imported contacts with `linkedin-import`
- Sets `source: linkedin` in frontmatter

**Manual Entry:**
- Script: `scripts/add-contact.sh` - creates contact from template
- Rust CLI: `acrm add "Name"` - creates contact programmatically

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None

## System Tool Dependencies

**Shell scripts require:**
- `uuidgen` - UUID generation (`scripts/add-contact.sh`, `scripts/import-linkedin.sh`)
- `grep` - Text search (`scripts/search.sh`)
- `sed` - Text extraction from frontmatter (`scripts/search.sh`, `scripts/due-followups.sh`)
- `date` - Current date formatting (`scripts/due-followups.sh`)

---

*Integration audit: 2026-03-05*
