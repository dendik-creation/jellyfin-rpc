//! Composition root helper.
//!
//! Knows every layer, so nothing else has to. Frontends describe *what* they
//! want (servers, display formats, image hosting) and get back a wired
//! [`PresenceService`].

use crate::application::{PresenceConfig, PresenceService};
use crate::domain::{Blacklist, Button, JfError, MediaDisplayOptions};
use crate::infrastructure::images::ChainImageProvider;
use crate::infrastructure::{
    DirectImageProvider, DiscordIpcSink, ImageProcessingOptions, ImgurImageProvider,
    JellyfinHttpSource, LitterboxImageProvider, ServerConfig, DEFAULT_APPLICATION_ID,
};
use crate::JfResult;

/// Where artwork shown in Discord comes from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ImageHosting {
    /// No artwork; Discord shows the built-in Jellyfin logo.
    #[default]
    Disabled,
    /// Link straight to the Jellyfin server. Requires the server to be reachable
    /// from the public internet.
    Direct,
    /// Upload to imgur, fall back to a direct link.
    Imgur,
    /// Upload to litterbox.catbox.moe, fall back to a direct link.
    Litterbox,
}

pub struct RpcBuilder {
    servers: Vec<ServerConfig>,
    usernames: Vec<String>,
    application_id: String,
    presence: PresenceConfig,
    blacklist: Blacklist,
    image_hosting: ImageHosting,
    imgur_client_id: String,
    image_cache_path: String,
    process_images: bool,
    image_processing: ImageProcessingOptions,
}

impl Default for RpcBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcBuilder {
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
            usernames: Vec::new(),
            application_id: DEFAULT_APPLICATION_ID.to_string(),
            presence: PresenceConfig::default(),
            blacklist: Blacklist::default(),
            image_hosting: ImageHosting::Disabled,
            imgur_client_id: String::new(),
            image_cache_path: String::new(),
            process_images: true,
            image_processing: ImageProcessingOptions::default(),
        }
    }

    /// Adds a Jellyfin server. Servers are polled in the order they were added
    /// and the first one with a playing session wins.
    pub fn add_server(&mut self, server: ServerConfig) -> &mut Self {
        self.servers.push(server);
        self
    }

    pub fn servers(&mut self, servers: Vec<ServerConfig>) -> &mut Self {
        self.servers = servers;
        self
    }

    /// Accounts to watch, in priority order. Applies to every server that does
    /// not carry its own list.
    pub fn usernames(&mut self, usernames: Vec<String>) -> &mut Self {
        self.usernames = usernames;
        self
    }

    pub fn username<T: Into<String>>(&mut self, username: T) -> &mut Self {
        self.usernames = vec![username.into()];
        self
    }

    /// Discord application id. The application's *name* is what Discord prints
    /// after "Watching"; register your own application to change it.
    pub fn application_id<T: Into<String>>(&mut self, application_id: T) -> &mut Self {
        let application_id = application_id.into();
        if !application_id.is_empty() {
            self.application_id = application_id;
        }
        self
    }

    pub fn music(&mut self, options: MediaDisplayOptions) -> &mut Self {
        self.presence.music = options;
        self
    }

    pub fn movies(&mut self, options: MediaDisplayOptions) -> &mut Self {
        self.presence.movies = options;
        self
    }

    pub fn episodes(&mut self, options: MediaDisplayOptions) -> &mut Self {
        self.presence.episodes = options;
        self
    }

    pub fn show_paused(&mut self, val: bool) -> &mut Self {
        self.presence.show_paused = val;
        self
    }

    pub fn buttons(&mut self, buttons: Vec<Button>) -> &mut Self {
        self.presence.buttons = Some(buttons);
        self
    }

    pub fn large_image_text<T: Into<String>>(&mut self, text: T) -> &mut Self {
        self.presence.large_image_text = text.into();
        self
    }

    pub fn blacklist(&mut self, blacklist: Blacklist) -> &mut Self {
        self.blacklist = blacklist;
        self
    }

    pub fn image_hosting(&mut self, hosting: ImageHosting) -> &mut Self {
        self.image_hosting = hosting;
        self
    }

    pub fn imgur_client_id<T: Into<String>>(&mut self, client_id: T) -> &mut Self {
        self.imgur_client_id = client_id.into();
        self
    }

    /// Where uploaded image urls are remembered, so the same artwork is uploaded once.
    pub fn image_cache_path<T: Into<String>>(&mut self, path: T) -> &mut Self {
        self.image_cache_path = path.into();
        self
    }

    /// Resize artwork to a 9:16 canvas before uploading it.
    pub fn process_images(&mut self, val: bool) -> &mut Self {
        self.process_images = val;
        self
    }

    pub fn image_processing(&mut self, options: ImageProcessingOptions) -> &mut Self {
        self.image_processing = options;
        self
    }

    pub fn build(self) -> JfResult<PresenceService> {
        if self.servers.is_empty() {
            return Err(Box::new(JfError::NoSources));
        }

        // Every server needs someone to watch: either the global list or its own.
        if self.usernames.is_empty() && self.servers.iter().any(|s| s.usernames.is_empty()) {
            return Err(Box::new(JfError::MissingRequiredValues));
        }

        let image_provider = self.build_image_provider()?;

        let mut sources: Vec<Box<dyn crate::application::MediaSource>> = Vec::new();
        for server in self.servers {
            sources.push(Box::new(JellyfinHttpSource::new(server)?));
        }

        let sink = Box::new(DiscordIpcSink::new(&self.application_id));

        PresenceService::new(
            sources,
            sink,
            image_provider,
            self.presence,
            self.usernames,
            self.blacklist,
        )
    }

    fn build_image_provider(
        &self,
    ) -> JfResult<Option<Box<dyn crate::application::ImageProvider>>> {
        let processing = self
            .process_images
            .then(|| self.image_processing.clone());

        // Direct links keep the artwork alive when an upload fails, so every
        // hosting mode except `Disabled` ends with it.
        let direct = || -> Box<dyn crate::application::ImageProvider> {
            Box::new(DirectImageProvider::new(self.image_processing.size))
        };

        let providers: Vec<Box<dyn crate::application::ImageProvider>> = match self.image_hosting {
            ImageHosting::Disabled => return Ok(None),
            ImageHosting::Direct => vec![direct()],
            ImageHosting::Imgur => vec![
                Box::new(ImgurImageProvider::new(
                    self.imgur_client_id.clone(),
                    self.image_cache_path.clone(),
                    processing,
                )?),
                direct(),
            ],
            ImageHosting::Litterbox => vec![
                Box::new(LitterboxImageProvider::new(
                    self.image_cache_path.clone(),
                    processing,
                )?),
                direct(),
            ],
        };

        Ok(Some(Box::new(ChainImageProvider::new(providers))))
    }
}
