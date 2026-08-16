//! JSON config file.
//!
//! Backwards compatible with the single-server `main.json` used before: the old
//! `jellyfin` block still works and becomes the first server. New setups can use
//! the `servers` array instead (or as well) to watch several Jellyfin servers.

use super::{CliResult, Settings};
use jellyfin_rpc::{
    ActivityKind, Blacklist, Button, DisplayFormat, EpisodeDisplayOptions, ImageHosting,
    MediaDisplayOptions, MediaType, StatusType,
};
use log::warn;
use serde::{Deserialize, Serialize};

/// Applies the file at `path`. Returns `false` when the file does not exist.
pub fn apply(settings: &mut Settings, path: &str) -> CliResult<bool> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(Box::new(err)),
    };

    let file: ConfigFile = serde_json::from_str(&raw)?;
    file.apply(settings);

    Ok(true)
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ConfigFile {
    /// Seconds between polls.
    pub poll_interval: Option<u64>,
    pub log_level: Option<String>,
    /// Multi-server list. Polled in order, after the legacy `jellyfin` entry.
    pub servers: Option<Vec<ServerEntry>>,
    /// Legacy single-server block. Also carries the display options.
    pub jellyfin: Option<JellyfinSection>,
    pub discord: Option<DiscordSection>,
    pub imgur: Option<ImgurSection>,
    pub images: Option<ImagesSection>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerEntry {
    pub name: Option<String>,
    pub url: String,
    pub api_key: String,
    pub username: Option<Username>,
    pub self_signed_cert: Option<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct JellyfinSection {
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub username: Option<Username>,
    pub self_signed_cert: Option<bool>,
    pub music: Option<DisplaySection>,
    pub movies: Option<DisplaySection>,
    pub episodes: Option<DisplaySection>,
    pub blacklist: Option<BlacklistSection>,
    // Legacy episode formatting, only used when `episodes.display` is absent.
    pub show_simple: Option<bool>,
    pub append_prefix: Option<bool>,
    pub add_divider: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Username {
    Vec(Vec<String>),
    String(String),
}

impl Username {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Username::Vec(usernames) => usernames,
            Username::String(username) => username
                .split(',')
                .map(|u| u.trim().to_string())
                .filter(|u| !u.is_empty())
                .collect(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DisplaySection {
    pub display: Option<Display>,
    pub separator: Option<String>,
    pub status_display_type: Option<String>,
    /// `playing`, `listening`, `watching` or `competing`.
    pub activity_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Display {
    /// Legacy list of placeholder names appended to the state line.
    Vec(Vec<String>),
    /// Legacy comma separated list.
    String(String),
    /// Explicit per-line templates.
    CustomFormat(DisplayFormat),
}

impl From<Display> for DisplayFormat {
    fn from(value: Display) -> Self {
        match value {
            Display::Vec(items) => DisplayFormat::from(items),
            Display::String(items) => DisplayFormat::from(items),
            Display::CustomFormat(format) => format,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct BlacklistSection {
    pub media_types: Option<Vec<MediaType>>,
    pub libraries: Option<Vec<String>>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct DiscordSection {
    pub application_id: Option<String>,
    pub buttons: Option<Vec<Button>>,
    pub show_paused: Option<bool>,
    pub large_image_text: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ImgurSection {
    pub client_id: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ImagesSection {
    /// `off`, `direct`, `imgur` or `litterbox`. Wins over the legacy toggles below.
    pub hosting: Option<String>,
    pub enable_images: Option<bool>,
    pub imgur_images: Option<bool>,
    pub litterbox_images: Option<bool>,
    pub process_images: Option<bool>,
    pub size: Option<u32>,
    pub bg: Option<bool>,
    pub bg_blur: Option<f32>,
    pub corner_radius: Option<f32>,
    /// Where uploaded image urls are remembered.
    pub cache_path: Option<String>,
}

impl ConfigFile {
    fn apply(self, settings: &mut Settings) {
        if let Some(interval) = self.poll_interval {
            settings.poll_interval_secs = interval.max(1);
        }
        if let Some(level) = self.log_level {
            settings.log_level = Some(level);
        }

        if let Some(jellyfin) = self.jellyfin {
            jellyfin.apply(settings);
        }

        for entry in self.servers.unwrap_or_default() {
            let index = settings.servers.len();
            let slot = settings.server_slot(index);
            slot.name = entry
                .name
                .unwrap_or_else(|| format!("server-{}", index + 1));
            slot.url = entry.url;
            slot.api_key = entry.api_key;
            slot.self_signed_cert = entry.self_signed_cert.unwrap_or(false);
            if let Some(username) = entry.username {
                slot.usernames = username.into_vec();
            }
        }

        if let Some(discord) = self.discord {
            if let Some(application_id) = discord.application_id {
                settings.application_id = application_id;
            }
            if let Some(buttons) = discord.buttons {
                settings.buttons = Some(buttons);
            }
            if let Some(show_paused) = discord.show_paused {
                settings.show_paused = show_paused;
            }
            if let Some(text) = discord.large_image_text {
                settings.large_image_text = text;
            }
        }

        if let Some(imgur) = self.imgur {
            if let Some(client_id) = imgur.client_id {
                settings.images.imgur_client_id = client_id;
            }
        }

        if let Some(images) = self.images {
            images.apply(settings);
        }
    }
}

impl JellyfinSection {
    fn apply(self, settings: &mut Settings) {
        let has_server = self.url.is_some() || self.api_key.is_some();

        if has_server {
            let slot = settings.server_slot(0);
            slot.name = "jellyfin".to_string();
            if let Some(url) = self.url {
                slot.url = url;
            }
            if let Some(api_key) = self.api_key {
                slot.api_key = api_key;
            }
            slot.self_signed_cert = self.self_signed_cert.unwrap_or(false);
        }

        // The legacy username is global: it applies to every server that has none.
        if let Some(username) = self.username {
            settings.usernames = username.into_vec();
        }

        apply_display(&mut settings.music, self.music);
        apply_display(&mut settings.movies, self.movies);

        let legacy_episode_format = self.show_simple.is_some()
            || self.append_prefix.is_some()
            || self.add_divider.is_some();

        let episodes_had_display = self
            .episodes
            .as_ref()
            .is_some_and(|section| section.display.is_some());

        apply_display(&mut settings.episodes, self.episodes);

        if legacy_episode_format && !episodes_had_display {
            settings.episodes.display = DisplayFormat::from(EpisodeDisplayOptions {
                divider: self.add_divider.unwrap_or(false),
                prefix: self.append_prefix.unwrap_or(false),
                simple: self.show_simple.unwrap_or(false),
            });
        }

        if let Some(blacklist) = self.blacklist {
            settings.blacklist = Blacklist::new(
                blacklist.media_types.unwrap_or_default(),
                blacklist.libraries.unwrap_or_default(),
            );
        }
    }
}

impl ImagesSection {
    fn apply(self, settings: &mut Settings) {
        let hosting = match self.hosting.as_deref() {
            Some(value) => parse_hosting(value),
            None => legacy_hosting(&self),
        };

        if let Some(hosting) = hosting {
            settings.images.hosting = hosting;
        }

        if let Some(process) = self.process_images {
            settings.images.process = process;
        }
        if let Some(path) = self.cache_path {
            settings.images.cache_path = path;
        }
        if self.size.is_some() {
            settings.images.processing.size = self.size;
        }
        if let Some(bg) = self.bg {
            settings.images.processing.background = bg;
        }
        if let Some(blur) = self.bg_blur {
            settings.images.processing.background_blur = blur;
        }
        if self.corner_radius.is_some() {
            settings.images.processing.corner_radius = self.corner_radius;
        }
    }
}

/// Maps the pre-`hosting` toggles onto the hosting enum.
fn legacy_hosting(images: &ImagesSection) -> Option<ImageHosting> {
    match images.enable_images {
        Some(false) => Some(ImageHosting::Disabled),
        Some(true) => Some(if images.imgur_images.unwrap_or(false) {
            ImageHosting::Imgur
        } else if images.litterbox_images.unwrap_or(false) {
            ImageHosting::Litterbox
        } else {
            ImageHosting::Direct
        }),
        None => None,
    }
}

pub fn parse_hosting(value: &str) -> Option<ImageHosting> {
    match value.trim().to_lowercase().as_str() {
        "off" | "none" | "disabled" | "false" => Some(ImageHosting::Disabled),
        "direct" | "jellyfin" => Some(ImageHosting::Direct),
        "imgur" => Some(ImageHosting::Imgur),
        "litterbox" | "catbox" => Some(ImageHosting::Litterbox),
        other => {
            warn!("Unknown image hosting '{}', keeping the current one", other);
            None
        }
    }
}

fn apply_display(target: &mut MediaDisplayOptions, section: Option<DisplaySection>) {
    let Some(section) = section else {
        return;
    };

    if let Some(display) = section.display {
        // Templates are merged so a config that only sets `state_text` keeps the
        // default `details_text`.
        target.display = target.display.clone().overlay(DisplayFormat::from(display));
    }
    if let Some(separator) = section.separator {
        target.separator = separator;
    }
    if let Some(status) = section.status_display_type {
        match StatusType::try_from(status.as_str()) {
            Ok(status) => target.status_display_type = status,
            Err(err) => warn!("{}", err),
        }
    }
    if let Some(activity) = section.activity_type {
        match ActivityKind::try_from(activity.as_str()) {
            Ok(activity) => target.activity_kind = Some(activity),
            Err(err) => warn!("{}", err),
        }
    }
}
