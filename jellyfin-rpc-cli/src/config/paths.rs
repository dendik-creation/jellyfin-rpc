use std::env;
use std::path::PathBuf;

/// Directory holding `main.json`, `urls.json` and (optionally) `.env`.
///
/// Windows: `%APPDATA%\jellyfin-rpc`
/// Linux/macOS: `$XDG_CONFIG_HOME/jellyfin-rpc`, falling back to `~/.config/jellyfin-rpc`
pub fn config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if cfg!(windows) {
        Ok(PathBuf::from(env::var("APPDATA")?).join("jellyfin-rpc"))
    } else {
        let base = match env::var("XDG_CONFIG_HOME") {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => PathBuf::from(env::var("HOME")?).join(".config"),
        };
        Ok(base.join("jellyfin-rpc"))
    }
}

pub fn config_path() -> Result<String, Box<dyn std::error::Error>> {
    Ok(config_dir()?.join("main.json").to_string_lossy().into_owned())
}

pub fn urls_path() -> Result<String, Box<dyn std::error::Error>> {
    Ok(config_dir()?.join("urls.json").to_string_lossy().into_owned())
}
