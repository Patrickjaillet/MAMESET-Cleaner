use crate::core::config_manager::AppConfig;
use crate::core::i18n::Translator;

slint::include_modules!();

pub fn run(config: &AppConfig, translator: &Translator) -> Result<(), slint::PlatformError> {
    let _ = translator;
    let window = AppWindow::new()?;
    window.set_app_version(env!("CARGO_PKG_VERSION").into());

    tracing::info!(theme = ?config.theme, language = ?config.language, "fenêtre principale initialisée");

    window.run()
}
