use super::cache::UrlCache;
use super::processing::{make_square_with_blur, ImageProcessingOptions};
use crate::application::ports::{ImageProvider, MediaSource};
use crate::domain::Session;
use crate::JfResult;
use chrono::Utc;
use log::debug;
use reqwest::blocking::multipart::{Form, Part};
use url::Url;

/// Litterbox deletes uploads after this window, so cached links must not outlive it.
const RETENTION_HOURS: i64 = 72;

/// Re-hosts Jellyfin artwork on litterbox.catbox.moe. Same purpose as
/// [`super::ImgurImageProvider`] but needs no account; links expire after 72h.
pub struct LitterboxImageProvider {
    cache: UrlCache,
    processing: Option<ImageProcessingOptions>,
    http: reqwest::blocking::Client,
}

impl LitterboxImageProvider {
    pub fn new<P: Into<std::path::PathBuf>>(
        cache_path: P,
        processing: Option<ImageProcessingOptions>,
    ) -> JfResult<Self> {
        Ok(Self {
            cache: UrlCache::new(cache_path, Some(RETENTION_HOURS)),
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

        debug!("Uploading artwork for {} to litterbox", session.item_id);

        let form = Form::new()
            .text("reqtype", "fileupload")
            .text("time", "72h")
            .part(
                "fileToUpload",
                Part::bytes(body).file_name(format!("{}.png", Utc::now().timestamp())),
            );

        let response = self
            .http
            .post("https://litterbox.catbox.moe/resources/internals/api.php")
            .multipart(form)
            .send()?
            .text()?;

        Ok(Url::parse(response.trim())?)
    }
}

impl ImageProvider for LitterboxImageProvider {
    fn name(&self) -> &str {
        "litterbox"
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
