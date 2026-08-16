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

/// Every place a `.env` is looked for, in priority order.
///
/// The working directory comes first so a per-checkout `.env` beats the global one.
pub fn env_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join(".env"));
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(".env"));
        }
    }

    if let Ok(dir) = config_dir() {
        candidates.push(dir.join(".env"));
    }

    candidates
}
