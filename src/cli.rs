use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "sage-restic-manager",
    version = env!("CARGO_PKG_VERSION"),
    about = "Production-grade backup management for Restic on self-hosted servers",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(long, env = "SRM_LOG", default_value = "info")]
    pub log_level: String,

    #[arg(long, help = "Path to config directory (overrides default)")]
    pub config_dir: Option<std::path::PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Run a backup of all selected sources (non-interactive)")]
    Backup {
        #[arg(long, help = "Run without user interaction (for systemd)")]
        non_interactive: bool,
    },

    #[command(about = "Check repository integrity")]
    Check {
        #[arg(long, help = "Also read and verify all data")]
        read_data: bool,
    },

    #[command(about = "List snapshots in the repository")]
    Snapshots,

    #[command(about = "Apply retention policy and prune unreferenced data")]
    Forget {
        #[arg(long, help = "Preview changes without executing")]
        dry_run: bool,
        #[arg(long, help = "Also run prune after forget")]
        prune: bool,
    },

    #[command(about = "Discover Docker volumes and update sources")]
    Discover,

    #[command(about = "Install systemd timer and service units")]
    InstallSchedule,

    #[command(about = "Show current configuration")]
    Config,

    #[command(about = "Check for and apply updates")]
    SelfUpdate {
        #[arg(long, help = "Only check, do not download")]
        check: bool,
    },

    #[command(about = "Export log file to stdout")]
    Logs {
        #[arg(long, default_value = "100")]
        lines: usize,
    },
}
