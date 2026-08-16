//! Configuration adapter.
//!
//! Two layers, each overriding the previous one:
//!
//! 1. built-in defaults
//! 2. the JSON config file (`main.json`)
//!
//! Command line flags are applied on top by `main`.

pub mod file;
pub mod paths;

use jellyfin_rpc::{
    Blacklist, Button, DisplayFormat, EpisodeDisplayOptions, ImageHosting, ImageProcessingOptions,
    MediaDisplayOptions, RpcBuilder, ServerConfig, DEFAULT_APPLICATION_ID,
};
use log::{debug, warn};

pub type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Fully resolved configuration. Everything downstream reads this, never the
/// file or the environment directly.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Jellyfin servers, polled in order. Slot 0 is the primary one.
    pub servers: Vec<ServerConfig>,
    /// Accounts to watch on servers that do not name their own.
    pub usernames: Vec<String>,
    pub poll_interval_secs: u64,
    /// `None` leaves the level to the `-v` flag / `RUST_LOG`.
    pub log_level: Option<String>,
    pub application_id: String,
    pub show_paused: bool,
    pub buttons: Option<Vec<Button>>,
    pub large_image_text: String,
    pub music: MediaDisplayOptions,
    pub movies: MediaDisplayOptions,
    pub episodes: MediaDisplayOptions,
    pub blacklist: Blacklist,
    pub images: ImageSettings,
}

#[derive(Debug, Clone)]
pub struct ImageSettings {
    pub hosting: ImageHosting,
    pub imgur_client_id: String,
    pub cache_path: String,
    pub process: bool,
    pub processing: ImageProcessingOptions,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            servers: Vec::new(),
            usernames: Vec::new(),
            poll_interval_secs: 3,
            log_level: None,
            application_id: DEFAULT_APPLICATION_ID.to_string(),
            show_paused: true,
            buttons: None,
            large_image_text: String::new(),
            music: MediaDisplayOptions::default(),
            movies: MediaDisplayOptions::default(),
            episodes: MediaDisplayOptions::new(DisplayFormat::from(EpisodeDisplayOptions {
                divider: true,
                prefix: true,
                simple: false,
            })),
            blacklist: Blacklist::default(),
            images: ImageSettings {
                hosting: ImageHosting::Disabled,
                imgur_client_id: String::new(),
                cache_path: String::new(),
                process: true,
                processing: ImageProcessingOptions::default(),
            },
        }
    }
}

impl Settings {
    /// Defaults, then the config file if it exists.
    pub fn load(config_path: Option<&str>) -> CliResult<Self> {
        let mut settings = Settings::default();

        let path = match config_path {
            Some(path) => Some(path.to_string()),
            None => paths::config_path().ok(),
        };

        if let Some(path) = path.as_deref() {
            match file::apply(&mut settings, path) {
                Ok(true) => debug!("Loaded config file {}", path),
                Ok(false) => debug!("No config file at {}, continuing without it", path),
                Err(err) => return Err(format!("config file {} is invalid: {}", path, err).into()),
            }
        }

        if settings.images.cache_path.is_empty() {
            settings.images.cache_path = paths::urls_path()?;
        }

        settings.drop_incomplete_servers();

        Ok(settings)
    }

    /// Returns the server at `index`, creating empty slots as needed.
    ///
    /// Lets the file fill slot 0 and the environment fill slot 1 without either
    /// having to know about the other.
    pub fn server_slot(&mut self, index: usize) -> &mut ServerConfig {
        while self.servers.len() <= index {
            let position = self.servers.len() + 1;
            self.servers
                .push(ServerConfig::new(format!("server-{}", position), "", ""));
        }
        &mut self.servers[index]
    }

    fn drop_incomplete_servers(&mut self) {
        self.servers.retain(|server| {
            if server.url.is_empty() || server.api_key.is_empty() {
                if !server.url.is_empty() || !server.api_key.is_empty() {
                    warn!(
                        "Ignoring server '{}': it needs both a url and an api key",
                        server.name
                    );
                }
                false
            } else {
                true
            }
        });
    }

    /// One line per server, for the startup log.
    pub fn describe_servers(&self) -> Vec<String> {
        self.servers
            .iter()
            .map(|server| {
                let watching = if server.usernames.is_empty() {
                    self.usernames.join(", ")
                } else {
                    server.usernames.join(", ")
                };
                format!("{} -> {} (user: {})", server.name, server.url, watching)
            })
            .collect()
    }

    pub fn into_builder(self) -> RpcBuilder {
        let mut builder = RpcBuilder::new();

        builder
            .servers(self.servers)
            .usernames(self.usernames)
            .application_id(self.application_id)
            .show_paused(self.show_paused)
            .large_image_text(self.large_image_text)
            .music(self.music)
            .movies(self.movies)
            .episodes(self.episodes)
            .blacklist(self.blacklist)
            .image_hosting(self.images.hosting)
            .imgur_client_id(self.images.imgur_client_id)
            .image_cache_path(self.images.cache_path)
            .process_images(self.images.process)
            .image_processing(self.images.processing);

        if let Some(buttons) = self.buttons {
            builder.buttons(buttons);
        }

        builder
    }
}
