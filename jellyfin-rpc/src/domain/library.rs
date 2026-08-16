use super::media::{MediaType, NowPlayingItem};

/// A Jellyfin library ("virtual folder") and the paths it maps to on disk.
#[derive(Debug, Clone)]
pub struct Library {
    pub name: Option<String>,
    pub locations: Vec<String>,
}

/// Rules deciding whether an item should be hidden from Discord.
#[derive(Debug, Clone, Default)]
pub struct Blacklist {
    pub media_types: Vec<MediaType>,
    pub library_names: Vec<String>,
}

impl Blacklist {
    pub fn new(media_types: Vec<MediaType>, library_names: Vec<String>) -> Self {
        Self {
            media_types,
            library_names,
        }
    }

    /// Whether the item is blocked, given the libraries already resolved for its server.
    ///
    /// `resolved` is the subset of the server's libraries whose names appear in
    /// [`Blacklist::library_names`]; it is empty when the library list could not be fetched.
    pub fn blocks(&self, item: &NowPlayingItem, resolved: &[Library]) -> bool {
        if self.media_types.contains(&item.media_type) {
            return true;
        }

        let item_path = item.path.as_deref().unwrap_or("");
        if item_path.is_empty() {
            return false;
        }

        resolved.iter().any(|library| {
            library
                .locations
                .iter()
                .any(|location| item_path.starts_with(location))
        })
    }

    /// Filters a server's full library list down to the blacklisted ones.
    pub fn resolve<'a>(&self, libraries: impl IntoIterator<Item = &'a Library>) -> Vec<Library> {
        libraries
            .into_iter()
            .filter(|library| {
                library
                    .name
                    .as_ref()
                    .is_some_and(|name| self.library_names.contains(name))
            })
            .cloned()
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.media_types.is_empty() && self.library_names.is_empty()
    }
}
