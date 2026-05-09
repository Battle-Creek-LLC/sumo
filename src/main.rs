mod auth;
mod config;
mod search;
mod output;
mod time;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sumo",
    about = "Query Sumo Logic logs from the terminal",
    long_about = "A fast CLI for querying Sumo Logic logs.\n\n\
        Credentials are stored in a TOML config file (default ~/.config/sumo/config.toml). Run 'sumo auth login' to get started.\n\n\
        For AI agents: use '-o json -q' for machine-readable output without progress noise.\n\n\
        Examples:\n  \
        sumo search 'error' -f -24h\n  \
        sumo search 'error | count by _sourceCategory' -f -24h -o json -q\n  \
        sumo search '_sourceCategory=prod/app/* ERROR' -f -1h --raw -q | grep Traceback",
    after_help = "QUERY SYNTAX:\n  \
        Filtering:  _sourceCategory=prod/app/*  |  \"exact phrase\"  |  error OR warning  |  error NOT \"health check\"\n  \
        Aggregation (after |):  count, sum, avg, min, max, timeslice, top, sort, parse, where\n  \
        Example: 'error | timeslice 1h | count by _timeslice | sort by _count desc'\n\n\
        TIME FORMAT:\n  \
        Relative: -30s, -15m, -1h, -24h, -7d, -2w    Absolute: 2026-03-17T14:00:00Z\n\n\
        TIPS:\n  \
        - Single-quote queries to protect pipes and wildcards from shell expansion\n  \
        - Use -o json -q for programmatic access (no truncation, no progress output)\n  \
        - Start with aggregation queries to understand the landscape, then drill in"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage authentication credentials stored in the config file
    Auth {
        #[command(subcommand)]
        action: AuthCommands,
    },

    /// Run a log search query and return results
    #[command(
        long_about = "Run a Sumo Logic search query and return results.\n\n\
            Manages the full search job lifecycle: create, poll, fetch results, cleanup.\n\
            Automatically detects aggregation queries (with |) and fetches records instead of messages.\n\n\
            Output formats:\n  \
            text  — human-readable table (default, truncates long messages)\n  \
            json  — full JSON array, no truncation (best for programmatic use)\n  \
            csv   — standard CSV with headers\n  \
            --raw — one _raw message per line (best for piping to grep)"
    )]
    Search {
        /// Sumo Logic query string (single-quote in shell to protect pipes)
        #[arg(help_heading = "Query")]
        query: String,

        /// Start time: relative (-15m, -24h, -7d, -2w) or ISO 8601
        #[arg(short = 'f', long, default_value = "-15m", allow_hyphen_values = true, help_heading = "Time Range")]
        from: String,

        /// End time: relative, ISO 8601, or 'now'
        #[arg(short = 't', long, default_value = "now", allow_hyphen_values = true, help_heading = "Time Range")]
        to: String,

        /// Timezone for query (e.g., UTC, America/New_York)
        #[arg(short = 'z', long, default_value = "UTC", help_heading = "Time Range")]
        timezone: String,

        /// Max results to return [max: 10000]
        #[arg(short = 'l', long, default_value = "100", help_heading = "Output")]
        limit: u32,

        /// Starting offset for manual pagination
        #[arg(long, default_value = "0", help_heading = "Output")]
        offset: u32,

        /// Output format: text, json, csv
        #[arg(short = 'o', long, default_value = "text", help_heading = "Output")]
        output: String,

        /// Comma-separated fields to include (works with all output formats)
        #[arg(long, help_heading = "Output")]
        fields: Option<String>,

        /// Use receipt time instead of message time
        #[arg(long, default_value = "false", help_heading = "Query")]
        by_receipt_time: bool,

        /// Output only the raw log line per message (ideal for piping)
        #[arg(short = 'r', long, default_value = "false", help_heading = "Output")]
        raw: bool,

        /// Seconds between status polls [max: 20]
        #[arg(long, default_value = "2", hide = true)]
        poll_interval: u64,

        /// Suppress progress output on stderr
        #[arg(short = 'q', long, default_value = "false", help_heading = "Output")]
        quiet: bool,

        /// Use credentials from the named project
        #[arg(short = 'p', long, help_heading = "Auth")]
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
    /// Store credentials in the config file
    #[command(long_about = "Store Sumo Logic credentials in the config file (~/.config/sumo/config.toml by default).\n\n\
        Prompts interactively for deployment, access ID, and access key.\n\
        Pass all options (--endpoint, --access-id, --access-key) to skip prompts for scripted setup.\n\
        Override the file location with $SUMO_CONFIG.")]
    Login {
        /// Project name (for managing multiple accounts)
        #[arg(long, default_value = "default")]
        project: String,

        /// API endpoint URL (skips deployment prompt)
        #[arg(long)]
        endpoint: Option<String>,

        /// Sumo Logic Access ID (skips prompt)
        #[arg(long)]
        access_id: Option<String>,

        /// Sumo Logic Access Key (skips prompt)
        #[arg(long)]
        access_key: Option<String>,
    },

    /// Remove credentials from the config file
    Logout {
        /// Project name to remove
        #[arg(long, default_value = "default")]
        project: String,

        /// Remove credentials for all projects
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

    /// Show current authentication state (key is masked)
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
