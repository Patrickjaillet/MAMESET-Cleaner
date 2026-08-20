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
    }

    #[test]
    fn default_config_uses_the_built_in_mame_system() {
        assert_eq!(AppConfig::default().selected_system, "mame");
    }
}
