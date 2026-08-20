use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::core::config_manager::config_dir;
use crate::models::rom_entry::RomEntry;

/// Bumped whenever the persisted shape changes (e.g. a new `RomEntry`
/// field). A cache file written by an older/incompatible version is simply
/// ignored — never misread — falling through to a real re-parse.
const CACHE_FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct PersistedCache {
    format_version: u32,
    path: PathBuf,
    size: u64,
    modified_unix_nanos: u128,
    discriminator: String,
    entries: HashMap<String, RomEntry>,
}

fn cache_file_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("dat_cache.json")
}

fn unix_nanos(time: SystemTime) -> Option<u128> {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos())
}

/// Returns the cached reference database for `path` if a persisted cache
/// file exists, matches the current format version, and its recorded
/// path/size/modified-time/discriminator all match `path` and
/// `discriminator` exactly. Any mismatch, missing file, or read/parse
/// failure is treated as a cache miss (`None`) rather than an error —
/// deleting the cache file, or it being from an incompatible version, must
/// always be safe and simply force a real re-parse.
pub fn load_if_matching(path: &Path, discriminator: &str) -> Option<HashMap<String, RomEntry>> {
    load_if_matching_in(&config_dir(), path, discriminator)
}

pub fn load_if_matching_in(
    cache_dir: &Path,
    path: &Path,
    discriminator: &str,
) -> Option<HashMap<String, RomEntry>> {
    let metadata = fs::metadata(path).ok()?;
    let modified = unix_nanos(metadata.modified().ok()?)?;

    let content = fs::read_to_string(cache_file_path(cache_dir)).ok()?;
    let cached: PersistedCache = serde_json::from_str(&content).ok()?;

    let matches = cached.format_version == CACHE_FORMAT_VERSION
        && cached.path == path
        && cached.size == metadata.len()
        && cached.modified_unix_nanos == modified
        && cached.discriminator == discriminator;

    if matches {
        tracing::info!(path = %path.display(), "reference database loaded from persistent cache");
        Some(cached.entries)
    } else {
        None
    }
}

/// Persists `entries` to disk so the next app start can skip re-parsing
/// `path` if it hasn't changed. Best-effort: any failure to fingerprint the
/// source file, serialize, or write is logged and otherwise ignored — a
/// failed cache write must never fail the scan that produced the data.
pub fn save(path: &Path, discriminator: &str, entries: &HashMap<String, RomEntry>) {
    save_in(&config_dir(), path, discriminator, entries);
}

pub fn save_in(cache_dir: &Path, path: &Path, discriminator: &str, entries: &HashMap<String, RomEntry>) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    let Some(modified) = metadata.modified().ok().and_then(unix_nanos) else {
        return;
    };

    let cached = PersistedCache {
        format_version: CACHE_FORMAT_VERSION,
        path: path.to_path_buf(),
        size: metadata.len(),
        modified_unix_nanos: modified,
        discriminator: discriminator.to_string(),
        entries: entries.clone(),
    };

    let Ok(content) = serde_json::to_string(&cached) else {
        return;
    };

    if fs::create_dir_all(cache_dir).is_err() {
        return;
    }
    if let Err(err) = fs::write(cache_file_path(cache_dir), content) {
        tracing::warn!(error = %err, "failed to write persistent DAT cache (non-fatal)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rom_entry::DriverStatus;

    fn sample_entry(name: &str) -> RomEntry {
        RomEntry {
            name: name.to_string(),
            description: String::new(),
            year: String::new(),
            manufacturer: String::new(),
            clone_of: None,
            rom_of: None,
            is_bios: false,
            is_device: false,
            is_mechanical: false,
            runnable: true,
            driver_status: DriverStatus::Good,
            category: None,
            languages: Vec::new(),
            roms: Vec::new(),
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mameset_cleaner_dat_cache_test_{name}_{}_{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn saves_and_loads_a_matching_cache() {
        let dir = temp_dir("hit");
        let dat_path = dir.join("sample.dat");
        fs::write(&dat_path, "irrelevant content").unwrap();

        let mut entries = HashMap::new();
        entries.insert("pacman".to_string(), sample_entry("pacman"));

        let cache_dir = dir.join("cache");
        save_in(&cache_dir, &dat_path, "mame", &entries);

        let loaded = load_if_matching_in(&cache_dir, &dat_path, "mame");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().len(), 1);
    }

    #[test]
    fn a_changed_source_file_is_not_matched() {
        let dir = temp_dir("changed");
        let dat_path = dir.join("sample.dat");
        fs::write(&dat_path, "version one").unwrap();

        let entries = HashMap::new();
        let cache_dir = dir.join("cache");
        save_in(&cache_dir, &dat_path, "mame", &entries);

        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&dat_path, "version two, much longer content").unwrap();

        assert!(load_if_matching_in(&cache_dir, &dat_path, "mame").is_none());
    }

    #[test]
    fn a_different_discriminator_is_not_matched() {
        let dir = temp_dir("discriminator");
        let dat_path = dir.join("sample.dat");
        fs::write(&dat_path, "content").unwrap();

        let entries = HashMap::new();
        let cache_dir = dir.join("cache");
        save_in(&cache_dir, &dat_path, "mame", &entries);

        assert!(load_if_matching_in(&cache_dir, &dat_path, "psx").is_none());
    }

    #[test]
    fn a_missing_cache_file_is_a_clean_miss_not_an_error() {
        let dir = temp_dir("missing");
        let dat_path = dir.join("sample.dat");
        fs::write(&dat_path, "content").unwrap();

        let cache_dir = dir.join("cache-that-does-not-exist");
        assert!(load_if_matching_in(&cache_dir, &dat_path, "mame").is_none());
    }

    #[test]
    fn a_cache_file_from_an_incompatible_format_version_is_ignored() {
        let dir = temp_dir("format-version");
        let dat_path = dir.join("sample.dat");
        fs::write(&dat_path, "content").unwrap();
        let metadata = fs::metadata(&dat_path).unwrap();

        let stale = PersistedCache {
            format_version: CACHE_FORMAT_VERSION + 1,
            path: dat_path.clone(),
            size: metadata.len(),
            modified_unix_nanos: unix_nanos(metadata.modified().unwrap()).unwrap(),
            discriminator: "mame".to_string(),
            entries: HashMap::new(),
        };

        let cache_dir = dir.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(
            cache_file_path(&cache_dir),
            serde_json::to_string(&stale).unwrap(),
        )
        .unwrap();

        assert!(load_if_matching_in(&cache_dir, &dat_path, "mame").is_none());
    }
}
