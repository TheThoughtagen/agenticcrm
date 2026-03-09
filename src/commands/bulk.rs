use std::fmt;
use std::io::Read;

use colored::Colorize;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};

use crate::format::{self, OutputFormat};
use crate::models::ContactFile;
use crate::ops;
use crate::ops::contact::{BulkChange, BulkResult, SearchMatch};
use crate::query::Query;
use crate::store;

// ── Display implementations ─────────────────────────────────────────────────

impl fmt::Display for BulkResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if self.dry_run { "[DRY RUN] " } else { "" };
        if self.changes.is_empty() {
            write!(f, "{}No changes applied.", prefix)
        } else {
            let action = &self.changes[0].action;
            let verb = if action.contains("deleted") {
                "Deleted"
            } else if action.contains("archived") {
                "Archived"
            } else if action.contains("tag") {
                "Tagged"
            } else {
                "Updated"
            };
            if self.dry_run {
                write!(
                    f,
                    "[DRY RUN] Would {} {} contact(s)",
                    verb.to_lowercase(),
                    self.affected
                )?;
            } else {
                write!(f, "{} {} contact(s)", verb, self.affected)?;
            }
            for change in &self.changes {
                write!(f, "\n  {} -- {}", change.name.bold(), change.action.dimmed())?;
            }
            Ok(())
        }
    }
}

impl fmt::Display for BulkChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -- {}", self.name.bold(), self.action.dimmed())
    }
}

// ── Stdin contact for pipe input ────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
struct StdinContact {
    name: String,
    #[serde(default)]
    company: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

// ── Helper: preview matched contacts ────────────────────────────────────────

fn print_preview(matched: &[ContactFile], action_desc: &str, dry_run: bool) {
    let prefix = if dry_run { "[DRY RUN] " } else { "" };
    println!(
        "{}Will {} {} contact(s):",
        prefix,
        action_desc,
        matched.len()
    );

    let display_count = matched.len().min(20);
    for cf in &matched[..display_count] {
        let company = if cf.contact.company.is_empty() {
            String::new()
        } else {
            format!(" @ {}", cf.contact.company)
        };
        println!("  {} {}{}", "-".dimmed(), cf.contact.name.bold(), company.dimmed());
    }

    if matched.len() > 20 {
        println!("  ...and {} more", matched.len() - 20);
    }
}

/// Describe what action flags request
fn describe_action(
    sets: &[String],
    delete: bool,
    archive: bool,
    add_tags: &[String],
    remove_tags: &[String],
) -> String {
    let mut parts = Vec::new();
    if delete {
        parts.push("delete".to_string());
    }
    if archive {
        parts.push("archive".to_string());
    }
    if !sets.is_empty() {
        parts.push(format!("update ({})", sets.join(", ")));
    }
    if !add_tags.is_empty() {
        parts.push(format!("add tags [{}]", add_tags.join(", ")));
    }
    if !remove_tags.is_empty() {
        parts.push(format!("remove tags [{}]", remove_tags.join(", ")));
    }
    parts.join(" + ")
}

fn has_action(
    sets: &[String],
    delete: bool,
    archive: bool,
    add_tags: &[String],
    remove_tags: &[String],
) -> bool {
    delete || archive || !sets.is_empty() || !add_tags.is_empty() || !remove_tags.is_empty()
}

// ── Shared action execution ─────────────────────────────────────────────────

fn execute_actions(
    root: &std::path::Path,
    matched: &[ContactFile],
    sets: &[String],
    delete: bool,
    archive: bool,
    add_tags: &[String],
    remove_tags: &[String],
    dry_run: bool,
    fmt: &OutputFormat,
) -> anyhow::Result<()> {
    // Execute operations in order: delete > archive > update > tag
    if delete {
        let result = ops::contact::bulk_delete(root, matched, dry_run)?;
        format::output(&result, fmt)?;
    } else if archive {
        let result = ops::contact::bulk_archive(root, matched, dry_run)?;
        format::output(&result, fmt)?;
    } else {
        // Can combine --set with --add-tag/--remove-tag
        if !sets.is_empty() {
            let result = ops::contact::bulk_update(root, matched, sets, dry_run)?;
            format::output(&result, fmt)?;
        }
        if !add_tags.is_empty() || !remove_tags.is_empty() {
            let result = ops::contact::bulk_tag(root, matched, add_tags, remove_tags, dry_run)?;
            format::output(&result, fmt)?;
        }
    }

    Ok(())
}

// ── Public command handlers ─────────────────────────────────────────────────

pub fn run_bulk(
    query: &str,
    sets: &[String],
    delete: bool,
    archive: bool,
    add_tags: &[String],
    remove_tags: &[String],
    yes: bool,
    dry_run: bool,
    fmt: &OutputFormat,
) -> anyhow::Result<()> {
    let root = store::find_crm_root()?;
    let q = Query::parse(query)?;
    let matched = ops::contact::query(&root, &q)?;

    if matched.is_empty() {
        match fmt {
            OutputFormat::Human => println!("No contacts match query '{}'.", query),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    // Query-only mode: just list matched contacts
    if !has_action(sets, delete, archive, add_tags, remove_tags) {
        let results: Vec<SearchMatch> = matched
            .iter()
            .map(|cf| SearchMatch {
                name: cf.contact.name.clone(),
                company: cf.contact.company.clone(),
                path: cf.path.display().to_string(),
            })
            .collect();

        return format::output_list(&results, fmt, "contact(s) matched");
    }

    // Action mode: preview, confirm, execute
    let action_desc = describe_action(sets, delete, archive, add_tags, remove_tags);

    if matches!(fmt, OutputFormat::Human) {
        print_preview(&matched, &action_desc, dry_run);
    }

    // Confirm unless --yes or --dry-run
    if !yes && !dry_run {
        let confirmed = Confirm::new()
            .with_prompt(format!("Apply to {} contact(s)?", matched.len()))
            .default(false)
            .interact()?;
        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    execute_actions(root.as_path(), &matched, sets, delete, archive, add_tags, remove_tags, dry_run, fmt)
}

pub fn run_bulk_update(
    stdin: bool,
    sets: &[String],
    delete: bool,
    archive: bool,
    add_tags: &[String],
    remove_tags: &[String],
    yes: bool,
    dry_run: bool,
    fmt: &OutputFormat,
) -> anyhow::Result<()> {
    if !stdin {
        anyhow::bail!("--stdin flag is required. Usage: acrm search ... --format json | acrm bulk-update --stdin --set ...");
    }

    // Check if stdin is a terminal (no pipe)
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        anyhow::bail!(
            "No input piped. Usage: acrm search ... --format json | acrm bulk-update --stdin --set ..."
        );
    }

    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let stdin_contacts: Vec<StdinContact> = serde_json::from_str(&input)
        .map_err(|e| anyhow::anyhow!("Invalid JSON from stdin: {}", e))?;

    if stdin_contacts.is_empty() {
        match fmt {
            OutputFormat::Human => println!("No contacts in stdin input."),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    let root = store::find_crm_root()?;

    // Resolve stdin contacts to ContactFiles
    let mut matched: Vec<ContactFile> = Vec::new();

    // Check if any contacts have paths -- if so, load directly
    let has_paths = stdin_contacts.iter().any(|sc| sc.path.is_some());

    if has_paths {
        for sc in &stdin_contacts {
            if let Some(path) = &sc.path {
                let path = std::path::Path::new(path);
                if path.exists() {
                    match store::parse_contact_file(path) {
                        Ok(cf) => matched.push(cf),
                        Err(e) => eprintln!("Warning: could not load {}: {}", path.display(), e),
                    }
                } else {
                    eprintln!("Warning: file not found: {}", path.display());
                }
            }
        }
    } else {
        // Fuzzy match by name -- load all contacts once, collect paths, then reload
        let all_contacts = store::load_all_contacts(&root)
            .map_err(|e| anyhow::anyhow!("Failed to load contacts: {}", e))?;

        let mut paths_to_load: Vec<std::path::PathBuf> = Vec::new();

        for sc in &stdin_contacts {
            let name_lower = sc.name.to_lowercase();
            let found: Vec<_> = all_contacts
                .iter()
                .filter(|cf| cf.contact.name.to_lowercase() == name_lower)
                .collect();

            if found.len() == 1 {
                paths_to_load.push(found[0].path.clone());
            } else if found.is_empty() {
                eprintln!("Warning: no contact found for '{}'", sc.name);
            } else {
                eprintln!(
                    "Warning: multiple contacts match '{}', skipping",
                    sc.name
                );
            }
        }

        // Drop all_contacts, reload matched by path
        drop(all_contacts);
        for path in &paths_to_load {
            match store::parse_contact_file(path) {
                Ok(cf) => matched.push(cf),
                Err(e) => eprintln!("Warning: could not load {}: {}", path.display(), e),
            }
        }
    }

    if matched.is_empty() {
        match fmt {
            OutputFormat::Human => println!("No contacts resolved from stdin input."),
            OutputFormat::Json => println!("[]"),
        }
        return Ok(());
    }

    if !has_action(sets, delete, archive, add_tags, remove_tags) {
        // No action flags -- just show resolved contacts
        let results: Vec<SearchMatch> = matched
            .iter()
            .map(|cf| SearchMatch {
                name: cf.contact.name.clone(),
                company: cf.contact.company.clone(),
                path: cf.path.display().to_string(),
            })
            .collect();

        return format::output_list(&results, fmt, "contact(s) resolved");
    }

    let action_desc = describe_action(sets, delete, archive, add_tags, remove_tags);

    if matches!(fmt, OutputFormat::Human) {
        print_preview(&matched, &action_desc, dry_run);
    }

    if !yes && !dry_run {
        let confirmed = Confirm::new()
            .with_prompt(format!("Apply to {} contact(s)?", matched.len()))
            .default(false)
            .interact()?;
        if !confirmed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    execute_actions(root.as_path(), &matched, sets, delete, archive, add_tags, remove_tags, dry_run, fmt)
}
