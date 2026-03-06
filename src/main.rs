mod commands;
mod format;
mod frontmatter;
mod models;
mod store;
mod validation;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "acrm", about = "Agent-friendly personal CRM")]
struct Cli {
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
    /// Show contacts due for follow-up
    Due,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { name } => commands::add::run(&name),
        Commands::List { tag } => commands::list::run(tag.as_deref()),
        Commands::Search { query } => commands::search::run(&query),
        Commands::Show { name } => commands::show::run(&name),
        Commands::Log {
            name,
            interaction_type,
            summary,
            notes,
        } => commands::log::run(&name, &interaction_type, &summary, notes.as_deref()),
        Commands::Due => commands::due::run(),
    }
}
