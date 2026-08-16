//! Application layer.
//!
//! Use cases and the ports (traits) they need. Depends on [`crate::domain`] only;
//! never on reqwest, discord-rich-presence or the filesystem.

pub mod ports;
pub mod presence_builder;
pub mod service;
pub mod template;

pub use ports::{ImageProvider, MediaSource, PresenceSink};
pub use presence_builder::{PresenceBuilder, PresenceConfig};
pub use service::{PresenceService, Tick};
pub use template::TemplateRenderer;
