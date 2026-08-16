use super::cache::UrlCache;
use super::processing::{make_square_with_blur, ImageProcessingOptions};
use crate::application::ports::{ImageProvider, MediaSource};
use crate::domain::Session;
use crate::JfResult;
use log::debug;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
struct ImgurResponse {
    data: ImgurData,
}

#[derive(Deserialize)]
struct ImgurData {
    link: String,
}

/// Re-hosts Jellyfin artwork on imgur so Discord can load it even when the
/// Jellyfin server is only reachable on the local network.
pub struct ImgurImageProvider {
    client_id: String,
    cache: UrlCache,
    processing: Option<ImageProcessingOptions>,
    http: reqwest::blocking::Client,
}

impl ImgurImageProvider {
    pub fn new<P: Into<std::path::PathBuf>>(
        client_id: String,
        cache_path: P,
        processing: Option<ImageProcessingOptions>,
    ) -> JfResult<Self> {
        Ok(Self {
            client_id,
            // Imgur links do not expire.
            cache: UrlCache::new(cache_path, None),
            processing,
            http: reqwest::blocking::Client::builder().build()?,
        })
    }

    fn upload(&self, source: &dyn MediaSource, session: &Session) -> JfResult<Url> {
        let max_height = self.processing.as_ref().and_then(|opts| opts.size);
        let jellyfin_url = source.primary_image_url(session, max_height)?;
        let bytes = source.fetch_bytes(&jellyfin_url)?;

        let body = match &self.processing {
            Some(options) => make_square_with_blur(&bytes, options)?,
            None => bytes,
        };

        debug!("Uploading artwork for {} to imgur", session.item_id);

        let response: ImgurResponse = self
            .http
            .post("https://api.imgur.com/3/image")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Client-ID {}", self.client_id),
            )
            .body(body)
            .send()?
            .json()?;

        Ok(Url::parse(&response.data.link)?)
    }
}

impl ImageProvider for ImgurImageProvider {
    fn name(&self) -> &str {
        "imgur"
    }

    fn image_url(&self, source: &dyn MediaSource, session: &Session) -> JfResult<Url> {
        if let Some(cached) = self.cache.get(&session.item_id) {
            return Ok(cached);
        }

        let uploaded = self.upload(source, session)?;
        let _ = self.cache.put(&session.item_id, &uploaded);

        Ok(uploaded)
    }
}
