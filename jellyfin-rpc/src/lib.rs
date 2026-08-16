//! Jellyfin Rich Presence.
//!
//! The crate is split into three layers that only depend inwards:
//!
//! * [`domain`] — media, sessions, presences, blacklist rules. Pure data and rules.
//! * [`application`] — the use case ([`PresenceService`]) plus the ports it needs
//!   ([`MediaSource`], [`PresenceSink`], [`ImageProvider`]).
//! * [`infrastructure`] — adapters implementing those ports with real I/O:
//!   the Jellyfin HTTP API, the Discord IPC socket, imgur/litterbox.
//!
//! [`RpcBuilder`] is the composition root helper that wires a service together.
//!
//! # Which device is watched?
//!
//! None in particular. Sessions are matched by **username**, so playback started
//! from a phone, a browser or the desktop app all show up identically. The one
//! machine that matters is the one running this code: Discord's IPC socket is
//! local, so the binary has to run next to the Discord client.
//!
//! # Example
//! ```no_run
//! use jellyfin_rpc::{RpcBuilder, ServerConfig};
//!
//! let mut builder = RpcBuilder::new();
//! builder
//!     .username("user")
//!     .add_server(ServerConfig::new("vps", "https://jellyfin.example.com", "abcd1234"))
//!     .add_server(ServerConfig::new("local", "http://localhost:8096", "efgh5678"));
//!
//! let mut service = builder.build().unwrap();
//! service.connect().unwrap();
//! service.tick().unwrap();
//! ```

pub mod application;
pub mod domain;
pub mod infrastructure;

mod builder;
#[cfg(test)]
mod tests;

pub use application::{
    ImageProvider, MediaSource, PresenceConfig, PresenceService, PresenceSink, Tick,
};
pub use builder::{ImageHosting, RpcBuilder};
pub use domain::{
    ActivityKind, Blacklist, Button, DeviceInfo, DisplayFormat, EpisodeDisplayOptions, JfError,
    Library, MediaDisplayOptions, MediaType, Presence, Session, StatusType,
};
pub use infrastructure::discord::DEFAULT_APPLICATION_ID;
pub use infrastructure::images::ImageProcessingOptions;
pub use infrastructure::jellyfin::ServerConfig;

pub type JfResult<T> = Result<T, Box<dyn std::error::Error>>;

pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
