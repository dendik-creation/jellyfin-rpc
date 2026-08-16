use super::button::Button;
use super::display::{ActivityKind, StatusType};

/// Discord refuses statuses shorter than 3 characters; pad with zero width joiners.
const MIN_LEN: usize = 3;
const MAX_LEN: usize = 128;
const PADDING: &str = "\u{200e}\u{200e}\u{200e}";

/// Everything Discord needs to render one activity.
///
/// This is the output of the application layer and the input of the Discord
/// adapter. It knows nothing about the IPC protocol.
#[derive(Debug, Clone)]
pub struct Presence {
    pub details: String,
    pub state: String,
    pub assets: PresenceAssets,
    pub timestamps: Option<PresenceTimestamps>,
    pub buttons: Vec<Button>,
    pub activity_kind: ActivityKind,
    pub status_display_type: StatusType,
}

#[derive(Debug, Clone, Default)]
pub struct PresenceAssets {
    pub large_image: String,
    pub large_text: String,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresenceTimestamps {
    pub start: i64,
    pub end: i64,
}

impl Presence {
    /// One-line summary used for logging and for change detection between polls.
    pub fn summary(&self) -> String {
        format!("{} | {}", self.details, self.state)
    }

    /// Clamps every text field to Discord's 3..=128 character window.
    pub fn clamp_text_fields(&mut self) {
        self.details = clamp(&self.details);
        self.state = clamp(&self.state);
        self.assets.large_text = clamp(&self.assets.large_text);
        self.assets.small_text = self.assets.small_text.as_deref().map(clamp);
    }
}

fn clamp(input: &str) -> String {
    let mut out = input.to_string();

    if out.chars().count() > MAX_LEN {
        out = out.chars().take(MAX_LEN).collect();
    } else if out.chars().count() < MIN_LEN {
        out += PADDING;
    }

    out
}
