use crate::application::ports::{ImageProvider, MediaSource};
use crate::domain::Session;
use crate::JfResult;
use url::Url;

/// Hands Discord the Jellyfin artwork URL as-is.
///
/// Only works when Discord's CDN can reach the server, so it is the right
/// provider for a public server and useless on its own for `localhost:8096`.
pub struct DirectImageProvider {
    max_height: Option<u32>,
}

impl DirectImageProvider {
    pub fn new(max_height: Option<u32>) -> Self {
        Self { max_height }
    }
}

impl ImageProvider for DirectImageProvider {
    fn name(&self) -> &str {
        "direct"
    }

    fn image_url(&self, source: &dyn MediaSource, session: &Session) -> JfResult<Url> {
        source.primary_image_url(session, self.max_height)
    }
}
