//! Wire format of the Jellyfin API, kept separate from the domain types.
//!
//! Everything here is `pub(crate)`: nothing outside the adapter should have to
//! know that Jellyfin speaks PascalCase JSON.

use crate::domain::{
    DeviceInfo, ExternalUrl, Library, MediaType, NowPlayingItem, Person, PlayState, Session,
};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawSession {
    pub user_name: Option<String>,
    pub now_playing_item: Option<RawNowPlayingItem>,
    pub play_state: Option<RawPlayState>,
    pub client: Option<String>,
    pub device_name: Option<String>,
    pub remote_end_point: Option<String>,
}

impl RawSession {
    /// A session is usable when it has an item, a play state, and is not a theme song.
    pub(crate) fn is_playable(&self) -> bool {
        match (&self.now_playing_item, &self.play_state) {
            (Some(item), Some(_)) => item.extra_type.as_deref() != Some("ThemeSong"),
            _ => false,
        }
    }

    /// Consumes the wire type into the domain type. Only call after [`Self::is_playable`].
    pub(crate) fn into_domain(self, source_name: &str) -> Option<Session> {
        let raw_item = self.now_playing_item?;
        let play_state = self.play_state?;

        let item = raw_item.into_domain();

        // Artwork lives on the series for episodes and on the album for tracks.
        let item_id = match item.media_type {
            MediaType::Episode => item.series_id.clone().unwrap_or_else(|| item.id.clone()),
            MediaType::Music => item.album_id.clone().unwrap_or_else(|| item.id.clone()),
            _ => item.id.clone(),
        };

        Some(Session {
            now_playing_item: item,
            play_state: PlayState {
                is_paused: play_state.is_paused,
                position_ticks: play_state.position_ticks,
            },
            item_id,
            device: DeviceInfo {
                client: self.client,
                device_name: self.device_name,
                remote_end_point: self.remote_end_point,
            },
            source_name: source_name.to_string(),
            user_name: self.user_name.unwrap_or_default(),
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawPlayState {
    pub is_paused: bool,
    pub position_ticks: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawNowPlayingItem {
    pub name: String,
    #[serde(rename = "Type")]
    pub media_type: MediaType,
    pub id: String,
    pub run_time_ticks: Option<i64>,
    pub production_year: Option<i64>,
    pub genres: Option<Vec<String>>,
    pub external_urls: Option<Vec<RawExternalUrl>>,
    pub critic_rating: Option<i64>,
    pub community_rating: Option<f64>,
    pub original_title: Option<String>,
    pub path: Option<String>,
    pub people: Option<Vec<RawPerson>>,
    pub parent_index_number: Option<i32>,
    pub index_number: Option<i32>,
    pub index_number_end: Option<i32>,
    pub series_name: Option<String>,
    pub series_id: Option<String>,
    pub series_studio: Option<String>,
    pub artists: Option<Vec<String>>,
    pub extra_type: Option<String>,
    pub album_id: Option<String>,
    pub album: Option<String>,
}

impl RawNowPlayingItem {
    fn into_domain(self) -> NowPlayingItem {
        NowPlayingItem {
            name: self.name,
            media_type: self.media_type,
            id: self.id,
            run_time_ticks: self.run_time_ticks,
            production_year: self.production_year,
            genres: self.genres,
            external_urls: self
                .external_urls
                .map(|urls| urls.into_iter().map(RawExternalUrl::into_domain).collect()),
            critic_rating: self.critic_rating,
            community_rating: self.community_rating,
            original_title: self.original_title,
            path: self.path,
            people: self
                .people
                .map(|people| people.into_iter().map(RawPerson::into_domain).collect()),
            parent_index_number: self.parent_index_number,
            index_number: self.index_number,
            index_number_end: self.index_number_end,
            series_name: self.series_name,
            series_id: self.series_id,
            series_studio: self.series_studio,
            artists: self.artists,
            extra_type: self.extra_type,
            album_id: self.album_id,
            album: self.album,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawExternalUrl {
    pub name: String,
    pub url: String,
}

impl RawExternalUrl {
    fn into_domain(self) -> ExternalUrl {
        ExternalUrl {
            name: self.name,
            url: self.url,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawPerson {
    pub name: String,
    #[serde(rename = "Type")]
    pub person_type: Option<String>,
}

impl RawPerson {
    fn into_domain(self) -> Person {
        Person {
            name: self.name,
            person_type: self.person_type,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawVirtualFolder {
    pub name: Option<String>,
    pub locations: Vec<String>,
}

impl RawVirtualFolder {
    pub(crate) fn into_domain(self) -> Library {
        Library {
            name: self.name,
            locations: self.locations,
        }
    }
}

/// `GET /Items/{id}?Fields=People`
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawItemDetails {
    pub people: Option<Vec<RawPerson>>,
}

impl RawItemDetails {
    pub(crate) fn into_domain(self) -> Vec<Person> {
        self.people
            .unwrap_or_default()
            .into_iter()
            .map(RawPerson::into_domain)
            .collect()
    }
}
