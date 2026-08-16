use serde::{de::Visitor, Deserialize, Serialize};

/// The type of the currently playing content.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum MediaType {
    /// If the content playing is a Movie.
    Movie,
    /// If the content playing is an Episode.
    Episode,
    /// If the content playing is a LiveTv.
    LiveTv,
    /// If the content playing is a Music.
    Music,
    /// If the content playing is a Book.
    Book,
    /// If the content playing is an Audio Book.
    AudioBook,
    /// If the content is unrecognized.
    #[default]
    None,
}

/// A person attached to a media item (Director, Actor, Writer, ...).
#[derive(Debug, Clone)]
pub struct Person {
    pub name: String,
    pub person_type: Option<String>,
}

/// A link Jellyfin knows about for an item (IMDb, TheTVDB, ...).
#[derive(Debug, Clone)]
pub struct ExternalUrl {
    pub name: String,
    pub url: String,
}

/// The item a session is currently playing.
#[derive(Debug, Clone, Default)]
pub struct NowPlayingItem {
    // Generic
    pub name: String,
    pub media_type: MediaType,
    pub id: String,
    pub run_time_ticks: Option<i64>,
    pub production_year: Option<i64>,
    pub genres: Option<Vec<String>>,
    pub external_urls: Option<Vec<ExternalUrl>>,
    pub critic_rating: Option<i64>,
    pub community_rating: Option<f64>,
    pub original_title: Option<String>,
    pub path: Option<String>,
    pub people: Option<Vec<Person>>,
    // Episode related
    pub parent_index_number: Option<i32>,
    pub index_number: Option<i32>,
    pub index_number_end: Option<i32>,
    pub series_name: Option<String>,
    pub series_id: Option<String>,
    pub series_studio: Option<String>,
    // Audio related
    pub artists: Option<Vec<String>>,
    pub extra_type: Option<String>,
    pub album_id: Option<String>,
    pub album: Option<String>,
}

impl NowPlayingItem {
    /// Name of the first person credited as Director, if any.
    pub fn director(&self) -> Option<&str> {
        self.people.as_ref()?.iter().find_map(|person| {
            person
                .person_type
                .as_deref()
                .filter(|t| *t == "Director")
                .map(|_| person.name.as_str())
        })
    }

    /// Runtime in whole minutes, derived from `run_time_ticks`.
    pub fn duration_minutes(&self) -> Option<i64> {
        self.run_time_ticks.map(|ticks| ticks / 10_000_000 / 60)
    }

    /// Genres joined with ", ".
    pub fn genres_joined(&self) -> String {
        self.genres
            .as_ref()
            .map(|g| g.join(", "))
            .unwrap_or_default()
    }

    /// True when this item is a theme song, which should never be shown.
    pub fn is_theme_song(&self) -> bool {
        self.extra_type.as_deref() == Some("ThemeSong")
    }
}

impl Serialize for MediaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            MediaType::Movie => serializer.serialize_unit_variant("MediaType", 0, "Movie"),
            MediaType::Episode => serializer.serialize_unit_variant("MediaType", 1, "Episode"),
            MediaType::LiveTv => serializer.serialize_unit_variant("MediaType", 2, "LiveTv"),
            MediaType::Music => serializer.serialize_unit_variant("MediaType", 3, "Music"),
            MediaType::Book => serializer.serialize_unit_variant("MediaType", 4, "Book"),
            MediaType::AudioBook => serializer.serialize_unit_variant("MediaType", 5, "AudioBook"),
            MediaType::None => serializer.serialize_unit_variant("MediaType", 6, "None"),
        }
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(MediaTypeVisitor)
    }
}

struct MediaTypeVisitor;

impl Visitor<'_> for MediaTypeVisitor {
    type Value = MediaType;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(MediaType::from(v.to_lowercase()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(MediaType::from(v.to_lowercase()))
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            MediaType::Episode => "Episode",
            MediaType::LiveTv => "LiveTv",
            MediaType::Movie => "Movie",
            MediaType::Music => "Music",
            MediaType::Book => "Book",
            MediaType::AudioBook => "AudioBook",
            MediaType::None => "None",
        };
        write!(f, "{}", res)
    }
}

impl From<&str> for MediaType {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "episode" => Self::Episode,
            "movie" => Self::Movie,
            "music" | "audio" => Self::Music,
            "livetv" | "tvchannel" => Self::LiveTv,
            "book" => Self::Book,
            "audiobook" => Self::AudioBook,
            _ => Self::None,
        }
    }
}

impl From<String> for MediaType {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}
