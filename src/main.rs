mod auth;
mod keychain;
mod search;
mod output;
mod time;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sumo", about = "A fast, minimal CLI for querying Sumo Logic logs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage authentication credentials
    Auth {
        #[command(subcommand)]
        action: AuthCommands,
    },
    /// Run a log search query
    Search {
        /// Sumo Logic search query string
        query: String,

        /// Start time (ISO 8601, relative like -24h, -7d, or now)
        #[arg(short = 'f', long, default_value = "-15m", allow_hyphen_values = true)]
        from: String,

        /// End time (same formats as --from)
        #[arg(short = 't', long, default_value = "now", allow_hyphen_values = true)]
        to: String,

        /// Timezone for query
        #[arg(short = 'z', long, default_value = "UTC")]
        timezone: String,

        /// Max number of messages to return (max 10000)
        #[arg(short = 'l', long, default_value = "100")]
        limit: u32,

        /// Starting offset for pagination
        #[arg(long, default_value = "0")]
        offset: u32,

        /// Output format: text, json, csv
        #[arg(short = 'o', long, default_value = "text")]
        output: String,

        /// Comma-separated list of fields to include
        #[arg(long)]
        fields: Option<String>,

        /// Use receipt time instead of message time
        #[arg(long, default_value = "false")]
        by_receipt_time: bool,

        /// Output raw _raw field only (one message per line)
        #[arg(short = 'r', long, default_value = "false")]
        raw: bool,

        /// Seconds between status polls
        #[arg(long, default_value = "2")]
        poll_interval: u64,

        /// Suppress progress output
        #[arg(short = 'q', long, default_value = "false")]
        quiet: bool,

        /// Use credentials from the named project
        #[arg(short = 'p', long)]
        project: Option<String>,
    },
    /// Check the status of a running search job
    Status {
        /// Search job ID
        job_id: String,

        /// Use credentials from the named project
        #[arg(short = 'p', long)]
        project: Option<String>,
    },
    /// Cancel a running search job
    Cancel {
        /// Search job ID
        job_id: String,

        /// Use credentials from the named project
        #[arg(short = 'p', long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Store credentials in the macOS Keychain
    Login {
        /// Project name
        #[arg(long, default_value = "default")]
        project: String,

        /// API endpoint URL
        #[arg(long)]
        endpoint: Option<String>,

        /// Access ID
        #[arg(long)]
        access_id: Option<String>,

        /// Access key
        #[arg(long)]
        access_key: Option<String>,
    },
    /// Remove credentials from the Keychain
    Logout {
        /// Project name
        #[arg(long, default_value = "default")]
        project: String,

        /// Remove all projects
        #[arg(long)]
        all: bool,
    },
    /// Switch the active project
    Use {
        /// Project name to activate
        name: String,
    },
    /// List all configured projects
    List,
    /// Show current authentication state
    Status,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Auth { action } => match action {
            AuthCommands::Login { project, endpoint, access_id, access_key } => {
                auth::login(&project, endpoint, access_id, access_key)
            }
            AuthCommands::Logout { project, all } => {
                auth::logout(&project, all)
            }
            AuthCommands::Use { name } => {
                auth::use_project(&name)
            }
            AuthCommands::List => {
                auth::list()
            }
            AuthCommands::Status => {
                auth::status()
            }
        },
        Commands::Search {
            query, from, to, timezone, limit, offset, output, fields,
            by_receipt_time, raw, poll_interval, quiet, project,
        } => {
            search::run(search::SearchArgs {
                query, from, to, timezone, limit, offset, output, fields,
                by_receipt_time, raw, poll_interval, quiet, project,
            })
        }
        Commands::Status { job_id, project } => {
            search::job_status(&job_id, project.as_deref())
        }
        Commands::Cancel { job_id, project } => {
            search::cancel_job(&job_id, project.as_deref())
        }
    };

    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
