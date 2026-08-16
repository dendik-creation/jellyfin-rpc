use super::media::NowPlayingItem;
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

const TICKS_PER_SECOND: i64 = 10_000_000;

/// Which client/device produced a session.
///
/// Purely informational: the presence is shown regardless of device, but this
/// lets the CLI log "playing from Jellyfin Android on Pixel 7" so you can tell a
/// phone session apart from a browser session.
#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    /// Jellyfin client name, e.g. "Jellyfin Android", "Jellyfin Web".
    pub client: Option<String>,
    /// Device name, e.g. "Pixel 7", "Chrome".
    pub device_name: Option<String>,
    /// Address the session connected from.
    pub remote_end_point: Option<String>,
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.client.as_deref(), self.device_name.as_deref()) {
            (Some(client), Some(device)) => write!(f, "{} on {}", client, device),
            (Some(client), None) => write!(f, "{}", client),
            (None, Some(device)) => write!(f, "{}", device),
            (None, None) => write!(f, "unknown device"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlayState {
    pub is_paused: bool,
    pub position_ticks: Option<i64>,
}

/// A playing Jellyfin session, already validated: it has an item and a play state.
#[derive(Debug, Clone)]
pub struct Session {
    pub now_playing_item: NowPlayingItem,
    pub play_state: PlayState,
    /// Id used for artwork lookups (series id for episodes, album id for music).
    pub item_id: String,
    pub device: DeviceInfo,
    /// Name of the configured server this session came from.
    pub source_name: String,
    pub user_name: String,
}

impl Session {
    /// Formats artists with comma separation and a final "and" before the last name.
    pub fn format_artists(&self) -> String {
        let default = Vec::new();
        let artists_vec = self.now_playing_item.artists.as_ref().unwrap_or(&default);
        let mut artists = String::new();

        for i in 0..artists_vec.len() {
            if i == 0 {
                artists += &artists_vec[i];
                continue;
            }

            if i == artists_vec.len() - 1 {
                artists += &format!(" and {}", artists_vec[i]);
                continue;
            }

            artists += &format!(", {}", artists_vec[i]);
        }

        artists
    }

    pub fn get_time(&self) -> Result<PlayTime, SystemTimeError> {
        use super::media::MediaType;

        match self.now_playing_item.media_type {
            MediaType::Book | MediaType::LiveTv => return Ok(PlayTime::None),
            _ => {}
        }

        if self.play_state.is_paused
            || self.play_state.position_ticks.is_none()
            || self.now_playing_item.run_time_ticks.is_none()
        {
            return Ok(PlayTime::Paused);
        }

        let position = self.play_state.position_ticks.unwrap_or(0) / TICKS_PER_SECOND;
        let runtime = self.now_playing_item.run_time_ticks.unwrap_or(0) / TICKS_PER_SECOND;
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        Ok(PlayTime::Some(now - position, now + (runtime - position)))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlayTime {
    Some(i64, i64),
    Paused,
    None,
}
