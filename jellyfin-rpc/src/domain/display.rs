use serde::{Deserialize, Serialize};

/// Represents the formatting details for `Display`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct DisplayFormat {
    /// First line of the activity.
    pub details_text: Option<String>,
    /// Second line of the activity.
    pub state_text: Option<String>,
    /// Third line / large image text of the activity.
    pub image_text: Option<String>,
}

impl DisplayFormat {
    pub fn details_or(&self, fallback: &str) -> String {
        self.details_text.clone().unwrap_or_else(|| fallback.into())
    }

    pub fn state_or(&self, fallback: &str) -> String {
        self.state_text.clone().unwrap_or_else(|| fallback.into())
    }

    pub fn image_or(&self, fallback: &str) -> String {
        self.image_text.clone().unwrap_or_else(|| fallback.into())
    }

    /// Overlays every `Some` field of `other` on top of `self`.
    pub fn overlay(mut self, other: DisplayFormat) -> Self {
        if other.details_text.is_some() {
            self.details_text = other.details_text;
        }
        if other.state_text.is_some() {
            self.state_text = other.state_text;
        }
        if other.image_text.is_some() {
            self.image_text = other.image_text;
        }
        self
    }
}

/// Legacy shorthand: a list of placeholder names appended to the state line.
pub struct EpisodeDisplayOptions {
    pub divider: bool,
    pub prefix: bool,
    pub simple: bool,
}

/// Converts legacy `Vec<String>` to `DisplayFormat`
impl From<Vec<String>> for DisplayFormat {
    fn from(items: Vec<String>) -> Self {
        let details_text = "{__default}".to_string();
        let image_text = "Jellyfin-RPC v{version}".to_string();
        let mut state_text = "{__default}".to_string();

        let items_joined = items
            .iter()
            .filter(|i| !i.trim().is_empty())
            .map(|i| format!("{{{}}}", i.trim()))
            .collect::<Vec<String>>()
            .join(" {sep} ");

        if !items_joined.is_empty() {
            state_text += &items_joined;
        }

        DisplayFormat {
            details_text: Some(details_text),
            state_text: Some(state_text),
            image_text: Some(image_text),
        }
    }
}

/// Reuses `DisplayFormat::from(Vec<String>)`
impl From<String> for DisplayFormat {
    fn from(item: String) -> Self {
        let data: Vec<String> = item.split(',').map(|d| d.to_string()).collect();
        DisplayFormat::from(data)
    }
}

/// Converts `EpisodeDisplayOptions` to `DisplayFormat`
impl From<EpisodeDisplayOptions> for DisplayFormat {
    fn from(value: EpisodeDisplayOptions) -> Self {
        let details_text = "{show-title}".to_string();
        let state_text = {
            let (season_tag, episode_tag) = if value.prefix {
                (
                    "S{season-padded}".to_string(),
                    "E{episode-padded}".to_string(),
                )
            } else {
                ("S{season}".to_string(), "E{episode}".to_string())
            };

            let divider = if value.divider { " - " } else { "" };

            if value.simple {
                format!("{}{}{}", season_tag, divider, episode_tag)
            } else {
                format!("{}{}{} {}", season_tag, divider, episode_tag, "{title}")
            }
        };
        let image_text = "Jellyfin-RPC v{version}".to_string();

        DisplayFormat {
            details_text: Some(details_text),
            state_text: Some(state_text),
            image_text: Some(image_text),
        }
    }
}

/// Which activity line Discord repeats in the member-list status text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusType {
    #[default]
    Name,
    State,
    Details,
}

#[derive(Debug)]
pub struct StatusTypeFromStringError;

impl std::fmt::Display for StatusTypeFromStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "status_display_type must be one of: name, state, details")
    }
}

impl std::error::Error for StatusTypeFromStringError {}

impl TryFrom<&str> for StatusType {
    type Error = StatusTypeFromStringError;
    fn try_from(x: &str) -> Result<Self, Self::Error> {
        match x.trim().to_lowercase().as_str() {
            "name" => Ok(Self::Name),
            "state" => Ok(Self::State),
            "details" => Ok(Self::Details),
            _ => Err(StatusTypeFromStringError),
        }
    }
}

impl TryFrom<String> for StatusType {
    type Error = StatusTypeFromStringError;
    fn try_from(x: String) -> Result<Self, Self::Error> {
        Self::try_from(x.as_str())
    }
}

/// The verb Discord shows in front of the activity ("Watching ...", "Listening to ...").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivityKind {
    Playing,
    Listening,
    Watching,
    Competing,
}

#[derive(Debug)]
pub struct ActivityKindFromStringError;

impl std::fmt::Display for ActivityKindFromStringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "activity_type must be one of: playing, listening, watching, competing"
        )
    }
}

impl std::error::Error for ActivityKindFromStringError {}

impl TryFrom<&str> for ActivityKind {
    type Error = ActivityKindFromStringError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "playing" => Ok(Self::Playing),
            "listening" => Ok(Self::Listening),
            "watching" => Ok(Self::Watching),
            "competing" => Ok(Self::Competing),
            _ => Err(ActivityKindFromStringError),
        }
    }
}

impl TryFrom<String> for ActivityKind {
    type Error = ActivityKindFromStringError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

/// Everything that controls how one media category is rendered.
#[derive(Debug, Clone)]
pub struct MediaDisplayOptions {
    /// What `{sep}` expands to.
    pub separator: String,
    pub display: DisplayFormat,
    pub status_display_type: StatusType,
    /// `None` keeps the per-media-type default (Watching for video, Listening for audio).
    pub activity_kind: Option<ActivityKind>,
}

impl MediaDisplayOptions {
    pub fn new(display: DisplayFormat) -> Self {
        Self {
            separator: " • ".to_string(),
            display,
            status_display_type: StatusType::default(),
            activity_kind: None,
        }
    }
}

impl Default for MediaDisplayOptions {
    fn default() -> Self {
        Self::new(DisplayFormat::from(vec!["genres".to_string()]))
    }
}
