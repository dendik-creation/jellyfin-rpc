use std::{error::Error, fmt::Display};

/// Error type
#[derive(Debug)]
pub enum JfError {
    /// MediaType returned from jellyfin is of type None,
    /// this should be reported on github
    UnrecognizedMediaType,
    /// Content is in blacklist
    ContentBlacklist,
    /// Builder was missing url / api key / username
    MissingRequiredValues,
    /// Media has no primary image
    NoImage,
    /// No media source (jellyfin server) was configured
    NoSources,
    /// All configured media sources failed to answer
    AllSourcesUnreachable,
}

impl Error for JfError {}

impl Display for JfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JfError::MissingRequiredValues => write!(f, "missing required values to build client"),
            JfError::UnrecognizedMediaType => write!(f, "unrecognized media type"),
            JfError::ContentBlacklist => write!(f, "content is blacklisted"),
            JfError::NoImage => write!(f, "media does not have an image"),
            JfError::NoSources => write!(f, "no jellyfin server configured"),
            JfError::AllSourcesUnreachable => {
                write!(f, "every configured jellyfin server failed to respond")
            }
        }
    }
}
