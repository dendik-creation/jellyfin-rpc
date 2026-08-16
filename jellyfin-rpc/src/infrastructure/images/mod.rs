mod cache;
mod direct;
mod imgur;
mod litterbox;
mod processing;

pub use cache::UrlCache;
pub use direct::DirectImageProvider;
pub use imgur::ImgurImageProvider;
pub use litterbox::LitterboxImageProvider;
pub use processing::{make_square_with_blur, ImageProcessingOptions};

use crate::application::ports::{ImageProvider, MediaSource};
use crate::domain::{JfError, Session};
use crate::JfResult;
use log::debug;
use url::Url;

/// Tries each provider in order and returns the first URL produced.
///
/// The usual chain is `[imgur, direct]`: re-host on imgur so Discord can load the
/// artwork of a server it cannot reach, and fall back to the raw Jellyfin URL
/// when the upload fails.
pub struct ChainImageProvider {
    providers: Vec<Box<dyn ImageProvider>>,
}

impl ChainImageProvider {
    pub fn new(providers: Vec<Box<dyn ImageProvider>>) -> Self {
        Self { providers }
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl ImageProvider for ChainImageProvider {
    fn name(&self) -> &str {
        "chain"
    }

    fn image_url(&self, source: &dyn MediaSource, session: &Session) -> JfResult<Url> {
        for provider in &self.providers {
            match provider.image_url(source, session) {
                Ok(url) => return Ok(url),
                Err(err) => debug!("Image provider '{}' failed: {}", provider.name(), err),
            }
        }

        Err(Box::new(JfError::NoImage))
    }
}
