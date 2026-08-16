use crate::JfResult;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

/// One remembered upload. `timestamp` is absent in caches written by older
/// versions, which is why it is optional.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct CacheEntry {
    id: String,
    url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
}

/// A JSON file mapping Jellyfin item ids to already uploaded image urls.
///
/// Avoids re-uploading the same artwork on every poll.
pub struct UrlCache {
    path: PathBuf,
    /// How long an entry stays valid. `None` means forever (imgur).
    ttl_hours: Option<i64>,
}

impl UrlCache {
    pub fn new<P: Into<PathBuf>>(path: P, ttl_hours: Option<i64>) -> Self {
        Self {
            path: path.into(),
            ttl_hours,
        }
    }

    pub fn get(&self, id: &str) -> Option<Url> {
        let entries = self.load().ok()?;
        let entry = entries.iter().find(|entry| entry.id == id)?;

        if self.is_expired(entry) {
            debug!("Cached image for {} expired, dropping it", id);
            let kept: Vec<CacheEntry> = entries.iter().filter(|e| e.id != id).cloned().collect();
            let _ = self.save(&kept);
            return None;
        }

        Url::parse(&entry.url).ok()
    }

    pub fn put(&self, id: &str, url: &Url) -> JfResult<()> {
        let mut entries = self.load().unwrap_or_default();
        entries.retain(|entry| entry.id != id);
        entries.push(CacheEntry {
            id: id.to_string(),
            url: url.to_string(),
            timestamp: Some(now_secs().to_string()),
        });
        self.save(&entries)
    }

    fn is_expired(&self, entry: &CacheEntry) -> bool {
        let Some(ttl_hours) = self.ttl_hours else {
            return false;
        };

        let Some(written) = entry.timestamp.as_ref().and_then(|t| t.parse::<i64>().ok()) else {
            // No timestamp means we cannot prove it is fresh; treat it as expired.
            return true;
        };

        (now_secs() - written) / 3600 >= ttl_hours
    }

    fn load(&self) -> JfResult<Vec<CacheEntry>> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => Ok(serde_json::from_str(&raw).unwrap_or_default()),
            Err(_) => Ok(Vec::new()),
        }
    }

    fn save(&self, entries: &[CacheEntry]) -> JfResult<()> {
        if let Some(parent) = Path::new(&self.path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, serde_json::to_string(entries)?)?;
        Ok(())
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
