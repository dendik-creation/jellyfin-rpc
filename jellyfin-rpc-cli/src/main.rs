//! Composition root.
//!
//! Parses flags, resolves configuration, wires a [`jellyfin_rpc::PresenceService`]
//! and hands it to the [`runner::Runner`]. No business logic lives here.

use clap::Parser;
use colored::Colorize;
use config::Settings;
use log::{error, info, warn};
use runner::Runner;
use simple_logger::SimpleLogger;
use time::macros::format_description;

mod config;
mod runner;
#[cfg(feature = "updates")]
mod updates;

#[derive(Parser)]
#[command(author = "Radical <Radiicall> <radical@radical.fun>")]
#[command(version)]
#[command(about = "Rich presence for Jellyfin", long_about = None)]
struct Args {
    #[arg(short = 'c', long = "config", help = "Path to the config file")]
    config: Option<String>,
    #[arg(
        short = 'i',
        long = "image-urls-file",
        help = "Path to the uploaded image url cache"
    )]
    image_urls: Option<String>,
    #[arg(
        short = 't',
        long = "wait-time",
        help = "Seconds between polls, overrides the config"
    )]
    wait_time: Option<u64>,
    #[arg(
        short = 'v',
        long = "log-level",
        help = "Sets the log level to one of: trace, debug, info, warn, error, off"
    )]
    log_level: Option<String>,
    #[arg(
        long = "print-config",
        help = "Print the resolved configuration and exit"
    )]
    print_config: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut settings = match Settings::load(args.config.as_deref()) {
        Ok(settings) => settings,
        Err(err) => {
            eprintln!("{} {}", "Configuration error:".red(), err);
            std::process::exit(1);
        }
    };

    // CLI flags beat both the file and the environment.
    if let Some(wait_time) = args.wait_time {
        settings.poll_interval_secs = wait_time.max(1);
    }
    if let Some(path) = args.image_urls {
        settings.images.cache_path = path;
    }
    if let Some(level) = args.log_level.or(settings.log_level.clone()) {
        settings.log_level = Some(level);
    }

    init_logger(settings.log_level.as_deref());

    info!("Initializing Jellyfin-RPC");

    #[cfg(feature = "updates")]
    updates::checker();

    if args.print_config {
        println!("{:#?}", settings);
        return Ok(());
    }

    if settings.servers.is_empty() {
        error!("No Jellyfin server configured.");
        error!(
            "Every server needs both {} and {}.",
            "url".green(),
            "api_key".green()
        );
        error!(
            "Config file location: {}",
            config::paths::config_path()
                .unwrap_or_else(|_| "unknown".to_string())
                .yellow()
        );
        std::process::exit(1);
    }

    if settings.usernames.is_empty() && settings.servers.iter().any(|s| s.usernames.is_empty()) {
        error!("No username configured. Set {}.", "jellyfin.username".green());
        std::process::exit(1);
    }

    for line in settings.describe_servers() {
        info!("Watching {}", line);
    }
    info!("Polling every {}s", settings.poll_interval_secs);

    if settings.servers.len() > 1 {
        info!("Servers are checked in order; the first one playing something wins");
    }

    warn_about_local_images(&settings);

    let interval = settings.poll_interval_secs;
    let service = settings.into_builder().build()?;

    let mut runner = Runner::new(service, interval);
    runner.connect();
    runner.run();
}

/// A `localhost` server's artwork URL is unreachable for Discord's CDN, so
/// direct hosting silently falls back to the generic Jellyfin logo.
fn warn_about_local_images(settings: &Settings) {
    use jellyfin_rpc::ImageHosting;

    if settings.images.hosting != ImageHosting::Direct {
        return;
    }

    let has_private_server = settings.servers.iter().any(|server| {
        let url = server.url.to_lowercase();
        url.contains("localhost") || url.contains("127.0.0.1") || url.contains("192.168.")
    });

    if has_private_server {
        warn!("A server is on a private address and images are set to 'direct'.");
        warn!("Discord cannot load artwork from it — use IMAGES_HOSTING=imgur or litterbox.");
    }
}

fn init_logger(level: Option<&str>) {
    if let Some(level) = level {
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", level);
        }
    }

    let _ = SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .with_timestamp_format(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .init();
}
