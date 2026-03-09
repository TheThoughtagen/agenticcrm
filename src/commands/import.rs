use std::fmt;
use std::path::Path;

use colored::Colorize;

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::import::ImportResult;
use crate::store;

// ── Display implementation ──────────────────────────────────────────────────

impl fmt::Display for ImportResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (created, updated, skipped, _warnings) = self.summary_counts();

        // Summary line
        if self.dry_run {
            write!(
                f,
                "[DRY RUN] Would import: {} created, {} updated, {} skipped",
                created, updated, skipped
            )?;
        } else {
            write!(
                f,
                "Imported: {} created, {} updated, {} skipped",
                created, updated, skipped
            )?;
        }

        // Created section
        if !self.created.is_empty() {
            write!(f, "\n\nCreated:")?;
            for change in &self.created {
                write!(
                    f,
                    "\n  {} {}",
                    change.name.bold(),
                    format!("({})", change.fields.join(", ")).dimmed()
                )?;
            }
        }

        // Updated section
        if !self.updated.is_empty() {
            write!(f, "\n\nUpdated:")?;
            for change in &self.updated {
                write!(
                    f,
                    "\n  {} {}",
                    change.name.bold(),
                    format!("({})", change.fields.join(", ")).dimmed()
                )?;
            }
        }

        // Detected changes section
        if !self.detected_changes.is_empty() {
            write!(
                f,
                "\n\n{}",
                "Detected changes (not applied -- CRM fields already set):".dimmed()
            )?;
            for dc in &self.detected_changes {
                write!(
                    f,
                    "\n  {}: {} {} -> {}",
                    dc.name.bold(),
                    dc.field,
                    dc.crm_value.dimmed(),
                    dc.linkedin_value
                )?;
            }
        }

        // Skipped section
        if !self.skipped.is_empty() {
            write!(f, "\n\nSkipped:")?;
            for skip in &self.skipped {
                write!(
                    f,
                    "\n  {}: {}",
                    skip.name.bold(),
                    skip.reason.dimmed()
                )?;
            }
        }

        // Warnings section
        if !self.warnings.is_empty() {
            write!(f, "\n\nWarnings:")?;
            for warning in &self.warnings {
                write!(f, "\n  {}", warning.dimmed())?;
            }
        }

        Ok(())
    }
}

// ── Handler ─────────────────────────────────────────────────────────────────

pub fn run_import_linkedin(file: &Path, dry_run: bool, fmt: &OutputFormat) -> anyhow::Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let root = store::find_crm_root()?;
    let result = ops::import::import_linkedin(&root, file, dry_run)?;
    format::output(&result, fmt)?;

    Ok(())
}
