use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppLanguage {
    Fr,
    En,
}

fn default_use_recycle_bin() -> bool {
    true
}

fn default_selected_system() -> String {
    "mame".to_string()
}

/// `World > USA > Europe > Japan` — the priority order used unmodified
/// since v0.1.0, kept as the default so nobody's ROM set changes shape
/// unless they deliberately customize it in Settings.
pub fn default_region_priority() -> Vec<String> {
    ["World", "USA", "Europe", "Japan"]
        .into_iter()
        .map(String::from)
        .collect()
}

fn default_treat_unofficial_as_official() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: AppLanguage,
    pub rom_set_path: Option<String>,
    pub dat_file_path: Option<String>,
    pub catver_ini_path: Option<String>,
    pub languages_ini_path: Option<String>,
    #[serde(default)]
    pub backup_dir_path: Option<String>,
    #[serde(default = "default_use_recycle_bin")]
    pub use_recycle_bin: bool,
    #[serde(default = "default_selected_system")]
    pub selected_system: String,
    /// 1G1R region priority, most-preferred first. Empty entries are never
    /// stored (see Settings UI) but an empty list overall is tolerated and
    /// treated as "no preference" (every region ties).
    #[serde(default = "default_region_priority")]
    pub region_priority: Vec<String>,
    /// 1G1R language tie-breaker, most-preferred first. Empty by default
    /// (no language preference — today's exact behavior).
    #[serde(default)]
    pub preferred_languages: Vec<String>,
    /// When false (the default — a correctness improvement over the old
    /// behavior), prototypes/betas/demos/unlicensed releases are never
    /// picked as the 1G1R "keep" copy over an official release in the same
    /// group. When true, they're treated as equally valid candidates.
    #[serde(default = "default_treat_unofficial_as_official")]
    pub treat_unofficial_as_official: bool,
    /// When true, verification also recomputes and compares SHA1 (not just
    /// CRC32 + size) for every scanned ROM. Slower — requires decompressing
    /// every archive entry — so it's opt-in.
    #[serde(default)]
    pub deep_verify_sha1: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: AppLanguage::Fr,
            rom_set_path: None,
            dat_file_path: None,
            catver_ini_path: None,
            languages_ini_path: None,
            backup_dir_path: None,
            use_recycle_bin: true,
            selected_system: default_selected_system(),
            region_priority: default_region_priority(),
            preferred_languages: Vec::new(),
            treat_unofficial_as_official: default_treat_unofficial_as_official(),
            deep_verify_sha1: false,
        }
    }
}

impl AppConfig {
    pub fn config_file_path() -> PathBuf {
        config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_file_path();
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => config,
                Err(err) => {
                    tracing::warn!(error = %err, "configuration invalide, utilisation des valeurs par défaut");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = config_dir();
        fs::create_dir_all(&dir)?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(Self::config_file_path(), content)
    }
}

pub(crate) fn config_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    base.join("MAMESET-Cleaner")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_french_language() {
        let config = AppConfig::default();
        assert_eq!(config.language, AppLanguage::Fr);
        assert!(config.rom_set_path.is_none());
    }

    #[test]
    fn config_round_trips_through_json() {
        let config = AppConfig {
            language: AppLanguage::En,
            rom_set_path: Some("C:/roms".to_string()),
            dat_file_path: None,
            catver_ini_path: None,
            languages_ini_path: None,
            backup_dir_path: None,
            use_recycle_bin: false,
            selected_system: "nes".to_string(),
            region_priority: default_region_priority(),
            preferred_languages: Vec::new(),
            treat_unofficial_as_official: false,
            deep_verify_sha1: false,
        };
        let json = serde_json::to_string(&config).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.language, AppLanguage::En);
        assert_eq!(restored.rom_set_path.as_deref(), Some("C:/roms"));
        assert_eq!(restored.selected_system, "nes");
    }

    #[test]
    fn a_config_file_saved_by_an_older_version_with_a_theme_field_still_loads() {
        let json = r#"{
            "language": "Fr",
            "theme": "Dark",
            "rom_set_path": null,
            "dat_file_path": null,
            "catver_ini_path": null,
            "languages_ini_path": null,
            "use_recycle_bin": true,
            "selected_system": "mame"
        }"#;
        let restored: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(restored.language, AppLanguage::Fr);
        assert_eq!(restored.region_priority, default_region_priority());
        assert!(restored.preferred_languages.is_empty());
        assert!(!restored.treat_unofficial_as_official);
        assert!(!restored.deep_verify_sha1);
    }

    #[test]
    fn default_config_uses_world_usa_europe_japan_region_priority() {
        assert_eq!(
            AppConfig::default().region_priority,
            vec!["World", "USA", "Europe", "Japan"]
        );
    }

    #[test]
    fn default_config_uses_the_built_in_mame_system() {
        assert_eq!(AppConfig::default().selected_system, "mame");
    }
}
