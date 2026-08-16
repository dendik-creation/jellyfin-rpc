//! Infrastructure layer.
//!
//! Adapters that implement the application's ports using real I/O: HTTP to
//! Jellyfin, the Discord IPC socket, image hosts and image encoding.

pub mod discord;
pub mod images;
pub mod jellyfin;

pub use discord::{DiscordIpcSink, DEFAULT_APPLICATION_ID};
pub use images::{
    ChainImageProvider, DirectImageProvider, ImageProcessingOptions, ImgurImageProvider,
    LitterboxImageProvider, UrlCache,
};
pub use jellyfin::{JellyfinHttpSource, ServerConfig};
