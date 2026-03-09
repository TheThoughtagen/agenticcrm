use std::fmt;

use anyhow::{Context, Result};

use crate::format::{self, OutputFormat};
use crate::ops;
use crate::ops::sync::{PushSyncResult, SyncCredentials, SyncOpts, SyncResult};
use crate::sync::{config, filter::SyncFilter};

impl fmt::Display for SyncResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if self.dry_run { "[DRY RUN] " } else { "" };
        write!(
            f,
            "{}Sync complete: {} new, {} updated, {} unchanged",
            prefix, self.new, self.updated, self.unchanged
        )
    }
}

impl fmt::Display for PushSyncResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if self.dry_run { "[DRY RUN] " } else { "" };
        write!(
            f,
            "{}Push complete: {} created, {} updated, {} deleted, {} conflicts",
            prefix, self.created, self.updated, self.deleted, self.conflicted
        )?;
        if self.failed > 0 {
            write!(f, ", {} failed", self.failed)?;
        }
        writeln!(f)?;
        for detail in &self.details {
            let prefix_char = match detail.action.as_str() {
                "created" => "+",
                "updated" => "~",
                "deleted" => "-",
                "conflict" => "!",
                "failed" => "x",
                _ => "?",
            };
            let label = match detail.action.as_str() {
                "created" => "Created",
                "updated" => "Updated",
                "deleted" => "Deleted",
                "conflict" => "Conflict",
                "failed" => "Failed",
                _ => &detail.action,
            };
            if let Some(ref err) = detail.error {
                writeln!(f, "  {} {}: {} ({})", prefix_char, label, detail.name, err)?;
            } else {
                writeln!(f, "  {} {}: {}", prefix_char, label, detail.name)?;
            }
        }
        Ok(())
    }
}

/// Run the `acrm sync setup` command: prompt for iCloud credentials and store them.
pub fn run_setup(_fmt: &OutputFormat) -> Result<()> {
    let apple_id: String = dialoguer::Input::new()
        .with_prompt("Apple ID (email)")
        .interact_text()
        .context("Failed to read Apple ID")?;

    let app_password: String = dialoguer::Password::new()
        .with_prompt("App-specific password")
        .interact()
        .context("Failed to read app password")?;

    config::store_credentials(&apple_id, &app_password)?;

    println!("iCloud credentials stored in macOS Keychain");
    println!("You can now run `acrm sync` to pull contacts");

    Ok(())
}

/// Run the `acrm sync push` command: push local changes to iCloud via CardDAV.
pub fn run_push(force: bool, dry_run: bool, filter: &SyncFilter, fmt: &OutputFormat) -> Result<()> {
    let (apple_id, app_password) = config::load_credentials()
        .context("Run `acrm sync setup` first to configure iCloud credentials")?;

    let crm_root = crate::store::find_crm_root()?;
    let credentials = SyncCredentials {
        apple_id,
        app_password,
    };
    let opts = SyncOpts { force, dry_run };

    println!("Discovering address book...");
    let result = ops::sync::sync_push(&crm_root, &credentials, filter, &opts)?;

    format::output(&result, fmt)
}

/// Run the `acrm sync` (pull) command: pull contacts from iCloud via CardDAV.
pub fn run_sync(force: bool, dry_run: bool, filter: &SyncFilter, fmt: &OutputFormat) -> Result<()> {
    let (apple_id, app_password) = config::load_credentials().context(
        "Run `acrm sync setup` first to configure iCloud credentials",
    )?;

    let crm_root = crate::store::find_crm_root()?;
    let credentials = SyncCredentials {
        apple_id,
        app_password,
    };
    let opts = SyncOpts { force, dry_run };

    println!("Discovering address book...");
    let result = ops::sync::sync_pull(&crm_root, &credentials, filter, &opts)?;

    format::output(&result, fmt)
}

/// Run bidirectional sync (pull then push).
pub fn run_bidirectional(
    force: bool,
    dry_run: bool,
    pull_filter: &SyncFilter,
    push_filter: &SyncFilter,
    fmt: &OutputFormat,
) -> Result<()> {
    let (apple_id, app_password) = config::load_credentials().context(
        "Run `acrm sync setup` first to configure iCloud credentials",
    )?;

    let crm_root = crate::store::find_crm_root()?;
    let credentials = SyncCredentials {
        apple_id,
        app_password,
    };
    let opts = SyncOpts { force, dry_run };

    println!("Starting bidirectional sync...\n");
    println!("--- Pull ---");
    let (pull_result, push_result) =
        ops::sync::sync_bidirectional(&crm_root, &credentials, pull_filter, push_filter, &opts)?;
    format::output(&pull_result, fmt)?;
    println!("\n--- Push ---");
    format::output(&push_result, fmt)?;
    println!("\nBidirectional sync complete.");
    Ok(())
}

/// Extract a UID from a vCard href path (delegates to ops).
pub fn extract_uid_from_href(href: &str) -> String {
    ops::sync::extract_uid_from_href(href)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::sync::{SyncResult, PushSyncResult};

    #[test]
    fn test_extract_uid_from_href() {
        assert_eq!(
            extract_uid_from_href("/123/carddavhome/card/ABC-DEF-123.vcf"),
            "ABC-DEF-123"
        );
        assert_eq!(extract_uid_from_href("contact1.vcf"), "contact1");
        assert_eq!(extract_uid_from_href("/path/to/file"), "file");
    }

    #[test]
    fn test_sync_result_display() {
        let result = SyncResult {
            new: 5,
            updated: 2,
            unchanged: 10,
            dry_run: false,
            contacts: vec![],
        };
        assert_eq!(
            format!("{}", result),
            "Sync complete: 5 new, 2 updated, 10 unchanged"
        );
    }

    #[test]
    fn test_sync_result_display_dry_run() {
        let result = SyncResult {
            new: 3,
            updated: 0,
            unchanged: 0,
            dry_run: true,
            contacts: vec![],
        };
        assert_eq!(
            format!("{}", result),
            "[DRY RUN] Sync complete: 3 new, 0 updated, 0 unchanged"
        );
    }

    #[test]
    fn test_push_sync_result_display() {
        let result = PushSyncResult {
            created: 2,
            updated: 1,
            deleted: 0,
            conflicted: 0,
            failed: 0,
            dry_run: false,
            details: vec![],
        };
        let display = format!("{}", result);
        assert!(display.contains("Push complete: 2 created, 1 updated, 0 deleted, 0 conflicts"));
    }
}
