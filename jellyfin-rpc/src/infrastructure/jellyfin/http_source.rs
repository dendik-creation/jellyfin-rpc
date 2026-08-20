use super::dto::{RawItemDetails, RawSession, RawVirtualFolder};
use crate::application::ports::MediaSource;
use crate::domain::{JfError, Library, MediaType, Session};
use crate::JfResult;
use log::debug;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use url::Url;

/// One Jellyfin server the RPC should watch.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Label shown in logs, e.g. "vps" or "local".
    pub name: String,
    /// Base url, e.g. `https://stream.example.dev` or `http://localhost:8096`.
    pub url: String,
    pub api_key: String,
    /// Accounts to watch on this server. Empty falls back to the global list.
    pub usernames: Vec<String>,
    /// Accept a self signed TLS certificate.
    pub self_signed_cert: bool,
}

impl ServerConfig {
    pub fn new<N: Into<String>, U: Into<String>, K: Into<String>>(
        name: N,
        url: U,
        api_key: K,
    ) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            api_key: api_key.into(),
            usernames: Vec::new(),
            self_signed_cert: false,
        }
    }
}

/// [`MediaSource`] backed by the Jellyfin HTTP API.
pub struct JellyfinHttpSource {
    name: String,
    url: Url,
    usernames: Vec<String>,
    http: reqwest::blocking::Client,
}

impl JellyfinHttpSource {
    pub fn new(config: ServerConfig) -> JfResult<Self> {
        if config.url.is_empty() || config.api_key.is_empty() {
            return Err(Box::new(JfError::MissingRequiredValues));
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("MediaBrowser Token=\"{}\"", config.api_key).parse()?,
        );
        headers.insert("X-Emby-Token", config.api_key.parse()?);

        // `Url::join` drops the last path segment unless the base ends in a slash.
        let base = if config.url.ends_with('/') {
            config.url.clone()
        } else {
            format!("{}/", config.url)
        };

        Ok(Self {
            name: config.name,
            url: base.parse()?,
            usernames: config.usernames,
            http: reqwest::blocking::Client::builder()
                .default_headers(headers)
                .danger_accept_invalid_certs(config.self_signed_cert)
                .build()?,
        })
    }

    /// `GET path`, with a readable error instead of "error decoding response body".
    fn get(&self, path: &str) -> JfResult<reqwest::blocking::Response> {
        let response = self.http.get(self.url.join(path)?).send()?;
        let status = response.status();

        if status.is_success() {
            return Ok(response);
        }

        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            return Err(format!(
                "{} rejected the API key ({})",
                self.url, status
            )
            .into());
        }

        Err(format!("{}{} returned {}", self.url, path, status).into())
    }

    fn effective_usernames<'a>(&'a self, fallback: &'a [String]) -> &'a [String] {
        if self.usernames.is_empty() {
            fallback
        } else {
            &self.usernames
        }
    }

    /// Movies only carry People when asked for explicitly, so `{director}` needs a second call.
    fn enrich_people(&self, session: &mut Session) {
        if session.now_playing_item.media_type != MediaType::Movie
            || session.now_playing_item.people.is_some()
        {
            return;
        }

        let path = format!("Items/{}?Fields=People", session.now_playing_item.id);

        let people = self
            .get(&path)
            .ok()
            .and_then(|response| response.json::<RawItemDetails>().ok())
            .map(RawItemDetails::into_domain);

        if let Some(people) = people {
            session.now_playing_item.people = Some(people);
        }
    }
}

impl MediaSource for JellyfinHttpSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn active_session(&self, usernames: &[String]) -> JfResult<Option<Session>> {
        let sessions: Vec<RawSession> = self.get("Sessions")?.json()?;

        debug!("'{}' returned {} sessions", self.name, sessions.len());

        let wanted = self.effective_usernames(usernames);

        // Username order is a priority order: the first account with something
        // playing wins, regardless of which device started it.
        let found = wanted.iter().find_map(|username| {
            sessions
                .iter()
                .filter(|session| session.user_name.as_ref().is_some_and(|u| u == username))
                .find(|session| session.is_playable())
                .cloned()
        });

        let Some(raw) = found else {
            return Ok(None);
        };

        let Some(mut session) = raw.into_domain(&self.name) else {
            return Ok(None);
        };

        self.enrich_people(&mut session);

        Ok(Some(session))
    }

    fn libraries(&self) -> JfResult<Vec<Library>> {
        let folders: Vec<RawVirtualFolder> = self.get("Library/VirtualFolders")?.json()?;

        Ok(folders
            .into_iter()
            .map(RawVirtualFolder::into_domain)
            .collect())
    }

    fn primary_image_url(&self, session: &Session, max_height: Option<u32>) -> JfResult<Url> {
        // For music the track can have its own cover; fall back to the album's.
        let ids: Vec<&str> = if session.now_playing_item.media_type == MediaType::Music {
            vec![&session.now_playing_item.id, &session.item_id]
        } else {
            vec![&session.item_id]
        };

        for id in ids {
            let mut image_url = self.url.join(&format!("Items/{}/Images/Primary", id))?;

            if let Some(height) = max_height {
                image_url
                    .query_pairs_mut()
                    .append_pair("maxHeight", &height.to_string());
            }

            // Jellyfin answers 404 with a text body rather than an image.
            if !self
                .http
                .get(image_url.as_ref())
                .send()?
                .text()?
                .contains("does not have an image of type Primary")
            {
                return Ok(image_url);
            }
        }

        Err(Box::new(JfError::NoImage))
    }

    fn fetch_bytes(&self, url: &Url) -> JfResult<Vec<u8>> {
        Ok(self.http.get(url.as_ref()).send()?.bytes()?.to_vec())
    }
}
