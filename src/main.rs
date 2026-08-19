#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mameset_cleaner::bridge;
use mameset_cleaner::core;

use core::config_manager::AppConfig;
use core::i18n::Translator;

fn main() -> Result<(), slint::PlatformError> {
    core::logging::init();

    let config = AppConfig::load();
    let translator = Translator::load(&config.language);

    bridge::ui_bindings::run(&config, &translator)
}
