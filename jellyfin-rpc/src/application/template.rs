use crate::domain::{MediaType, Session};
use crate::VERSION;

/// Expands `{placeholder}` templates against a session.
///
/// Pure: no I/O, no clock, no network. Everything the templates can reference is
/// already inside the [`Session`].
pub struct TemplateRenderer<'a> {
    session: &'a Session,
    separator: &'a str,
}

impl<'a> TemplateRenderer<'a> {
    pub fn new(session: &'a Session, separator: &'a str) -> Self {
        Self { session, separator }
    }

    /// Renders a template for whatever media type the session is playing.
    pub fn render(&self, input: &str) -> String {
        match self.session.now_playing_item.media_type {
            MediaType::Music | MediaType::AudioBook => self.render_music(input),
            MediaType::Episode => self.render_episode(input),
            MediaType::Movie => self.render_movie(input),
            _ => self.finish(self.render_common(input)),
        }
    }

    pub fn render_music(&self, input: &str) -> String {
        let item = &self.session.now_playing_item;

        let result = self
            .render_common(input)
            .replace("{track}", &item.name)
            .replace("{album}", item.album.as_deref().unwrap_or(""))
            .replace("{artists}", &self.session.format_artists());

        self.finish(result)
    }

    pub fn render_movie(&self, input: &str) -> String {
        let item = &self.session.now_playing_item;

        let critic_score = item
            .critic_rating
            .map(|s| format!("🍅 {}/100", s))
            .unwrap_or_default();
        let community_score = item
            .community_rating
            .map(|s| format!("⭐ {:.1}/10", s))
            .unwrap_or_default();
        let duration_minutes = item
            .duration_minutes()
            .map(|m| format!("{} Minutes", m))
            .unwrap_or_default();

        let result = self
            .render_common(input)
            .replace("{critic-score}", &critic_score)
            .replace("{community-score}", &community_score)
            .replace("{director}", item.director().unwrap_or(""))
            .replace("{duration-minutes}", &duration_minutes);

        self.finish(result)
    }

    pub fn render_episode(&self, input: &str) -> String {
        let item = &self.session.now_playing_item;

        let season = item.parent_index_number.unwrap_or(0);

        // One Jellyfin episode can span several actual episodes (E01-03 in one file).
        let episode_range = (item.index_number.unwrap_or(0), item.index_number_end);
        let episode = match episode_range {
            (first, Some(last)) => format!("{}-{}", first, last),
            (episode, None) => format!("{}", episode),
        };
        let episode_padded = match episode_range {
            (first, Some(last)) => format!("{:02}-{:02}", first, last),
            (episode, None) => format!("{:02}", episode),
        };

        let result = self
            .render_common(input)
            .replace("{show-title}", item.series_name.as_deref().unwrap_or(""))
            .replace("{studio}", item.series_studio.as_deref().unwrap_or(""))
            .replace("{episode}", &episode)
            .replace("{episode-padded}", &episode_padded)
            .replace("{season}", &season.to_string())
            .replace("{season-padded}", &format!("{:02}", season));

        self.finish(result)
    }

    /// Placeholders every media type understands.
    fn render_common(&self, input: &str) -> String {
        let item = &self.session.now_playing_item;

        let year = item
            .production_year
            .map(|y| y.to_string())
            .unwrap_or_default();

        input
            .trim()
            .replace("{title}", &item.name)
            .replace("{original-title}", item.original_title.as_deref().unwrap_or(""))
            .replace("{genres}", &item.genres_joined())
            .replace("{year}", &year)
            .replace("{version}", VERSION.unwrap_or("UNKNOWN"))
            // session context — handy for telling a phone session apart from a browser one
            .replace("{server}", &self.session.source_name)
            .replace("{username}", &self.session.user_name)
            .replace(
                "{client}",
                self.session.device.client.as_deref().unwrap_or(""),
            )
            .replace(
                "{device}",
                self.session.device.device_name.as_deref().unwrap_or(""),
            )
    }

    fn finish(&self, result: String) -> String {
        Self::sanitize(&result).replace("{sep}", self.separator)
    }

    /// Collapses whitespace and drops separators left dangling by empty placeholders.
    pub fn sanitize(input: &str) -> String {
        let mut result = input.split_whitespace().collect::<Vec<&str>>().join(" ");

        // The separator carries its own padding (" • "), so spaces written around
        // `{sep}` in a template would otherwise double up.
        while result.contains(" {sep}") || result.contains("{sep} ") {
            result = result.replace(" {sep}", "{sep}").replace("{sep} ", "{sep}");
        }

        while result.contains("{sep}{sep}") || result.contains("{sep} {sep}") {
            result = result.replace("{sep}{sep}", "{sep}");
            result = result.replace("{sep} {sep}", "{sep}");
        }

        while result.starts_with("{sep}") {
            result = result
                .drain(5..)
                .collect::<String>()
                .trim_start()
                .to_string();
        }

        while result.ends_with("{sep}") {
            result = result
                .drain(..result.len() - 5)
                .collect::<String>()
                .trim_end()
                .to_string();
        }

        result
    }
}
