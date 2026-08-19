use std::collections::HashMap;

use crate::core::config_manager::AppLanguage;

const FR_JSON: &str = include_str!("../../assets/i18n/fr.json");
const EN_JSON: &str = include_str!("../../assets/i18n/en.json");

pub struct Translator {
    entries: HashMap<String, String>,
}

impl Translator {
    pub fn load(language: &AppLanguage) -> Self {
        let source = match language {
            AppLanguage::Fr => FR_JSON,
            AppLanguage::En => EN_JSON,
        };
        let entries: HashMap<String, String> =
            serde_json::from_str(source).expect("fichier i18n intégré invalide");
        Self { entries }
    }

    pub fn get(&self, key: &str) -> String {
        self.entries
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_translator_resolves_known_key() {
        let translator = Translator::load(&AppLanguage::Fr);
        assert_eq!(translator.get("nav.scan"), "Scan");
        assert_eq!(translator.get("nav.settings"), "Paramètres");
    }

    #[test]
    fn english_translator_resolves_known_key() {
        let translator = Translator::load(&AppLanguage::En);
        assert_eq!(translator.get("nav.settings"), "Settings");
    }

    #[test]
    fn unknown_key_falls_back_to_key_itself() {
        let translator = Translator::load(&AppLanguage::Fr);
        assert_eq!(translator.get("nav.unknown"), "nav.unknown");
    }
}
