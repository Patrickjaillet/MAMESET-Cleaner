use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::config_manager::config_dir;

use super::github_client::{self, GitHubClientError};

/// A plugin manifest as stored locally (JSON) and downloaded from the
/// repository. Kept separate from `plugin_interface::PluginManifest` (whose
/// fields use FFI-safe `abi_stable` string types) so this module has no
/// dependency on the plugin ABI crate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredManifest {
    pub id: String,
    pub name: String,
    pub emulator_frontend: String,
    pub console_family: String,
    pub version: String,
    pub dat_format: String,
    pub sha256: String,
    pub min_app_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginStatus {
    NotInstalled,
    Installed,
    UpdateAvailable,
}

pub fn default_plugins_dir() -> PathBuf {
    config_dir().join("plugins")
}

fn manifest_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

pub fn plugin_dll_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}{}", std::env::consts::DLL_SUFFIX))
}

/// Lists every plugin manifest saved in `dir`. Missing or unreadable files
/// are silently skipped rather than failing the whole listing.
pub fn list_installed(dir: &Path) -> Vec<StoredManifest> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut manifests: Vec<StoredManifest> = entries
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("json"))
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|content| serde_json::from_str(&content).ok())
        .collect();

    manifests.sort_by(|a, b| a.id.cmp(&b.id));
    manifests
}

fn save_manifest(dir: &Path, manifest: &StoredManifest) -> Result<(), GitHubClientError> {
    fs::create_dir_all(dir)?;
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|err| GitHubClientError::Json(err.to_string()))?;
    fs::write(manifest_path(dir, &manifest.id), content)?;
    Ok(())
}

pub fn compare_status(installed: Option<&StoredManifest>, remote_version: &str) -> PluginStatus {
    match installed {
        None => PluginStatus::NotInstalled,
        Some(local) if local.version != remote_version => PluginStatus::UpdateAvailable,
        Some(_) => PluginStatus::Installed,
    }
}

/// Installs a plugin from already-downloaded bytes: writes the `.dll`,
/// verifies it against `manifest.sha256`, and only saves the manifest (thus
/// making the plugin "detected") if the check passes. On failure the
/// partially written `.dll` is removed so a rejected file never lingers as
/// if it were installed.
pub fn install_plugin_from_bytes(
    dir: &Path,
    bytes: &[u8],
    manifest: &StoredManifest,
) -> Result<(), GitHubClientError> {
    fs::create_dir_all(dir)?;
    let dll_path = plugin_dll_path(dir, &manifest.id);
    fs::write(&dll_path, bytes)?;

    if let Err(err) = github_client::verify_sha256(&dll_path, &manifest.sha256) {
        let _ = fs::remove_file(&dll_path);
        return Err(err);
    }

    save_manifest(dir, manifest)
}

/// Downloads a plugin `.dll` from `download_url` and installs it, verifying
/// its integrity against `manifest.sha256` before it is considered
/// installed.
pub fn install_plugin_from_url(
    dir: &Path,
    download_url: &str,
    manifest: &StoredManifest,
) -> Result<(), GitHubClientError> {
    install_plugin_from_url_with_progress(dir, download_url, manifest, |_, _| {})
}

/// Same as [`install_plugin_from_url`], but reports download progress
/// through `on_progress(bytes_downloaded, total_bytes)`.
pub fn install_plugin_from_url_with_progress(
    dir: &Path,
    download_url: &str,
    manifest: &StoredManifest,
    on_progress: impl FnMut(u64, Option<u64>),
) -> Result<(), GitHubClientError> {
    fs::create_dir_all(dir)?;
    let dll_path = plugin_dll_path(dir, &manifest.id);
    github_client::download_file_with_progress(download_url, &dll_path, on_progress)?;

    if let Err(err) = github_client::verify_sha256(&dll_path, &manifest.sha256) {
        let _ = fs::remove_file(&dll_path);
        return Err(err);
    }

    save_manifest(dir, manifest)
}

pub fn remove_plugin(dir: &Path, id: &str) -> std::io::Result<()> {
    let _ = fs::remove_file(plugin_dll_path(dir, id));
    let manifest = manifest_path(dir, id);
    if manifest.exists() {
        fs::remove_file(manifest)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mameset_cleaner_plugin_registry_{label}_{}",
            std::process::id()
        ))
    }

    fn sample_manifest(id: &str, version: &str, sha256: &str) -> StoredManifest {
        StoredManifest {
            id: id.to_string(),
            name: format!("{id} system"),
            emulator_frontend: "Batocera".to_string(),
            console_family: "Test".to_string(),
            version: version.to_string(),
            dat_format: "fake".to_string(),
            sha256: sha256.to_string(),
            min_app_version: "1.2.0".to_string(),
        }
    }

    #[test]
    fn listing_an_empty_or_missing_directory_returns_no_plugins() {
        let dir = temp_dir("empty");
        let _ = fs::remove_dir_all(&dir);
        assert!(list_installed(&dir).is_empty());
    }

    #[test]
    fn installing_valid_bytes_makes_the_plugin_detectable() {
        let dir = temp_dir("install_ok");
        let _ = fs::remove_dir_all(&dir);

        let bytes = b"fake dll content";
        let hash = github_client::compute_sha256_hex_bytes(bytes);
        let manifest = sample_manifest("nes", "1.0.0", &hash);

        install_plugin_from_bytes(&dir, bytes, &manifest).unwrap();

        let installed = list_installed(&dir);
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id, "nes");
        assert!(plugin_dll_path(&dir, "nes").exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn installing_bytes_with_a_wrong_hash_is_rejected_and_leaves_no_trace() {
        let dir = temp_dir("install_bad_hash");
        let _ = fs::remove_dir_all(&dir);

        let bytes = b"fake dll content";
        let manifest = sample_manifest("nes", "1.0.0", &"0".repeat(64));

        let result = install_plugin_from_bytes(&dir, bytes, &manifest);
        assert!(result.is_err());
        assert!(list_installed(&dir).is_empty());
        assert!(!plugin_dll_path(&dir, "nes").exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn compares_status_between_local_and_remote_versions() {
        let local = sample_manifest("nes", "1.0.0", "abc");
        assert_eq!(compare_status(None, "1.0.0"), PluginStatus::NotInstalled);
        assert_eq!(
            compare_status(Some(&local), "1.0.0"),
            PluginStatus::Installed
        );
        assert_eq!(
            compare_status(Some(&local), "1.1.0"),
            PluginStatus::UpdateAvailable
        );
    }

    #[test]
    fn removing_a_plugin_deletes_its_dll_and_manifest() {
        let dir = temp_dir("remove");
        let _ = fs::remove_dir_all(&dir);

        let bytes = b"fake dll content";
        let hash = github_client::compute_sha256_hex_bytes(bytes);
        let manifest = sample_manifest("nes", "1.0.0", &hash);
        install_plugin_from_bytes(&dir, bytes, &manifest).unwrap();

        remove_plugin(&dir, "nes").unwrap();

        assert!(list_installed(&dir).is_empty());
        assert!(!plugin_dll_path(&dir, "nes").exists());

        fs::remove_dir_all(&dir).unwrap();
    }
}
