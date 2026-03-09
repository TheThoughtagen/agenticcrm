mod commands;
mod format;
mod frontmatter;
mod mcp;
mod models;
mod ops;
pub mod query;
mod store;
mod sync;
mod tui;
mod validation;

use clap::{Parser, Subcommand};
use format::OutputFormat;
use sync::config::FilterConfig;
use sync::filter::SyncFilter;

#[derive(Parser)]
#[command(name = "acrm", about = "Agent-friendly personal CRM")]
struct Cli {
    /// Output format (human or json)
    #[arg(short, long, global = true, default_value = "human")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new contact
    Add {
        /// Full name, e.g. "Jane Smith"
        name: String,
    },
    /// List all contacts
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },
    /// Search contacts by name, company, tag, or notes
    Search {
        /// Search query
        query: String,
    },
    /// Show full details for a contact
    Show {
        /// Name (or partial match)
        name: String,
    },
    /// Log an interaction with a contact
    Log {
        /// Contact name (or partial match)
        name: String,
        /// Interaction type (coffee, call, email, meeting, etc.)
        #[arg(short = 't', long = "type")]
        interaction_type: String,
        /// Short summary
        summary: String,
        /// Optional detailed notes
        #[arg(short, long)]
        notes: Option<String>,
    },
    /// Edit a contact's fields
    Edit {
        /// Contact name (or partial match)
        name: String,
        /// Set a field value (key=value), repeatable
        #[arg(short, long = "set", num_args = 1)]
        sets: Vec<String>,
    },
    /// Delete a contact
    Delete {
        /// Contact name (or partial match)
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
    /// Archive a contact (sets status to archived, moves to archive/)
    Archive {
        /// Contact name (or partial match)
        name: String,
    },
    /// Unarchive a contact (sets status to active, moves back to contacts/)
    Unarchive {
        /// Contact name (or partial match)
        name: String,
    },
    /// Bulk query and operate on contacts
    Bulk {
        /// Query predicate (e.g. 'status=dormant', 'tags~friend')
        query: String,
        /// Set field values (repeatable, e.g. --set status=active)
        #[arg(long = "set", num_args = 1)]
        sets: Vec<String>,
        /// Delete matched contacts
        #[arg(long, conflicts_with = "archive")]
        delete: bool,
        /// Archive matched contacts
        #[arg(long, conflicts_with = "delete")]
        archive: bool,
        /// Add tag to matched contacts (repeatable)
        #[arg(long = "add-tag", num_args = 1)]
        add_tags: Vec<String>,
        /// Remove tag from matched contacts (repeatable)
        #[arg(long = "remove-tag", num_args = 1)]
        remove_tags: Vec<String>,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
        /// Preview changes without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Bulk update contacts from JSON stdin
    BulkUpdate {
        /// Read contact list from stdin (JSON array)
        #[arg(long)]
        stdin: bool,
        /// Set field values (repeatable)
        #[arg(long = "set", num_args = 1)]
        sets: Vec<String>,
        /// Delete matched contacts
        #[arg(long, conflicts_with = "archive")]
        delete: bool,
        /// Archive matched contacts
        #[arg(long, conflicts_with = "delete")]
        archive: bool,
        /// Add tag (repeatable)
        #[arg(long = "add-tag", num_args = 1)]
        add_tags: Vec<String>,
        /// Remove tag (repeatable)
        #[arg(long = "remove-tag", num_args = 1)]
        remove_tags: Vec<String>,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Start MCP server for AI agent integration
    Serve {
        /// Use HTTP transport instead of stdio
        #[arg(long)]
        http: bool,
        /// HTTP port (default: 3000)
        #[arg(long, default_value = "3000")]
        port: u16,
        /// Allow sync operations (disabled by default for safety)
        #[arg(long)]
        allow_sync: bool,
    },
    /// Show contacts due for follow-up
    Due,
    /// Launch interactive TUI
    Tui,
    /// Sync contacts with iCloud (pull then push)
    Sync {
        #[command(subcommand)]
        action: Option<SyncAction>,
        /// Force re-download all contacts (ignore ETags)
        #[arg(long)]
        force: bool,
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,
        /// Filter by tag (repeatable, overrides config)
        #[arg(long)]
        tag: Vec<String>,
        /// Filter by status (repeatable, overrides config)
        #[arg(long)]
        status: Vec<String>,
    },
}

#[derive(Subcommand)]
enum SyncAction {
    /// Set up iCloud credentials
    Setup,
    /// Pull contacts from iCloud
    Pull {
        /// Force re-download all contacts (ignore ETags)
        #[arg(long)]
        force: bool,
        /// Show what would change without writing
        #[arg(long)]
        dry_run: bool,
        /// Filter by tag (repeatable, overrides config)
        #[arg(long)]
        tag: Vec<String>,
        /// Filter by status (repeatable, overrides config)
        #[arg(long)]
        status: Vec<String>,
    },
    /// Push local changes to iCloud
    Push {
        /// Force push even when server has newer changes
        #[arg(long)]
        force: bool,
        /// Show what would change without pushing
        #[arg(long)]
        dry_run: bool,
        /// Filter by tag (repeatable, overrides config)
        #[arg(long)]
        tag: Vec<String>,
        /// Filter by status (repeatable, overrides config)
        #[arg(long)]
        status: Vec<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let fmt = &cli.format;

    match cli.command {
        Commands::Add { name } => commands::add::run(&name, fmt),
        Commands::List { tag } => commands::list::run(tag.as_deref(), fmt),
        Commands::Search { query } => commands::search::run(&query, fmt),
        Commands::Show { name } => commands::show::run(&name, fmt),
        Commands::Log {
            name,
            interaction_type,
            summary,
            notes,
        } => commands::log::run(&name, &interaction_type, &summary, notes.as_deref(), fmt),
        Commands::Edit { name, sets } => commands::edit::run(&name, &sets, fmt),
        Commands::Delete { name, yes } => commands::delete::run(&name, yes, fmt),
        Commands::Archive { name } => commands::archive::run_archive(&name, fmt),
        Commands::Unarchive { name } => commands::archive::run_unarchive(&name, fmt),
        Commands::Bulk {
            query,
            sets,
            delete,
            archive,
            add_tags,
            remove_tags,
            yes,
            dry_run,
        } => commands::bulk::run_bulk(&query, &sets, delete, archive, &add_tags, &remove_tags, yes, dry_run, fmt),
        Commands::BulkUpdate {
            stdin,
            sets,
            delete,
            archive,
            add_tags,
            remove_tags,
            yes,
            dry_run,
        } => commands::bulk::run_bulk_update(stdin, &sets, delete, archive, &add_tags, &remove_tags, yes, dry_run, fmt),
        Commands::Serve {
            http,
            port,
            allow_sync,
        } => {
            let root = store::find_crm_root()?;
            let server = mcp::CrmServer::new(root, allow_sync);
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                if http {
                    mcp::serve_http(server, port).await
                } else {
                    mcp::serve_stdio(server).await
                }
            })
        }
        Commands::Due => commands::due::run(fmt),
        Commands::Tui => tui::run(),
        Commands::Sync {
            action,
            force,
            dry_run,
            tag,
            status,
        } => {
            // Load config for filter sections; default to empty if not configured yet
            let (push_fc, pull_fc) = match sync::config::load_sync_config() {
                Ok(cfg) => (cfg.push_filters, cfg.pull_filters),
                Err(_) => (FilterConfig::default(), FilterConfig::default()),
            };

            match action {
                Some(SyncAction::Setup) => commands::sync::run_setup(fmt),
                Some(SyncAction::Push {
                    force: f,
                    dry_run: d,
                    tag: sub_tag,
                    status: sub_status,
                }) => {
                    let merged_tags = merge_vecs(tag, sub_tag);
                    let merged_statuses = merge_vecs(status, sub_status);
                    let filter = SyncFilter::from_config_and_cli(
                        push_fc.tags,
                        push_fc.statuses,
                        merged_tags,
                        merged_statuses,
                    );
                    commands::sync::run_push(force || f, dry_run || d, &filter, fmt)
                }
                Some(SyncAction::Pull {
                    force: f,
                    dry_run: d,
                    tag: sub_tag,
                    status: sub_status,
                }) => {
                    let merged_tags = merge_vecs(tag, sub_tag);
                    let merged_statuses = merge_vecs(status, sub_status);
                    let filter = SyncFilter::from_config_and_cli(
                        pull_fc.tags,
                        pull_fc.statuses,
                        merged_tags,
                        merged_statuses,
                    );
                    commands::sync::run_sync(force || f, dry_run || d, &filter, fmt)
                }
                None => {
                    // Build separate filters for pull and push
                    let pull_filter = SyncFilter::from_config_and_cli(
                        pull_fc.tags,
                        pull_fc.statuses,
                        tag.clone(),
                        status.clone(),
                    );
                    let push_filter = SyncFilter::from_config_and_cli(
                        push_fc.tags,
                        push_fc.statuses,
                        tag,
                        status,
                    );

                    commands::sync::run_bidirectional(force, dry_run, &pull_filter, &push_filter, fmt)
                }
            }
        }
    }
}

/// Merge two Vec<String> by concatenating and deduplicating.
fn merge_vecs(mut a: Vec<String>, b: Vec<String>) -> Vec<String> {
    for item in b {
        if !a.contains(&item) {
            a.push(item);
        }
    }
    a
}
