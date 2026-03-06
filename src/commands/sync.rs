use std::fmt;

use anyhow::{Context, Result};
use serde::Serialize;
use url::Url;

use crate::format::{self, OutputFormat};
use crate::frontmatter;
use crate::models::ContactFile;
use crate::store;
use crate::sync::{carddav::CardDavClient, config, dedup, vcard_map};

#[derive(Serialize)]
pub struct SyncResult {
    pub new: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub dry_run: bool,
    pub contacts: Vec<SyncedContact>,
}

#[derive(Serialize)]
pub struct SyncedContact {
    pub name: String,
    pub action: String, // "new", "updated", "unchanged"
}

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

/// Run the `acrm sync` command: pull contacts from iCloud via CardDAV.
pub fn run_sync(force: bool, dry_run: bool, fmt: &OutputFormat) -> Result<()> {
    // Load credentials
    let (apple_id, app_password) = config::load_credentials().context(
        "Run `acrm sync setup` first to configure iCloud credentials",
    )?;

    // Create CardDAV client
    let client = CardDavClient::new(&apple_id, &app_password)?;

    // Discover address book
    println!("Discovering address book...");
    let addressbook_url = client.discover_address_book()?;

    // Fetch vCard list
    let entries = client.fetch_vcard_list(&addressbook_url)?;
    println!("Found {} contacts on iCloud", entries.len());

    // Load existing local contacts
    let crm_root = store::find_crm_root()?;
    let existing_contacts = store::load_all_contacts(&crm_root)?;

    let mut new_count: usize = 0;
    let mut updated_count: usize = 0;
    let mut unchanged_count: usize = 0;
    let mut synced: Vec<SyncedContact> = Vec::new();

    for entry in &entries {
        // Build the full vCard URL from the href
        let vcard_url = resolve_vcard_url(&addressbook_url, &entry.href)?;

        // Fetch the vCard content
        let vcard_text = match client.fetch_vcard(&vcard_url) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Warning: failed to fetch {}: {}", entry.href, e);
                continue;
            }
        };

        // Extract UID from the href (last path segment without .vcf)
        let uid = extract_uid_from_href(&entry.href);

        // Map vCard to Contact
        let mapped = match vcard_map::map_vcard_to_contact(&vcard_text, &uid, &entry.etag) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Warning: failed to parse vCard {}: {}", entry.href, e);
                continue;
            }
        };

        let contact_name = mapped.contact.name.clone();

        // Check for existing contact by source_id
        if let Some(existing) = dedup::find_existing_by_source_id(&existing_contacts, &uid) {
            if !force && !dedup::should_update(existing, &entry.etag) {
                // Unchanged
                unchanged_count += 1;
                synced.push(SyncedContact {
                    name: contact_name,
                    action: "unchanged".to_string(),
                });
                continue;
            }

            // Update existing contact
            if dry_run {
                println!("[DRY RUN] Would update: {}", contact_name);
            } else {
                update_existing_contact(existing, &mapped, &entry.etag)?;
            }
            updated_count += 1;
            synced.push(SyncedContact {
                name: contact_name,
                action: "updated".to_string(),
            });
        } else {
            // New contact
            if dry_run {
                println!("[DRY RUN] Would create: {}", contact_name);
            } else {
                create_new_contact(&crm_root, &mapped, &uid, &entry.etag)?;
            }
            new_count += 1;
            synced.push(SyncedContact {
                name: contact_name,
                action: "new".to_string(),
            });
        }
    }

    let result = SyncResult {
        new: new_count,
        updated: updated_count,
        unchanged: unchanged_count,
        dry_run,
        contacts: synced,
    };

    format::output(&result, fmt)
}

/// Create a new contact file from a mapped vCard.
fn create_new_contact(
    crm_root: &std::path::Path,
    mapped: &vcard_map::MappedContact,
    uid: &str,
    etag: &str,
) -> Result<()> {
    // Generate raw frontmatter from template
    let mut fm = store::generate_raw_frontmatter(&mapped.contact, crm_root)?;

    // Apply fields that generate_raw_frontmatter doesn't handle
    if !mapped.contact.email.is_empty() {
        fm = frontmatter::update_array_field(&fm, "email", &mapped.contact.email);
    }
    if !mapped.contact.phone.is_empty() {
        fm = frontmatter::update_array_field(&fm, "phone", &mapped.contact.phone);
    }
    if !mapped.contact.tags.is_empty() {
        fm = frontmatter::update_array_field(&fm, "tags", &mapped.contact.tags);
    }

    fm = frontmatter::update_field(&fm, "source", "icloud");
    fm = frontmatter::update_field(&fm, "source_id", &format!("\"{}\"", uid));
    fm = frontmatter::update_field(&fm, "etag", &format!("\"{}\"", etag));

    if !mapped.contact.website.is_empty() {
        fm = frontmatter::update_field(
            &fm,
            "website",
            &format!("\"{}\"", mapped.contact.website),
        );
    }
    if let Some(bday) = mapped.contact.birthday {
        fm = frontmatter::update_field(&fm, "birthday", &bday.to_string());
    }

    // Ensure trailing newline
    if !fm.ends_with('\n') {
        fm.push('\n');
    }

    // Build the body
    let mut body = String::from("## Notes\n\n");
    if !mapped.notes.is_empty() {
        body.push_str(&mapped.notes);
        body.push_str("\n\n");
    }
    body.push_str("## Interaction Log\n");

    let cf = ContactFile {
        contact: mapped.contact.clone(),
        body,
        path: crm_root.join("contacts"),
        raw_frontmatter: fm,
    };

    store::write_contact(crm_root, &cf)?;
    Ok(())
}

/// Update an existing contact's frontmatter fields from a new vCard version.
fn update_existing_contact(
    existing: &ContactFile,
    mapped: &vcard_map::MappedContact,
    new_etag: &str,
) -> Result<()> {
    let mut fm = existing.raw_frontmatter.clone();

    // Update name
    fm = frontmatter::update_field(&fm, "name", &format!("\"{}\"", mapped.contact.name));

    // Update contact info
    fm = frontmatter::update_array_field(&fm, "email", &mapped.contact.email);
    fm = frontmatter::update_array_field(&fm, "phone", &mapped.contact.phone);

    // Update professional fields
    if !mapped.contact.company.is_empty() {
        fm = frontmatter::update_field(
            &fm,
            "company",
            &format!("\"{}\"", mapped.contact.company),
        );
    }
    if !mapped.contact.role.is_empty() {
        fm = frontmatter::update_field(&fm, "role", &format!("\"{}\"", mapped.contact.role));
    }

    // Update website
    if !mapped.contact.website.is_empty() {
        fm = frontmatter::update_field(
            &fm,
            "website",
            &format!("\"{}\"", mapped.contact.website),
        );
    }

    // Update birthday
    if let Some(bday) = mapped.contact.birthday {
        fm = frontmatter::update_field(&fm, "birthday", &bday.to_string());
    }

    // Update etag
    fm = frontmatter::update_field(&fm, "etag", &format!("\"{}\"", new_etag));

    // Ensure trailing newline
    if !fm.ends_with('\n') {
        fm.push('\n');
    }

    // Write directly to existing path
    let content = format!("---\n{}---\n\n{}", fm, existing.body);
    std::fs::write(&existing.path, content)
        .with_context(|| format!("Failed to write {}", existing.path.display()))?;

    Ok(())
}

/// Resolve a vCard href to a full URL relative to the addressbook URL.
fn resolve_vcard_url(addressbook_url: &Url, href: &str) -> Result<Url> {
    // If href is absolute (starts with /), resolve against the base URL origin
    if href.starts_with('/') {
        let base = format!(
            "{}://{}",
            addressbook_url.scheme(),
            addressbook_url
                .host_str()
                .unwrap_or("contacts.icloud.com")
        );
        Url::parse(&format!("{}{}", base, href)).context("Failed to resolve vCard URL")
    } else {
        addressbook_url
            .join(href)
            .context("Failed to join vCard URL")
    }
}

/// Extract a UID from a vCard href path.
/// e.g., "/123/carddavhome/card/ABC-DEF-123.vcf" -> "ABC-DEF-123"
fn extract_uid_from_href(href: &str) -> String {
    href.rsplit('/')
        .next()
        .unwrap_or(href)
        .trim_end_matches(".vcf")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_resolve_vcard_url_absolute() {
        let base = Url::parse("https://contacts.icloud.com/123/carddavhome/card/").unwrap();
        let result = resolve_vcard_url(&base, "/123/carddavhome/card/abc.vcf").unwrap();
        assert_eq!(
            result.as_str(),
            "https://contacts.icloud.com/123/carddavhome/card/abc.vcf"
        );
    }

    #[test]
    fn test_resolve_vcard_url_relative() {
        let base = Url::parse("https://contacts.icloud.com/123/carddavhome/card/").unwrap();
        let result = resolve_vcard_url(&base, "abc.vcf").unwrap();
        assert_eq!(
            result.as_str(),
            "https://contacts.icloud.com/123/carddavhome/card/abc.vcf"
        );
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
}
