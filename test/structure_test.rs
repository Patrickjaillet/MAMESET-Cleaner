use mameset_cleaner::core::config_manager::{AppConfig, AppLanguage};
use mameset_cleaner::core::i18n::Translator;

#[test]
fn default_config_is_ready_for_first_launch() {
    let config = AppConfig::default();
    assert_eq!(config.language, AppLanguage::Fr);
    assert!(config.rom_set_path.is_none());
    assert!(config.dat_file_path.is_none());
    assert!(config.catver_ini_path.is_none());
    assert!(config.languages_ini_path.is_none());
}

#[test]
fn translator_covers_all_navigation_entries_in_both_languages() {
    let keys = [
        "nav.scan",
        "nav.filters",
        "nav.results",
        "nav.settings",
        "nav.about",
    ];

    for language in [AppLanguage::Fr, AppLanguage::En] {
        let translator = Translator::load(&language);
        for key in keys {
            let value = translator.get(key);
            assert_ne!(value, key, "clé i18n manquante : {key}");
        }
    }
}
