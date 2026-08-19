use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use slint::{ModelRc, VecModel, Weak};

use crate::core::config_manager::{AppConfig, AppLanguage, AppTheme};
use crate::core::i18n::Translator;
use crate::core::{catver_parser, dat_parser, dedup_engine, filter_engine, languages_parser, rom_scanner};
use crate::models::filter_profile::FilterProfile;
use crate::models::rom_entry::{DriverStatus, RomEntry};
use crate::models::rom_set::{RomSet, RomStatus};

slint::include_modules!();

struct AppState {
    config: AppConfig,
    dat_entries: HashMap<String, RomEntry>,
    rom_set: RomSet,
    dedup_remove: HashSet<String>,
    filter_profile: FilterProfile,
    sort_column: String,
    sort_ascending: bool,
}

impl AppState {
    fn new(config: AppConfig) -> Self {
        Self {
            config,
            dat_entries: HashMap::new(),
            rom_set: RomSet::new(),
            dedup_remove: HashSet::new(),
            filter_profile: FilterProfile::default(),
            sort_column: "name".to_string(),
            sort_ascending: true,
        }
    }
}

pub fn run(config: &AppConfig, translator: &Translator) -> Result<(), slint::PlatformError> {
    let _ = translator;
    let window = AppWindow::new()?;

    window.set_app_version(env!("CARGO_PKG_VERSION").into());
    window.set_rom_set_path(config.rom_set_path.clone().unwrap_or_default().into());
    window.set_dat_file_path(config.dat_file_path.clone().unwrap_or_default().into());
    window.set_catver_ini_path(config.catver_ini_path.clone().unwrap_or_default().into());
    window.set_languages_ini_path(config.languages_ini_path.clone().unwrap_or_default().into());
    window.set_language_is_english(config.language == AppLanguage::En);
    window.set_dark_theme(config.theme == AppTheme::Dark);
    window.set_scan_progress(0.0);
    window.set_scan_status_text("Aucun scan effectué.".into());
    window.set_scan_counts_text(String::new().into());
    window.set_filter_summary_text(String::new().into());
    window.set_result_count_text(String::new().into());

    let state = Arc::new(Mutex::new(AppState::new(config.clone())));

    setup_browse_callbacks(&window);
    setup_save_settings(&window, &state);
    setup_start_scan(&window, &state);
    setup_apply_filters(&window, &state);
    setup_search(&window, &state);
    setup_sort(&window, &state);

    tracing::info!("fenêtre principale initialisée");

    window.run()
}

fn setup_browse_callbacks(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_browse_rom_set_path(move || {
        if let Some(window) = weak.upgrade() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                window.set_rom_set_path(path.display().to_string().into());
            }
        }
    });

    let weak = window.as_weak();
    window.on_browse_dat_file_path(move || {
        if let Some(window) = weak.upgrade() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Fichier DAT", &["xml", "dat"])
                .pick_file()
            {
                window.set_dat_file_path(path.display().to_string().into());
            }
        }
    });

    let weak = window.as_weak();
    window.on_browse_catver_ini_path(move || {
        if let Some(window) = weak.upgrade() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Fichier INI", &["ini"])
                .pick_file()
            {
                window.set_catver_ini_path(path.display().to_string().into());
            }
        }
    });

    let weak = window.as_weak();
    window.on_browse_languages_ini_path(move || {
        if let Some(window) = weak.upgrade() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Fichier INI", &["ini"])
                .pick_file()
            {
                window.set_languages_ini_path(path.display().to_string().into());
            }
        }
    });
}

fn setup_save_settings(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_save_settings(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let language = if window.get_language_is_english() {
            AppLanguage::En
        } else {
            AppLanguage::Fr
        };
        let theme = if window.get_dark_theme() {
            AppTheme::Dark
        } else {
            AppTheme::Light
        };

        let config = AppConfig {
            language,
            theme,
            rom_set_path: non_empty(window.get_rom_set_path().to_string()),
            dat_file_path: non_empty(window.get_dat_file_path().to_string()),
            catver_ini_path: non_empty(window.get_catver_ini_path().to_string()),
            languages_ini_path: non_empty(window.get_languages_ini_path().to_string()),
        };

        let status = match config.save() {
            Ok(()) => "Paramètres enregistrés.".to_string(),
            Err(err) => format!("Erreur lors de l'enregistrement : {err}"),
        };

        {
            let mut guard = state.lock().unwrap();
            guard.config = config;
        }

        window.set_settings_status_text(status.into());
    });
}

fn setup_start_scan(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_start_scan(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let (rom_set_path, dat_file_path, catver_ini_path, languages_ini_path) = {
            let guard = state.lock().unwrap();
            (
                guard.config.rom_set_path.clone(),
                guard.config.dat_file_path.clone(),
                guard.config.catver_ini_path.clone(),
                guard.config.languages_ini_path.clone(),
            )
        };

        let Some(rom_set_path) = rom_set_path else {
            window.set_scan_status_text(
                "Veuillez définir le dossier de ROMs dans Paramètres.".into(),
            );
            return;
        };
        let Some(dat_file_path) = dat_file_path else {
            window
                .set_scan_status_text("Veuillez définir le fichier DAT dans Paramètres.".into());
            return;
        };

        window.set_scan_running(true);
        window.set_scan_progress(0.0);
        window.set_scan_status_text("Analyse en cours...".into());

        let state_for_thread = Arc::clone(&state);
        let weak_for_thread = weak.clone();

        std::thread::spawn(move || {
            let weak_shared = Arc::new(Mutex::new(weak_for_thread.clone()));

            let result = run_scan_pipeline(
                &rom_set_path,
                &dat_file_path,
                catver_ini_path.as_deref(),
                languages_ini_path.as_deref(),
                Arc::clone(&weak_shared),
            );

            let weak_done = weak_for_thread.clone();
            match result {
                Ok((dat_entries, rom_set, dedup_remove)) => {
                    let counts_text = format_scan_counts(&rom_set);
                    {
                        let mut guard = state_for_thread.lock().unwrap();
                        guard.dat_entries = dat_entries;
                        guard.rom_set = rom_set;
                        guard.dedup_remove = dedup_remove;
                    }

                    let state_after = Arc::clone(&state_for_thread);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = weak_done.upgrade() {
                            window.set_scan_running(false);
                            window.set_scan_progress(1.0);
                            window.set_scan_status_text("Scan terminé.".into());
                            window.set_scan_counts_text(counts_text.into());
                            refresh_results(&window, &state_after);
                        }
                    });
                }
                Err(message) => {
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = weak_done.upgrade() {
                            window.set_scan_running(false);
                            window.set_scan_status_text(format!("Erreur : {message}").into());
                        }
                    });
                }
            }
        });
    });
}

fn run_scan_pipeline(
    rom_set_path: &str,
    dat_file_path: &str,
    catver_ini_path: Option<&str>,
    languages_ini_path: Option<&str>,
    weak_shared: Arc<Mutex<Weak<AppWindow>>>,
) -> Result<(HashMap<String, RomEntry>, RomSet, HashSet<String>), String> {
    let mut dat_entries =
        dat_parser::parse_dat_file(Path::new(dat_file_path)).map_err(|e| e.to_string())?;

    let categories = catver_ini_path
        .filter(|p| !p.is_empty())
        .and_then(|p| catver_parser::parse_catver_file(Path::new(p)).ok())
        .unwrap_or_default();

    let languages = languages_ini_path
        .filter(|p| !p.is_empty())
        .and_then(|p| languages_parser::parse_languages_file(Path::new(p)).ok())
        .unwrap_or_default();

    dat_parser::merge_metadata(&mut dat_entries, &categories, &languages);

    let rom_set = rom_scanner::scan_rom_directory(
        Path::new(rom_set_path),
        &dat_entries,
        move |progress: rom_scanner::ScanProgress| {
            let ratio = if progress.total == 0 {
                1.0
            } else {
                progress.processed as f32 / progress.total as f32
            };
            let status = format!(
                "{} / {} — {}",
                progress.processed, progress.total, progress.current_name
            );
            let weak = weak_shared.lock().unwrap().clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    window.set_scan_progress(ratio);
                    window.set_scan_status_text(status.into());
                }
            });
        },
    )
    .map_err(|e| e.to_string())?;

    let dedup_plan =
        dedup_engine::build_dedup_plan(&dat_entries, &dedup_engine::RegionPriority::default_profile());
    let dedup_remove: HashSet<String> = dedup_plan.roms_to_remove().into_iter().collect();

    Ok((dat_entries, rom_set, dedup_remove))
}

fn format_scan_counts(rom_set: &RomSet) -> String {
    format!(
        "OK : {} | Manquantes : {} | Corrompues : {} | Non référencées : {}",
        rom_set.count_by_status(RomStatus::Ok),
        rom_set.count_by_status(RomStatus::Missing),
        rom_set.count_by_status(RomStatus::Corrupted),
        rom_set.count_by_status(RomStatus::Unreferenced),
    )
}

fn setup_apply_filters(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_apply_filters(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let profile = FilterProfile {
            name: "Courant".to_string(),
            categories: split_csv(&window.get_filter_categories_text()),
            languages: split_csv(&window.get_filter_languages_text()),
            regions: split_csv(&window.get_filter_regions_text()),
            manufacturers: split_csv(&window.get_filter_manufacturers_text()),
            year_min: parse_year(&window.get_filter_year_min_text()),
            year_max: parse_year(&window.get_filter_year_max_text()),
            driver_statuses: collect_statuses(&window),
            include_bios: window.get_filter_include_bios(),
            include_device: window.get_filter_include_device(),
            include_mechanical: window.get_filter_include_mechanical(),
            include_adult: window.get_filter_include_adult(),
        };

        let (match_count, total) = {
            let mut guard = state.lock().unwrap();
            guard.filter_profile = profile;
            let match_count = filter_engine::apply_filter(&guard.dat_entries, &guard.filter_profile).len();
            (match_count, guard.dat_entries.len())
        };

        window.set_filter_summary_text(
            format!("{match_count} / {total} ROMs correspondent aux critères").into(),
        );

        refresh_results(&window, &state);
    });
}

fn setup_search(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_search_triggered(move || {
        if let Some(window) = weak.upgrade() {
            refresh_results(&window, &state);
        }
    });
}

fn setup_sort(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_sort_by(move |column| {
        let Some(window) = weak.upgrade() else {
            return;
        };

        {
            let mut guard = state.lock().unwrap();
            if guard.sort_column == column.as_str() {
                guard.sort_ascending = !guard.sort_ascending;
            } else {
                guard.sort_column = column.to_string();
                guard.sort_ascending = true;
            }
        }

        refresh_results(&window, &state);
    });
}

fn refresh_results(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let guard = state.lock().unwrap();
    let search = window.get_search_text().to_string().to_ascii_lowercase();

    let mut matched: Vec<&RomEntry> = filter_engine::apply_filter(&guard.dat_entries, &guard.filter_profile);

    if !search.is_empty() {
        matched.retain(|entry| {
            entry.name.to_ascii_lowercase().contains(&search)
                || entry.description.to_ascii_lowercase().contains(&search)
        });
    }

    match guard.sort_column.as_str() {
        "year" => matched.sort_by(|a, b| a.year.cmp(&b.year)),
        _ => matched.sort_by(|a, b| a.name.cmp(&b.name)),
    }
    if !guard.sort_ascending {
        matched.reverse();
    }

    let total_scanned = guard.dat_entries.len();
    let match_count = matched.len();

    let rows: Vec<ResultRow> = matched
        .iter()
        .map(|entry| {
            let status_text = guard
                .rom_set
                .entries
                .get(&entry.name)
                .map(|scanned| format_rom_status(scanned.status))
                .unwrap_or_else(|| "Non scannée".to_string());
            let action_text = if guard.dedup_remove.contains(&entry.name) {
                "À supprimer".to_string()
            } else {
                "À conserver".to_string()
            };

            ResultRow {
                name: entry.name.clone().into(),
                description: entry.description.clone().into(),
                year: entry.year.clone().into(),
                manufacturer: entry.manufacturer.clone().into(),
                category: entry.category.clone().unwrap_or_default().into(),
                status: status_text.into(),
                action: action_text.into(),
            }
        })
        .collect();

    drop(guard);

    window.set_result_rows(ModelRc::new(VecModel::from(rows)));
    window.set_result_count_text(format!("{match_count} / {total_scanned} ROMs affichées").into());
}

fn format_rom_status(status: RomStatus) -> String {
    match status {
        RomStatus::Ok => "OK".to_string(),
        RomStatus::Missing => "Manquante".to_string(),
        RomStatus::Corrupted => "Corrompue".to_string(),
        RomStatus::Unreferenced => "Non référencée".to_string(),
    }
}

fn collect_statuses(window: &AppWindow) -> Vec<DriverStatus> {
    let mut statuses = Vec::new();
    if window.get_filter_status_good() {
        statuses.push(DriverStatus::Good);
    }
    if window.get_filter_status_imperfect() {
        statuses.push(DriverStatus::Imperfect);
    }
    if window.get_filter_status_preliminary() {
        statuses.push(DriverStatus::Preliminary);
    }
    if window.get_filter_status_unknown() {
        statuses.push(DriverStatus::Unknown);
    }
    statuses
}

fn split_csv(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_year(text: &str) -> Option<u32> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse().ok()
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
