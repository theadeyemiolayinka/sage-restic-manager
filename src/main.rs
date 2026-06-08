mod backup;
mod cli;
mod config;
mod discovery;
mod error;
mod restic;
mod scheduler;
mod tui;
mod updater;

use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use cli::{Cli, Commands};
use config::{AppConfig, SchedulesConfig, SourcesConfig};
use error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = std::env::var("SRM_LOG").unwrap_or_else(|_| cli.log_level.clone());

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&log_level)),
        )
        .with_target(false)
        .compact()
        .init();

    let config = AppConfig::load()?;
    let sources_config = SourcesConfig::load()?;
    let schedules_config = SchedulesConfig::load()?;

    match cli.command {
        None => {
            let app = tui::app::App::new(config, sources_config, schedules_config);
            tui::run(app).await?;
        }

        Some(Commands::Backup { non_interactive }) => {
            info!("Running backup (non-interactive: {})", non_interactive);
            let mut sources = sources_config;
            backup::run_backup(&config, &mut sources).await?;
        }

        Some(Commands::Check { read_data }) => {
            let client = restic::ResticClient::new(&config);
            let result = client.check(read_data).await?;
            if result.ok {
                println!("Repository check passed.");
            } else {
                eprintln!("Repository check FAILED:");
            }
            print!("{}", result.output);
            if !result.ok {
                std::process::exit(1);
            }
        }

        Some(Commands::Snapshots) => {
            let client = restic::ResticClient::new(&config);
            let snaps = client.snapshots().await?;
            println!("{} snapshots:", snaps.len());
            for snap in &snaps {
                println!("  {}  {}  {}  {}", snap.short_id, snap.time.format("%Y-%m-%d %H:%M"), snap.hostname, snap.display_paths());
            }
        }

        Some(Commands::Forget { dry_run, prune }) => {
            let client = restic::ResticClient::new(&config);
            let results = client.forget(&config.retention, dry_run).await?;
            for group in &results {
                println!("Keep: {} snapshots, Remove: {} snapshots", group.keep.len(), group.remove.len());
            }
            if prune && !dry_run {
                println!("Running prune...");
                let output = client.prune().await?;
                print!("{}", output);
            }
        }

        Some(Commands::Discover) => {
            use discovery::{DockerDiscoveryResult, VolumeDiscovery};
            let docker_path = sources_config.effective_docker_path();
            match VolumeDiscovery::discover_docker_volumes(&docker_path).await {
                DockerDiscoveryResult::Found { container, children } => {
                    println!("Docker volumes path: {}", container.path.display());
                    println!("Found {} volumes:", children.len());
                    for child in &children {
                        println!("  {}  {}", child.label, child.display_size());
                    }
                }
                DockerDiscoveryResult::PathNotFound { searched } => {
                    eprintln!("Docker volumes path not found: {}", searched.display());
                    eprintln!("This tool is primarily designed for Docker workloads.");
                    eprintln!("Set a custom path in sources.toml (docker_volumes_path).");
                    std::process::exit(1);
                }
                DockerDiscoveryResult::PermissionDenied { path } => {
                    eprintln!("Permission denied: {}", path.display());
                    eprintln!("Run as root or with sufficient permissions.");
                    std::process::exit(1);
                }
            }
        }

        Some(Commands::InstallSchedule) => {
            let binary_path = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/usr/local/bin/sage-restic-manager".into());
            let schedule = schedules_config.active_schedule()
                .cloned()
                .unwrap_or_default();
            scheduler::SystemdScheduler::install(&schedule, &binary_path).await
                .map_err(|e| { eprintln!("Install failed: {}", e); e })?;
            scheduler::SystemdScheduler::enable().await
                .map_err(|e| { eprintln!("Enable failed: {}", e); e })?;
            println!("Systemd timer installed and enabled.");
        }

        Some(Commands::Config) => {
            println!("Config directory: {}", config::config_dir()?.display());
            println!("Repository:       {}", config.restic_repository_url());
            println!("Budget:           {:.1} GB", config.budget.budget_gib());
            println!("Warning at:       {:.1} GB", config.budget.warning_gib());
            println!("Critical at:      {:.1} GB", config.budget.critical_gib());
            println!("Restic binary:    {}", config.restic_binary);
            println!("Update channel:   {}", config.update_channel);
            println!("Selected sources: {}", sources_config.selected_sources().len());
        }

        Some(Commands::SelfUpdate { check }) => {
            let updater = updater::Updater::new(config.update_channel.clone());
            println!("Current version: {}", updater::Updater::current_version());
            match updater.check_for_update().await? {
                Some(release) => {
                    println!("New version available: {}", release.tag_name);
                    if let Some(notes) = &release.body {
                        println!("Release notes:\n{}", notes);
                    }
                    if !check {
                        println!("Downloading update...");
                        updater.perform_update(&release).await?;
                        println!("Update complete. Restart to use the new version.");
                    }
                }
                None => {
                    println!("Already up to date.");
                }
            }
        }

        Some(Commands::Logs { lines }) => {
            let log_dir = config::log_dir()?;
            println!("Log directory: {}", log_dir.display());
            println!("Use 'journalctl -u sage-restic-manager' for systemd logs.");
        }
    }

    Ok(())
}

