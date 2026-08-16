use crate::domain::{Library, Presence, Session};
use crate::JfResult;
use url::Url;

/// A place sessions can be read from — in practice one Jellyfin server.
///
/// The service holds several of these, which is what lets one RPC process watch
/// a remote VPS server and a server on localhost at the same time.
pub trait MediaSource: Send {
    /// Human readable name used in logs and to key per-server caches.
    fn name(&self) -> &str;

    /// The first playable session belonging to one of `usernames`, if any.
    ///
    /// Device independent on purpose: a session started from a phone, a browser
    /// or the desktop app all look the same here.
    fn active_session(&self, usernames: &[String]) -> JfResult<Option<Session>>;

    /// All libraries on this server, used to resolve the library blacklist.
    fn libraries(&self) -> JfResult<Vec<Library>>;

    /// URL of the primary artwork for the session's item.
    fn primary_image_url(&self, session: &Session, max_height: Option<u32>) -> JfResult<Url>;

    /// Downloads bytes from this server (authenticated).
    fn fetch_bytes(&self, url: &Url) -> JfResult<Vec<u8>>;
}

/// Where a built [`Presence`] is published — in practice the Discord IPC socket.
pub trait PresenceSink: Send {
    fn connect(&mut self) -> JfResult<()>;
    fn reconnect(&mut self) -> JfResult<()>;
    fn set(&mut self, presence: &Presence) -> JfResult<()>;
    fn clear(&mut self) -> JfResult<()>;
}

/// Turns a session's artwork into a URL Discord can load.
///
/// Discord cannot reach `http://localhost:8096`, so a local server needs an
/// implementation that re-hosts the image somewhere public.
pub trait ImageProvider: Send {
    fn name(&self) -> &str;
    fn image_url(&self, source: &dyn MediaSource, session: &Session) -> JfResult<Url>;
}
