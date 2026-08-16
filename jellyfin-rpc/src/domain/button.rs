use serde::{Deserialize, Serialize};

/// Contains information about buttons displayed in Discord
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Button {
    /// What the name should be showed as in Discord.
    ///
    /// # Example
    /// `"My personal website!"`
    pub name: String,
    /// What clicking it should point to in Discord.
    ///
    /// # Example
    /// `"https://example.com"`
    pub url: String,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            name: String::from("dynamic"),
            url: String::from("dynamic"),
        }
    }
}

impl Button {
    /// Creates a new button with the supplied name and url.
    ///
    /// # Example
    /// ```
    /// use jellyfin_rpc::Button;
    ///
    /// let name = "My personal website!".to_string();
    /// let url = "https://example.com".to_string();
    ///
    /// let button = Button::new(name, url);
    /// ```
    pub fn new(name: String, url: String) -> Self {
        Self { name, url }
    }

    /// A `dynamic` button is a placeholder that gets replaced by whatever
    /// external url Jellyfin reports for the playing item.
    pub fn is_dynamic(&self) -> bool {
        self.name == "dynamic" && self.url == "dynamic"
    }
}
