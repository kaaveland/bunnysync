use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Action {
    /// Sync a local folder to a path within a bunny.net Storage Zone
    Sync {
        #[command(flatten)]
        args: SyncArgs,
    },
    /// Provide shell completions
    Completions {
        #[arg(short, long, default_value = "bash", value_parser=clap::builder::PossibleValuesParser::new(["bash", "zsh", "fish", "pwsh", "powershell"]))]
        shell: String,
    },
    /// Purge a URL from the bunny.net cache
    PurgeUrl {
        /// URL to purge, wildcard * is allowed at the end
        #[arg(name = "url")]
        url: String,
        /// API key for bunny CDN --  looked up in environment variable BUNNYSYNC_API_KEY if not present
        #[arg(short, long)]
        api_key: Option<String>,
    },
    /// Purge an entire pull zone from bunny.net cache
    PurgeZone {
        /// Numeric ID of pull zone to purge
        #[arg(name = "pullzone")]
        pullzone: u64,
        /// API key for bunny CDN --  looked up in environment variable BUNNYSYNC_API_KEY if not present
        #[arg(short, long)]
        api_key: Option<String>,
        /// Optional Cache Tag to target
        #[arg(short, long)]
        cache_tag: Option<String>,
    },
}

#[derive(Parser)]
#[command(name = "bunnysync")]
#[command(arg_required_else_help = true)]
#[command(about = "Sync your files to bunny cdn storage zone")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    long_about = "bunnysync is a tool for synchronizing files to bunny cdn storage zones

bunnysync can sync to subtrees of your storage zone, the entire storage zone, or selectively skip
parts of the tree. It can easily deploy a static site with a single command.

bunnysync refuses to sync if it looks like there's already an active sync job to the storage
zone. It places a lockfile into the storage zone during the sync to have rudimentary concurrency
control.

bunnysync aims to make the local_path and the path within the storage zone exactly equal. It will sync
HTML at the end, to ensure other assets like CSS are already updated by the time they sync."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Action,
}

#[derive(Parser)]
pub struct SyncArgs {
    /// Which bunny cdn endpoint to use
    #[arg(short, long, default_value = "storage.bunnycdn.com")]
    pub endpoint: String,
    /// Password for the storage zone - looked up in environment variable BUNNYSYNC_KEY if not present
    #[arg(short, long)]
    pub access_key: Option<String>,
    /// Local directory to put in the storage zone
    #[arg(name = "local_path", required = true, num_args = 1)]
    pub local_path: String,
    /// Which storage zone to sync to
    #[arg(name = "storage_zone", required = true, num_args = 1)]
    pub storage_zone: String,
    /// Path inside the storage zone to sync to, path to a directory
    #[arg(short, long, default_value = "/")]
    pub path: String,
    /// Don't sync, just show what would change
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Force a sync despite a hanging lock file
    #[arg(short, long, default_value_t = false)]
    pub force: bool,
    /// Filename to use for the lockfile. bunnysync will not sync if this file exists in the destination.
    #[arg(long, default_value = ".bunnysync.lock")]
    pub lockfile: String,
    /// Do not delete anything in the storage zone paths that start with this prefix (can pass multiple times)
    #[arg(short, long)]
    pub ignore: Vec<String>,
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
    /// Number of threads to use when calling bunny.net API (default to number of cpus)
    #[arg(short, long)]
    pub concurrency: Option<usize>,
}
