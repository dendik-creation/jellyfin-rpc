use crate::application::ports::PresenceSink;
use crate::domain::{ActivityKind, Presence, StatusType};
use crate::JfResult;
use discord_rich_presence::activity::{
    Activity, ActivityType, Assets, Button as ActButton, StatusDisplayType, Timestamps,
};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

/// Default application id. The *name* Discord shows ("Jellyfin") is a property
/// of the Discord application itself, so changing it means registering your own
/// application and putting its id here.
pub const DEFAULT_APPLICATION_ID: &str = "1053747938519679018";

/// Publishes presences over the local Discord IPC socket.
///
/// The socket is always local, which is why the RPC binary has to run on the
/// machine Discord runs on, even when playback happens on a phone.
pub struct DiscordIpcSink {
    client: DiscordIpcClient,
}

impl DiscordIpcSink {
    pub fn new(application_id: &str) -> Self {
        Self {
            client: DiscordIpcClient::new(application_id),
        }
    }
}

impl PresenceSink for DiscordIpcSink {
    fn connect(&mut self) -> JfResult<()> {
        self.client.connect()?;
        Ok(())
    }

    fn reconnect(&mut self) -> JfResult<()> {
        self.client.reconnect()?;
        Ok(())
    }

    fn clear(&mut self) -> JfResult<()> {
        self.client.clear_activity()?;
        Ok(())
    }

    fn set(&mut self, presence: &Presence) -> JfResult<()> {
        let mut assets = Assets::new()
            .large_image(&presence.assets.large_image)
            .large_text(&presence.assets.large_text);

        if let Some(small_image) = presence.assets.small_image.as_deref() {
            assets = assets.small_image(small_image);
        }
        if let Some(small_text) = presence.assets.small_text.as_deref() {
            assets = assets.small_text(small_text);
        }

        let mut activity = Activity::new()
            .details(&presence.details)
            .state(&presence.state)
            .assets(assets)
            .activity_type(to_activity_type(presence.activity_kind))
            .status_display_type(to_status_display_type(presence.status_display_type));

        if let Some(timestamps) = presence.timestamps {
            activity = activity.timestamps(
                Timestamps::new()
                    .start(timestamps.start)
                    .end(timestamps.end),
            );
        }

        if !presence.buttons.is_empty() {
            activity = activity.buttons(
                presence
                    .buttons
                    .iter()
                    .map(|b| ActButton::new(&b.name, &b.url))
                    .collect(),
            );
        }

        self.client.set_activity(activity)?;
        Ok(())
    }
}

fn to_activity_type(kind: ActivityKind) -> ActivityType {
    match kind {
        ActivityKind::Playing => ActivityType::Playing,
        ActivityKind::Listening => ActivityType::Listening,
        ActivityKind::Watching => ActivityType::Watching,
        ActivityKind::Competing => ActivityType::Competing,
    }
}

fn to_status_display_type(status: StatusType) -> StatusDisplayType {
    match status {
        StatusType::Name => StatusDisplayType::Name,
        StatusType::State => StatusDisplayType::State,
        StatusType::Details => StatusDisplayType::Details,
    }
}
