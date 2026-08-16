//! Domain layer.
//!
//! Pure business types. No HTTP, no filesystem, no Discord, no image encoding.
//! The only outside crate allowed in here is `serde`, and only on the handful of
//! value objects that are written verbatim in user config files
//! ([`MediaType`], [`DisplayFormat`], [`Button`]) so the config layer does not
//! have to maintain a parallel copy of them.

pub mod button;
pub mod display;
pub mod error;
pub mod library;
pub mod media;
pub mod presence;
pub mod session;

pub use button::Button;
pub use display::{
    ActivityKind, DisplayFormat, EpisodeDisplayOptions, MediaDisplayOptions, StatusType,
    StatusTypeFromStringError,
};
pub use error::JfError;
pub use library::{Blacklist, Library};
pub use media::{ExternalUrl, MediaType, NowPlayingItem, Person};
pub use presence::{Presence, PresenceAssets, PresenceTimestamps};
pub use session::{DeviceInfo, PlayState, PlayTime, Session};
