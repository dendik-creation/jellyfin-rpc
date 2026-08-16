use super::template::TemplateRenderer;
use crate::domain::{
    ActivityKind, Button, MediaDisplayOptions, MediaType, Presence, PresenceAssets,
    PresenceTimestamps, Session,
};
use crate::JfResult;
use crate::VERSION;

pub const DEFAULT_IMAGE: &str = "https://i.imgur.com/oX6vcds.png";
pub const DEFAULT_LIVE_TV_IMAGE: &str = "https://i.imgur.com/XxdHOqm.png";
pub const DEFAULT_PAUSED_IMAGE: &str = "https://i.imgur.com/wlHSvYy.png";

/// Everything the user can tune about how activities look.
#[derive(Debug, Clone)]
pub struct PresenceConfig {
    pub music: MediaDisplayOptions,
    pub movies: MediaDisplayOptions,
    pub episodes: MediaDisplayOptions,
    /// Keep showing the activity while playback is paused.
    pub show_paused: bool,
    /// `None` means "generate buttons from the item's external urls".
    pub buttons: Option<Vec<Button>>,
    /// Fallback for the large image hover text when the template renders empty.
    pub large_image_text: String,
    pub default_image: String,
    pub live_tv_image: String,
    pub paused_image: String,
    pub paused_text: String,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            music: MediaDisplayOptions::default(),
            movies: MediaDisplayOptions::default(),
            episodes: MediaDisplayOptions::new(crate::domain::DisplayFormat::from(
                crate::domain::EpisodeDisplayOptions {
                    divider: true,
                    prefix: true,
                    simple: false,
                },
            )),
            show_paused: true,
            buttons: None,
            large_image_text: String::new(),
            default_image: DEFAULT_IMAGE.to_string(),
            live_tv_image: DEFAULT_LIVE_TV_IMAGE.to_string(),
            paused_image: DEFAULT_PAUSED_IMAGE.to_string(),
            paused_text: "Paused".to_string(),
        }
    }
}

/// Turns a [`Session`] plus a resolved artwork URL into a [`Presence`].
///
/// Pure apart from the clock read inside [`Session::get_time`].
pub struct PresenceBuilder<'a> {
    config: &'a PresenceConfig,
}

impl<'a> PresenceBuilder<'a> {
    pub fn new(config: &'a PresenceConfig) -> Self {
        Self { config }
    }

    /// Returns `Ok(None)` when the session should not be shown at all
    /// (paused while `show_paused` is off).
    pub fn build(&self, session: &Session, image_url: Option<&str>) -> JfResult<Option<Presence>> {
        let options = self.options_for(session.now_playing_item.media_type);
        let renderer = TemplateRenderer::new(session, &options.separator);

        let large_image = match session.now_playing_item.media_type {
            MediaType::LiveTv => self.config.live_tv_image.clone(),
            _ => image_url
                .map(str::to_string)
                .unwrap_or_else(|| self.config.default_image.clone()),
        };

        let mut assets = PresenceAssets {
            large_image,
            large_text: self.image_text(session, &renderer, options),
            small_image: None,
            small_text: None,
        };

        let timestamps = match session.get_time()? {
            crate::domain::PlayTime::Some(start, end) => Some(PresenceTimestamps { start, end }),
            crate::domain::PlayTime::None => None,
            crate::domain::PlayTime::Paused if self.config.show_paused => {
                assets.small_image = Some(self.config.paused_image.clone());
                assets.small_text = Some(self.config.paused_text.clone());
                None
            }
            crate::domain::PlayTime::Paused => return Ok(None),
        };

        let mut presence = Presence {
            details: self.details(session, &renderer, options),
            state: self.state(session, &renderer, options),
            assets,
            timestamps,
            buttons: self.buttons(session),
            activity_kind: self.activity_kind(session, options),
            status_display_type: options.status_display_type,
        };

        presence.clamp_text_fields();

        Ok(Some(presence))
    }

    fn options_for(&self, media_type: MediaType) -> &MediaDisplayOptions {
        match media_type {
            MediaType::Music | MediaType::AudioBook => &self.config.music,
            MediaType::Episode => &self.config.episodes,
            _ => &self.config.movies,
        }
    }

    fn activity_kind(&self, session: &Session, options: &MediaDisplayOptions) -> ActivityKind {
        if let Some(kind) = options.activity_kind {
            return kind;
        }

        match session.now_playing_item.media_type {
            MediaType::Music | MediaType::AudioBook => ActivityKind::Listening,
            MediaType::Book => ActivityKind::Playing,
            _ => ActivityKind::Watching,
        }
    }

    fn details(
        &self,
        session: &Session,
        renderer: &TemplateRenderer,
        options: &MediaDisplayOptions,
    ) -> String {
        let item = &session.now_playing_item;

        match item.media_type {
            MediaType::Music => {
                renderer.render_music(&options.display.details_or("{track}").replace("{__default}", "{track}"))
            }
            MediaType::Movie => {
                renderer.render_movie(&options.display.details_or("{title}").replace("{__default}", "{title}"))
            }
            MediaType::Episode => renderer.render_episode(
                &options
                    .display
                    .details_or("{show-title}")
                    .replace("{__default}", "{show-title}"),
            ),
            MediaType::AudioBook => item
                .album
                .clone()
                .unwrap_or_else(|| item.name.clone()),
            _ => item.name.clone(),
        }
    }

    fn state(
        &self,
        session: &Session,
        renderer: &TemplateRenderer,
        options: &MediaDisplayOptions,
    ) -> String {
        let item = &session.now_playing_item;

        match item.media_type {
            MediaType::Episode => {
                renderer.render_episode(&options.display.state_or("").replace("{__default}", ""))
            }
            MediaType::LiveTv => "Live TV".to_string(),
            MediaType::Music => renderer.render_music(
                &options
                    .display
                    .state_or("By {artists}")
                    .replace("{__default}", "By {artists} {sep} "),
            ),
            MediaType::Movie => {
                renderer.render_movie(&options.display.state_or("").replace("{__default}", ""))
            }
            MediaType::Book => {
                // Jellyfin reports book progress as ticks, 10000 ticks per page.
                match session.play_state.position_ticks {
                    Some(ticks) => format!("Reading page {}", ticks / 10000),
                    None => String::new(),
                }
            }
            MediaType::AudioBook => {
                let artists = session.format_artists();
                let genres = item.genres_joined();

                let mut state = String::new();
                if !artists.is_empty() {
                    state += &format!("By {}", artists);
                }
                if !state.is_empty() && !genres.is_empty() {
                    state += " - ";
                }
                state += &genres;
                state
            }
            _ => item.genres_joined(),
        }
    }

    fn image_text(
        &self,
        session: &Session,
        renderer: &TemplateRenderer,
        options: &MediaDisplayOptions,
    ) -> String {
        let rendered = match session.now_playing_item.media_type {
            MediaType::Music | MediaType::AudioBook => {
                renderer.render_music(&options.display.image_or(""))
            }
            MediaType::Movie => renderer.render_movie(&options.display.image_or("")),
            MediaType::Episode => renderer.render_episode(&options.display.image_or("")),
            _ => String::new(),
        };

        if !rendered.is_empty() {
            return rendered;
        }

        if !self.config.large_image_text.is_empty() {
            return self.config.large_image_text.clone();
        }

        format!("Jellyfin-RPC v{}", VERSION.unwrap_or("UNKNOWN"))
    }

    /// At most two buttons: configured ones first, `dynamic` slots filled from
    /// the item's external urls. Localhost urls are dropped, Discord can't open them.
    fn buttons(&self, session: &Session) -> Vec<Button> {
        let external: Vec<&crate::domain::ExternalUrl> = session
            .now_playing_item
            .external_urls
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .filter(|eu| !eu.url.starts_with("http://localhost"))
            .filter(|eu| !eu.url.starts_with("https://localhost"))
            .collect();

        let Some(configured) = self.config.buttons.as_ref() else {
            return external
                .iter()
                .take(2)
                .map(|eu| Button::new(eu.name.clone(), eu.url.clone()))
                .collect();
        };

        let mut out = Vec::new();
        let mut next_external = 0;

        for button in configured {
            if out.len() == 2 {
                break;
            }

            if button.is_dynamic() {
                if let Some(eu) = external.get(next_external) {
                    out.push(Button::new(eu.name.clone(), eu.url.clone()));
                    next_external += 1;
                }
            } else {
                out.push(button.clone());
            }
        }

        out
    }
}
