//! Environment overlay.
//!
//! Seeds the process environment from a `.env` file, then maps variables onto
//! [`Settings`]. Real environment variables always win over the `.env` file, so
//! a systemd unit or a shell export can override a committed file.
//!
//! Every variable is listed in `.env.example`.

use super::file::parse_hosting;
use super::{paths, CliResult, Settings};
use jellyfin_rpc::{
    ActivityKind, Blacklist, Button, MediaDisplayOptions, MediaType, StatusType,
};
use log::{debug, warn};
use std::env;

/// Highest server index looked for (`JELLYFIN_2_URL` .. `JELLYFIN_9_URL`).
const MAX_EXTRA_SERVERS: usize = 9;

/// Reads the first `.env` found and inserts the variables it does not already have.
///
/// `explicit` skips the search and uses exactly that path.
pub fn load_dotenv(explicit: Option<&str>) {
    let candidates = match explicit {
        Some(path) => vec![std::path::PathBuf::from(path)],
        None => paths::env_file_candidates(),
    };

    for candidate in candidates {
        let Ok(contents) = std::fs::read_to_string(&candidate) else {
            continue;
        };

        debug!("Loading environment from {}", candidate.display());

        for (key, value) in parse(&contents) {
            if env::var_os(&key).is_none() {
                env::set_var(&key, &value);
            }
        }

        return;
    }
}

/// Parses `KEY=value` lines. Supports `#` comments, `export ` prefixes and
/// single or double quoted values.
fn parse(contents: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();

    for line in contents.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line);

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        out.push((key.to_string(), unquote(value.trim())));
    }

    out
}

fn unquote(value: &str) -> String {
    let quoted = |q: char| value.len() >= 2 && value.starts_with(q) && value.ends_with(q);

    if quoted('"') {
        value[1..value.len() - 1].replace("\\n", "\n")
    } else if quoted('\'') {
        value[1..value.len() - 1].to_string()
    } else {
        // An unquoted value ends at the first inline comment.
        value
            .split_once(" #")
            .map(|(head, _)| head.trim())
            .unwrap_or(value)
            .to_string()
    }
}

pub fn apply(settings: &mut Settings) -> CliResult<()> {
    apply_servers(settings);
    apply_runtime(settings);
    apply_discord(settings);
    apply_display(settings);
    apply_images(settings);
    apply_blacklist(settings);
    Ok(())
}

fn apply_servers(settings: &mut Settings) {
    if let Some(usernames) = list("JELLYFIN_USERNAME") {
        settings.usernames = usernames;
    }

    // Slot 0 has no numeric suffix so a single-server setup stays terse.
    apply_server_slot(settings, 0, "JELLYFIN");

    for index in 2..=MAX_EXTRA_SERVERS {
        apply_server_slot(settings, index - 1, &format!("JELLYFIN_{}", index));
    }
}

fn apply_server_slot(settings: &mut Settings, index: usize, prefix: &str) {
    let url = string(&format!("{}_URL", prefix));
    let api_key = string(&format!("{}_API_KEY", prefix));
    let name = string(&format!("{}_NAME", prefix));
    let self_signed = bool_var(&format!("{}_SELF_SIGNED_CERT", prefix));
    // Slot 0's username is the global one, handled in `apply_servers`.
    let usernames = if index == 0 {
        None
    } else {
        list(&format!("{}_USERNAME", prefix))
    };

    if url.is_none() && api_key.is_none() && name.is_none() && usernames.is_none() {
        return;
    }

    let slot = settings.server_slot(index);

    if let Some(url) = url {
        slot.url = url;
    }
    if let Some(api_key) = api_key {
        slot.api_key = api_key;
    }
    if let Some(name) = name {
        slot.name = name;
    }
    if let Some(self_signed) = self_signed {
        slot.self_signed_cert = self_signed;
    }
    if let Some(usernames) = usernames {
        slot.usernames = usernames;
    }
}

fn apply_runtime(settings: &mut Settings) {
    if let Some(interval) = number::<u64>("JELLYFIN_RPC_POLL_INTERVAL") {
        settings.poll_interval_secs = interval.max(1);
    }
    if let Some(level) = string("JELLYFIN_RPC_LOG_LEVEL") {
        settings.log_level = Some(level);
    }
}

fn apply_discord(settings: &mut Settings) {
    if let Some(application_id) = string("DISCORD_APPLICATION_ID") {
        settings.application_id = application_id;
    }
    if let Some(show_paused) = bool_var("DISCORD_SHOW_PAUSED") {
        settings.show_paused = show_paused;
    }
    if let Some(text) = string("DISCORD_LARGE_IMAGE_TEXT") {
        settings.large_image_text = text;
    }

    let buttons: Vec<Button> = (1..=2)
        .filter_map(|slot| {
            let name = string(&format!("DISCORD_BUTTON_{}_NAME", slot))?;
            let url = string(&format!("DISCORD_BUTTON_{}_URL", slot))?;
            Some(Button::new(name, url))
        })
        .collect();

    if !buttons.is_empty() {
        settings.buttons = Some(buttons);
    } else if bool_var("DISCORD_BUTTONS_DYNAMIC") == Some(true) {
        settings.buttons = Some(vec![Button::default(), Button::default()]);
    } else if bool_var("DISCORD_BUTTONS_DYNAMIC") == Some(false) {
        settings.buttons = Some(Vec::new());
    }
}

fn apply_display(settings: &mut Settings) {
    apply_display_category(&mut settings.movies, "RPC_MOVIES");
    apply_display_category(&mut settings.episodes, "RPC_EPISODES");
    apply_display_category(&mut settings.music, "RPC_MUSIC");
}

fn apply_display_category(target: &mut MediaDisplayOptions, prefix: &str) {
    if let Some(details) = string(&format!("{}_DETAILS", prefix)) {
        target.display.details_text = Some(details);
    }
    if let Some(state) = string(&format!("{}_STATE", prefix)) {
        target.display.state_text = Some(state);
    }
    if let Some(image_text) = string(&format!("{}_IMAGE_TEXT", prefix)) {
        target.display.image_text = Some(image_text);
    }
    // Separators carry their own padding (" • "), so this one must not be trimmed.
    if let Some(separator) = raw_string(&format!("{}_SEPARATOR", prefix)) {
        target.separator = separator;
    }
    if let Some(status) = string(&format!("{}_STATUS_TYPE", prefix)) {
        match StatusType::try_from(status.as_str()) {
            Ok(status) => target.status_display_type = status,
            Err(err) => warn!("{}_STATUS_TYPE: {}", prefix, err),
        }
    }
    if let Some(activity) = string(&format!("{}_ACTIVITY_TYPE", prefix)) {
        match ActivityKind::try_from(activity.as_str()) {
            Ok(activity) => target.activity_kind = Some(activity),
            Err(err) => warn!("{}_ACTIVITY_TYPE: {}", prefix, err),
        }
    }
}

fn apply_images(settings: &mut Settings) {
    if let Some(hosting) = string("IMAGES_HOSTING").and_then(|value| parse_hosting(&value)) {
        settings.images.hosting = hosting;
    } else if bool_var("IMAGES_ENABLE") == Some(false) {
        settings.images.hosting = jellyfin_rpc::ImageHosting::Disabled;
    }

    if let Some(client_id) = string("IMGUR_CLIENT_ID") {
        settings.images.imgur_client_id = client_id;
    }
    if let Some(path) = string("IMAGES_CACHE_PATH") {
        settings.images.cache_path = path;
    }
    if let Some(process) = bool_var("IMAGES_PROCESS") {
        settings.images.process = process;
    }
    if let Some(size) = number::<u32>("IMAGES_SIZE") {
        settings.images.processing.size = Some(size);
    }
    if let Some(bg) = bool_var("IMAGES_BG") {
        settings.images.processing.background = bg;
    }
    if let Some(blur) = number::<f32>("IMAGES_BG_BLUR") {
        settings.images.processing.background_blur = blur;
    }
    if let Some(radius) = number::<f32>("IMAGES_CORNER_RADIUS") {
        settings.images.processing.corner_radius = Some(radius);
    }
}

fn apply_blacklist(settings: &mut Settings) {
    let media_types = list("BLACKLIST_MEDIA_TYPES")
        .map(|values| values.iter().map(|v| MediaType::from(v.as_str())).collect());
    let libraries = list("BLACKLIST_LIBRARIES");

    if media_types.is_none() && libraries.is_none() {
        return;
    }

    settings.blacklist = Blacklist::new(
        media_types.unwrap_or_else(|| settings.blacklist.media_types.clone()),
        libraries.unwrap_or_else(|| settings.blacklist.library_names.clone()),
    );
}

fn string(key: &str) -> Option<String> {
    let value = env::var(key).ok()?;
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

/// Like [`string`] but keeps surrounding whitespace. Quote the value in `.env`
/// to make the spaces survive: `RPC_MOVIES_SEPARATOR=" • "`.
fn raw_string(key: &str) -> Option<String> {
    let value = env::var(key).ok()?;
    (!value.trim().is_empty()).then_some(value)
}

fn list(key: &str) -> Option<Vec<String>> {
    let values: Vec<String> = string(key)?
        .split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();

    (!values.is_empty()).then_some(values)
}

fn bool_var(key: &str) -> Option<bool> {
    match string(key)?.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        other => {
            warn!("{} is not a boolean: '{}'", key, other);
            None
        }
    }
}

fn number<T: std::str::FromStr>(key: &str) -> Option<T> {
    match string(key)?.parse() {
        Ok(value) => Some(value),
        Err(_) => {
            warn!("{} is not a number", key);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotenv_syntax() {
        let parsed = parse(
            r#"
# comment
export JELLYFIN_URL=http://localhost:8096
JELLYFIN_API_KEY="secret key"
RPC_MOVIES_STATE='{year} {sep} {director}'
IMAGES_SIZE=512 # inline comment
"#,
        );

        assert_eq!(
            parsed,
            vec![
                ("JELLYFIN_URL".into(), "http://localhost:8096".to_string()),
                ("JELLYFIN_API_KEY".into(), "secret key".to_string()),
                (
                    "RPC_MOVIES_STATE".into(),
                    "{year} {sep} {director}".to_string()
                ),
                ("IMAGES_SIZE".into(), "512".to_string()),
            ]
        );
    }
}
