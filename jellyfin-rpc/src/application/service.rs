use super::ports::{ImageProvider, MediaSource, PresenceSink};
use super::presence_builder::{PresenceBuilder, PresenceConfig};
use crate::domain::{Blacklist, JfError, Library};
use crate::JfResult;
use log::{debug, warn};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// How long a server's library list is trusted before it is fetched again.
const LIBRARY_CACHE_TTL: Duration = Duration::from_secs(3600);

/// Outcome of one poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tick {
    /// Something is playing and the presence was published.
    Playing {
        summary: String,
        source: String,
        device: String,
    },
    /// Nothing is playing; the presence was cleared if it had been set.
    Idle,
    /// A session exists but is hidden (blacklist, or paused with show_paused off).
    Hidden,
}

/// The main use case: read sessions from every configured server, decide what
/// should be shown, publish it to the presence sink.
pub struct PresenceService {
    sources: Vec<Box<dyn MediaSource>>,
    sink: Box<dyn PresenceSink>,
    image_provider: Option<Box<dyn ImageProvider>>,
    config: PresenceConfig,
    usernames: Vec<String>,
    blacklist: Blacklist,
    /// Per-server cache of the blacklisted libraries resolved on that server.
    library_cache: HashMap<String, (Vec<Library>, SystemTime)>,
    showing: bool,
}

impl PresenceService {
    pub fn new(
        sources: Vec<Box<dyn MediaSource>>,
        sink: Box<dyn PresenceSink>,
        image_provider: Option<Box<dyn ImageProvider>>,
        config: PresenceConfig,
        usernames: Vec<String>,
        blacklist: Blacklist,
    ) -> JfResult<Self> {
        if sources.is_empty() {
            return Err(Box::new(JfError::NoSources));
        }

        Ok(Self {
            sources,
            sink,
            image_provider,
            config,
            usernames,
            blacklist,
            library_cache: HashMap::new(),
            showing: false,
        })
    }

    pub fn connect(&mut self) -> JfResult<()> {
        self.sink.connect()
    }

    pub fn reconnect(&mut self) -> JfResult<()> {
        self.showing = false;
        self.sink.reconnect()
    }

    pub fn clear(&mut self) -> JfResult<()> {
        self.showing = false;
        self.sink.clear()
    }

    pub fn source_names(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.name()).collect()
    }

    /// Polls every server once and publishes the result.
    pub fn tick(&mut self) -> JfResult<Tick> {
        let Some((index, session)) = self.find_session()? else {
            return self.go_idle();
        };

        debug!(
            "Session on '{}': {} ({}) via {}",
            session.source_name,
            session.now_playing_item.name,
            session.now_playing_item.media_type,
            session.device
        );

        if session.now_playing_item.media_type == crate::domain::MediaType::None {
            return Err(Box::new(JfError::UnrecognizedMediaType));
        }

        if self.is_blacklisted(index, &session) {
            debug!("'{}' is blacklisted", session.now_playing_item.name);
            if self.showing {
                self.clear()?;
            }
            return Ok(Tick::Hidden);
        }

        let image_url = self.resolve_image(index, &session);

        let presence = PresenceBuilder::new(&self.config).build(&session, image_url.as_deref())?;

        let Some(presence) = presence else {
            // Paused and show_paused is off.
            if self.showing {
                self.clear()?;
            }
            return Ok(Tick::Hidden);
        };

        let summary = presence.summary();
        self.sink.set(&presence)?;
        self.showing = true;

        Ok(Tick::Playing {
            summary,
            source: session.source_name.clone(),
            device: session.device.to_string(),
        })
    }

    fn go_idle(&mut self) -> JfResult<Tick> {
        if self.showing {
            self.clear()?;
        }
        Ok(Tick::Idle)
    }

    /// First server (in configuration order) that reports a playable session wins.
    ///
    /// A server that is down is logged and skipped, so a VPS going offline never
    /// stops the local server from being read.
    fn find_session(&self) -> JfResult<Option<(usize, crate::domain::Session)>> {
        let mut failures = 0;

        for (index, source) in self.sources.iter().enumerate() {
            match source.active_session(&self.usernames) {
                Ok(Some(session)) => return Ok(Some((index, session))),
                Ok(None) => {}
                Err(err) => {
                    failures += 1;
                    warn!("Server '{}' unreachable: {}", source.name(), err);
                }
            }
        }

        if failures == self.sources.len() {
            return Err(Box::new(JfError::AllSourcesUnreachable));
        }

        Ok(None)
    }

    fn is_blacklisted(&mut self, index: usize, session: &crate::domain::Session) -> bool {
        if self.blacklist.is_empty() {
            return false;
        }

        if !self.blacklist.library_names.is_empty() {
            self.refresh_libraries(index);
        }

        let resolved = self
            .library_cache
            .get(self.sources[index].name())
            .map(|(libraries, _)| libraries.as_slice())
            .unwrap_or(&[]);

        self.blacklist.blocks(&session.now_playing_item, resolved)
    }

    fn refresh_libraries(&mut self, index: usize) {
        let name = self.sources[index].name().to_string();

        let fresh = self
            .library_cache
            .get(&name)
            .and_then(|(_, at)| SystemTime::now().duration_since(*at).ok())
            .is_some_and(|age| age < LIBRARY_CACHE_TTL);

        if fresh {
            return;
        }

        match self.sources[index].libraries() {
            Ok(libraries) => {
                let resolved = self.blacklist.resolve(libraries.iter());
                debug!(
                    "Resolved {} blacklisted libraries on '{}'",
                    resolved.len(),
                    name
                );
                self.library_cache.insert(name, (resolved, SystemTime::now()));
            }
            Err(err) => warn!("Failed to load libraries from '{}': {}", name, err),
        }
    }

    fn resolve_image(&self, index: usize, session: &crate::domain::Session) -> Option<String> {
        let provider = self.image_provider.as_ref()?;
        let source = self.sources[index].as_ref();

        match provider.image_url(source, session) {
            Ok(url) => Some(url.to_string()),
            Err(err) => {
                debug!("Image provider '{}' failed: {}", provider.name(), err);
                None
            }
        }
    }
}
