use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use slint::{Model, ModelRc, VecModel, Weak};

use crate::core::cleanup_engine::{CleanupOptions, CleanupTarget};
use crate::core::config_manager::{AppConfig, AppLanguage};
use crate::core::i18n::Translator;
use crate::core::{
    catver_parser, cleanup_engine, dat_parser, dedup_engine, filter_engine, languages_parser,
    profile_manager, report_generator, rom_scanner,
};
use crate::models::filter_profile::FilterProfile;
use crate::models::rom_entry::{DriverStatus, RomEntry};
use crate::models::rom_set::{RomSet, RomStatus};
use crate::plugin::github_client::{self, GitHubClientError, RemoteFile};
use crate::plugin::loader;
use crate::plugin::registry::{self, PluginStatus, StoredManifest};
use crate::plugin::{self, RomSystem};

slint::include_modules!();

const PLUGIN_REPO_CONTENTS_URL: &str =
    "https://api.github.com/repos/Patrickjaillet/MAMESET-Cleaner/contents/plugins";

struct RemotePlugin {
    manifest: StoredManifest,
    dll_download_url: String,
}

struct AppState {
    config: AppConfig,
    dat_entries: HashMap<String, RomEntry>,
    rom_set: RomSet,
    dedup_remove: HashSet<String>,
    dedup_keep: HashSet<String>,
    filter_profile: FilterProfile,
    sort_column: String,
    sort_ascending: bool,
    scan_cancel_flag: Arc<AtomicBool>,
    plugins_dir: PathBuf,
    remote_plugins: HashMap<String, RemotePlugin>,
}

impl AppState {
    fn new(config: AppConfig) -> Self {
        Self {
            config,
            dat_entries: HashMap::new(),
            rom_set: RomSet::new(),
            dedup_remove: HashSet::new(),
            dedup_keep: HashSet::new(),
            filter_profile: FilterProfile::default(),
            sort_column: "name".to_string(),
            sort_ascending: true,
            scan_cancel_flag: Arc::new(AtomicBool::new(false)),
            plugins_dir: registry::default_plugins_dir(),
            remote_plugins: HashMap::new(),
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
    window.set_backup_dir_path(config.backup_dir_path.clone().unwrap_or_default().into());
    window.set_use_recycle_bin(config.use_recycle_bin);
    window.set_language_is_english(config.language == AppLanguage::En);
    window.set_scan_progress(0.0);
    window.set_scan_status_text("Aucun scan effectué.".into());
    window.set_scan_counts_text(String::new().into());
    window.set_filter_summary_text(String::new().into());
    window.set_result_count_text(String::new().into());
    window.set_cleanup_status_text(String::new().into());
    window.set_integrity_status_text(String::new().into());
    window.set_plugins_status_text(String::new().into());
    window.set_plugin_install_status_text(String::new().into());
    window.set_plugin_rows(ModelRc::new(VecModel::from(Vec::<PluginRow>::new())));
    window.set_selected_system_id(config.selected_system.clone().into());
    window.set_filter_live_count_text(String::new().into());
    window.set_profile_status_text(String::new().into());

    let state = Arc::new(Mutex::new(AppState::new(config.clone())));

    refresh_available_systems(&window, &state);
    reset_selected_system_if_uninstalled(&window, &state);

    refresh_filter_options(&window, &state);
    refresh_profile_list(&window);

    setup_browse_callbacks(&window);
    setup_open_url(&window);
    setup_save_settings(&window, &state);
    setup_start_scan(&window, &state);
    setup_cancel_scan(&window, &state);
    setup_filter_option_toggles(&window, &state);
    setup_apply_filters(&window, &state);
    setup_filters_changed(&window, &state);
    setup_clear_all_filters(&window, &state);
    setup_profile_management(&window, &state);
    setup_search(&window, &state);
    setup_sort(&window, &state);
    setup_cleanup(&window, &state);
    setup_plugins(&window, &state);

    refresh_plugin_rows(&window, &state);

    tracing::info!("fenêtre principale initialisée");

    window.run()
}

fn setup_open_url(window: &AppWindow) {
    window.on_open_url(|url| {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url.as_str()])
            .spawn();
    });
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

    let weak = window.as_weak();
    window.on_browse_backup_dir_path(move || {
        if let Some(window) = weak.upgrade() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                window.set_backup_dir_path(path.display().to_string().into());
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
        let config = AppConfig {
            language,
            rom_set_path: non_empty(window.get_rom_set_path().to_string()),
            dat_file_path: non_empty(window.get_dat_file_path().to_string()),
            catver_ini_path: non_empty(window.get_catver_ini_path().to_string()),
            languages_ini_path: non_empty(window.get_languages_ini_path().to_string()),
            backup_dir_path: non_empty(window.get_backup_dir_path().to_string()),
            use_recycle_bin: window.get_use_recycle_bin(),
            selected_system: window.get_selected_system_id().to_string(),
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

        let (rom_set_path, dat_file_path, catver_ini_path, languages_ini_path, selected_system, plugins_dir) = {
            let guard = state.lock().unwrap();
            (
                guard.config.rom_set_path.clone(),
                guard.config.dat_file_path.clone(),
                guard.config.catver_ini_path.clone(),
                guard.config.languages_ini_path.clone(),
                guard.config.selected_system.clone(),
                guard.plugins_dir.clone(),
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

        let cancel_flag = Arc::new(AtomicBool::new(false));
        {
            let mut guard = state.lock().unwrap();
            guard.scan_cancel_flag = Arc::clone(&cancel_flag);
        }

        let state_for_thread = Arc::clone(&state);
        let weak_for_thread = weak.clone();

        std::thread::spawn(move || {
            let weak_shared = Arc::new(Mutex::new(weak_for_thread.clone()));

            let result = run_scan_pipeline(
                ScanPipelineInput {
                    rom_set_path: &rom_set_path,
                    dat_file_path: &dat_file_path,
                    catver_ini_path: catver_ini_path.as_deref(),
                    languages_ini_path: languages_ini_path.as_deref(),
                    selected_system: &selected_system,
                    plugins_dir: &plugins_dir,
                },
                &cancel_flag,
                Arc::clone(&weak_shared),
            );

            let was_cancelled = cancel_flag.load(Ordering::Relaxed);
            let weak_done = weak_for_thread.clone();
            match result {
                Ok((dat_entries, rom_set, dedup_remove, dedup_keep)) => {
                    let counts_text = format_scan_counts(&rom_set);
                    {
                        let mut guard = state_for_thread.lock().unwrap();
                        guard.dat_entries = dat_entries;
                        guard.rom_set = rom_set;
                        guard.dedup_remove = dedup_remove;
                        guard.dedup_keep = dedup_keep;
                    }

                    let state_after = Arc::clone(&state_for_thread);
                    let status_text = if was_cancelled {
                        "Scan annulé (résultats partiels)."
                    } else {
                        "Scan terminé."
                    };
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = weak_done.upgrade() {
                            window.set_scan_running(false);
                            window.set_scan_progress(1.0);
                            window.set_scan_status_text(status_text.into());
                            window.set_scan_counts_text(counts_text.into());
                            refresh_filter_options(&window, &state_after);
                            recompute_live_filter_preview(&window, &state_after);
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

fn setup_cancel_scan(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_cancel_scan(move || {
        if let Some(window) = weak.upgrade() {
            state
                .lock()
                .unwrap()
                .scan_cancel_flag
                .store(true, Ordering::Relaxed);
            window.set_scan_status_text("Annulation en cours...".into());
        }
    });
}

type ScanPipelineResult = Result<(HashMap<String, RomEntry>, RomSet, HashSet<String>, HashSet<String>), String>;

/// Loads the reference database for the currently selected system: MAME's
/// own `-listxml` parser for the built-in `"mame"` system, or, for any other
/// system, the corresponding plugin's `.dll` (dynamically loaded) and its
/// own `parse_reference_database`, converted into the shared [`RomEntry`]
/// model so the rest of the pipeline (scanner, dedup, filters) stays
/// system-agnostic.
fn load_reference_database(
    selected_system: &str,
    plugins_dir: &Path,
    dat_file_path: &Path,
) -> Result<HashMap<String, RomEntry>, String> {
    if selected_system == "mame" {
        return dat_parser::parse_dat_file(dat_file_path).map_err(|e| e.to_string());
    }

    let dll_path = registry::plugin_dll_path(plugins_dir, selected_system);
    if !dll_path.exists() {
        return Err(format!(
            "le système « {selected_system} » n'est plus installé — sélectionnez un autre système dans Paramètres"
        ));
    }
    let loaded_plugin = loader::load_plugin_from_file_expecting_id(&dll_path, selected_system)
        .map_err(|err| format!("impossible de charger le plugin « {selected_system} » : {err}"))?;
    let plugin_entries = loaded_plugin.parse_reference_database(dat_file_path)?;
    Ok(plugin::plugin_entries_to_rom_entries(plugin_entries))
}

struct ScanPipelineInput<'a> {
    rom_set_path: &'a str,
    dat_file_path: &'a str,
    catver_ini_path: Option<&'a str>,
    languages_ini_path: Option<&'a str>,
    selected_system: &'a str,
    plugins_dir: &'a Path,
}

fn run_scan_pipeline(
    input: ScanPipelineInput,
    cancel_flag: &AtomicBool,
    weak_shared: Arc<Mutex<Weak<AppWindow>>>,
) -> ScanPipelineResult {
    let mut dat_entries = load_reference_database(
        input.selected_system,
        input.plugins_dir,
        Path::new(input.dat_file_path),
    )?;

    let categories = input
        .catver_ini_path
        .filter(|p| !p.is_empty())
        .and_then(|p| catver_parser::parse_catver_file(Path::new(p)).ok())
        .unwrap_or_default();

    let languages = input
        .languages_ini_path
        .filter(|p| !p.is_empty())
        .and_then(|p| languages_parser::parse_languages_file(Path::new(p)).ok())
        .unwrap_or_default();

    dat_parser::merge_metadata(&mut dat_entries, &categories, &languages);

    let rom_set = rom_scanner::scan_rom_directory(
        Path::new(input.rom_set_path),
        &dat_entries,
        cancel_flag,
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
    let dedup_keep: HashSet<String> = dedup_plan.roms_to_keep().into_iter().collect();

    Ok((dat_entries, rom_set, dedup_remove, dedup_keep))
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

fn build_profile_from_window(window: &AppWindow, name: &str) -> FilterProfile {
    FilterProfile {
        name: name.to_string(),
        categories: selected_values(&window.get_category_options()),
        languages: selected_values(&window.get_language_options()),
        regions: selected_values(&window.get_region_options()),
        manufacturers: selected_values(&window.get_manufacturer_options()),
        year_min: parse_year(&window.get_filter_year_min_text()),
        year_max: parse_year(&window.get_filter_year_max_text()),
        driver_statuses: collect_statuses(window),
        include_bios: window.get_filter_include_bios(),
        include_device: window.get_filter_include_device(),
        include_mechanical: window.get_filter_include_mechanical(),
        include_adult: window.get_filter_include_adult(),
    }
}

fn setup_apply_filters(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_apply_filters(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let profile = build_profile_from_window(&window, "Courant");

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

fn recompute_live_filter_preview(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let guard = state.lock().unwrap();
    if guard.dat_entries.is_empty() {
        window.set_filter_live_count_text(String::new().into());
        return;
    }

    let profile = build_profile_from_window(window, "Aperçu");
    let match_count = filter_engine::apply_filter(&guard.dat_entries, &profile).len();
    let total = guard.dat_entries.len();
    drop(guard);

    window.set_filter_live_count_text(
        format!("{match_count} / {total} ROMs correspondront à ces critères").into(),
    );
}

fn setup_filters_changed(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_filters_changed(move || {
        if let Some(window) = weak.upgrade() {
            recompute_live_filter_preview(&window, &state);
        }
    });
}

fn setup_clear_all_filters(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state = Arc::clone(state);

    window.on_clear_all_filters(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let clear = |options: ModelRc<FilterOption>| -> ModelRc<FilterOption> {
            let cleared: Vec<FilterOption> = options
                .iter()
                .map(|mut opt| {
                    opt.selected = false;
                    opt
                })
                .collect();
            ModelRc::new(VecModel::from(cleared))
        };

        window.set_category_options(clear(window.get_category_options()));
        window.set_language_options(clear(window.get_language_options()));
        window.set_region_options(clear(window.get_region_options()));
        window.set_manufacturer_options(clear(window.get_manufacturer_options()));
        window.set_category_selected_count(0);
        window.set_language_selected_count(0);
        window.set_region_selected_count(0);
        window.set_manufacturer_selected_count(0);
        window.set_filter_year_min_text(String::new().into());
        window.set_filter_year_max_text(String::new().into());
        window.set_filter_status_good(true);
        window.set_filter_status_imperfect(true);
        window.set_filter_status_preliminary(true);
        window.set_filter_status_unknown(true);
        window.set_filter_include_bios(true);
        window.set_filter_include_device(true);
        window.set_filter_include_mechanical(true);
        window.set_filter_include_adult(true);

        recompute_live_filter_preview(&window, &state);
    });
}

fn refresh_profile_list(window: &AppWindow) {
    let names = profile_manager::list_profiles().unwrap_or_default();
    let items: Vec<slint::SharedString> = names.into_iter().map(Into::into).collect();
    window.set_profile_names(ModelRc::new(VecModel::from(items)));
}

fn setup_profile_management(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    window.on_save_profile(move |name| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let trimmed = name.trim();
        if trimmed.is_empty() {
            window.set_profile_status_text("Veuillez saisir un nom de profil.".into());
            return;
        }

        let profile = build_profile_from_window(&window, trimmed);
        match profile_manager::save_profile(&profile) {
            Ok(()) => {
                refresh_profile_list(&window);
                window.set_profile_status_text(format!("Profil « {trimmed} » enregistré.").into());
            }
            Err(err) => {
                window.set_profile_status_text(format!("Erreur : {err}").into());
            }
        }
    });

    let weak = window.as_weak();
    let state_load = Arc::clone(state);
    window.on_load_profile(move |name| {
        let Some(window) = weak.upgrade() else {
            return;
        };

        match profile_manager::load_profile(&name) {
            Ok(profile) => {
                apply_loaded_profile(&window, &state_load, profile);
                window.set_profile_status_text(format!("Profil « {name} » chargé et appliqué.").into());
                refresh_results(&window, &state_load);
            }
            Err(err) => {
                window.set_profile_status_text(format!("Erreur : {err}").into());
            }
        }
    });

    let weak = window.as_weak();
    window.on_delete_profile(move |name| {
        let Some(window) = weak.upgrade() else {
            return;
        };

        match profile_manager::delete_profile(&name) {
            Ok(()) => {
                refresh_profile_list(&window);
                window.set_profile_status_text(format!("Profil « {name} » supprimé.").into());
            }
            Err(err) => {
                window.set_profile_status_text(format!("Erreur : {err}").into());
            }
        }
    });
}

fn apply_loaded_profile(window: &AppWindow, state: &Arc<Mutex<AppState>>, profile: FilterProfile) {
    let mut guard = state.lock().unwrap();
    guard.filter_profile = profile.clone();

    let category_options =
        build_filter_options(&guard.dat_entries, &profile.categories, |entry| {
            entry.category.clone().into_iter().collect()
        });
    let language_options = build_filter_options(&guard.dat_entries, &profile.languages, |entry| {
        entry.languages.clone()
    });
    let region_options = build_filter_options(&guard.dat_entries, &profile.regions, |entry| {
        dedup_engine::extract_region(&entry.description)
            .into_iter()
            .collect()
    });
    let manufacturer_options =
        build_filter_options(&guard.dat_entries, &profile.manufacturers, |entry| {
            vec![entry.manufacturer.clone()]
        });

    drop(guard);

    window.set_category_selected_count(category_options.iter().filter(|o| o.selected).count() as i32);
    window.set_language_selected_count(language_options.iter().filter(|o| o.selected).count() as i32);
    window.set_region_selected_count(region_options.iter().filter(|o| o.selected).count() as i32);
    window
        .set_manufacturer_selected_count(manufacturer_options.iter().filter(|o| o.selected).count() as i32);

    window.set_category_options(category_options);
    window.set_language_options(language_options);
    window.set_region_options(region_options);
    window.set_manufacturer_options(manufacturer_options);

    window.set_filter_year_min_text(
        profile
            .year_min
            .map(|y| y.to_string())
            .unwrap_or_default()
            .into(),
    );
    window.set_filter_year_max_text(
        profile
            .year_max
            .map(|y| y.to_string())
            .unwrap_or_default()
            .into(),
    );

    let statuses = &profile.driver_statuses;
    let matches_all = statuses.is_empty();
    window.set_filter_status_good(matches_all || statuses.contains(&DriverStatus::Good));
    window.set_filter_status_imperfect(matches_all || statuses.contains(&DriverStatus::Imperfect));
    window.set_filter_status_preliminary(matches_all || statuses.contains(&DriverStatus::Preliminary));
    window.set_filter_status_unknown(matches_all || statuses.contains(&DriverStatus::Unknown));

    window.set_filter_include_bios(profile.include_bios);
    window.set_filter_include_device(profile.include_device);
    window.set_filter_include_mechanical(profile.include_mechanical);
    window.set_filter_include_adult(profile.include_adult);
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

fn collect_cleanup_targets(state: &AppState) -> Vec<CleanupTarget> {
    state
        .dedup_remove
        .iter()
        .filter_map(|name| {
            let scanned = state.rom_set.entries.get(name)?;
            let path = scanned.file_path.clone()?;
            Some(CleanupTarget {
                name: name.clone(),
                file_path: path,
                reason: "doublon (1G1R)".to_string(),
            })
        })
        .collect()
}

fn setup_cleanup(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state_request = Arc::clone(state);
    window.on_request_cleanup(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let count = {
            let guard = state_request.lock().unwrap();
            collect_cleanup_targets(&guard).len()
        };

        if count == 0 {
            window
                .set_cleanup_status_text("Aucune ROM en double à nettoyer pour le moment.".into());
            return;
        }

        window.set_cleanup_target_count_text(
            format!("{count} ROM(s) en double vont être traitées.").into(),
        );
        window.set_cleanup_confirm_visible(true);
    });

    let weak = window.as_weak();
    window.on_cancel_cleanup(move || {
        if let Some(window) = weak.upgrade() {
            window.set_cleanup_confirm_visible(false);
        }
    });

    let weak = window.as_weak();
    let state = Arc::clone(state);
    window.on_confirm_cleanup(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        window.set_cleanup_confirm_visible(false);

        let (targets, use_recycle_bin, backup_dir) = {
            let guard = state.lock().unwrap();
            let targets = collect_cleanup_targets(&guard);
            (
                targets,
                guard.config.use_recycle_bin,
                guard.config.backup_dir_path.clone(),
            )
        };

        let options = CleanupOptions {
            use_recycle_bin,
            backup_dir: backup_dir
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty()),
            confirmed: true,
        };

        let records = match cleanup_engine::run_cleanup(&targets, &options) {
            Ok(records) => records,
            Err(err) => {
                window.set_cleanup_status_text(format!("Nettoyage annulé : {err}").into());
                return;
            }
        };

        let success_count = records.iter().filter(|r| r.error.is_none()).count();
        let failure_count = records.len() - success_count;

        let kept_records: Vec<cleanup_engine::CleanupRecord> = {
            let guard = state.lock().unwrap();
            guard
                .dedup_keep
                .iter()
                .map(|name| {
                    let file_path = guard
                        .rom_set
                        .entries
                        .get(name)
                        .and_then(|scanned| scanned.file_path.clone())
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    cleanup_engine::CleanupRecord {
                        name: name.clone(),
                        file_path,
                        reason: "meilleur exemplaire du groupe (1G1R)".to_string(),
                        action: "conservé".to_string(),
                        backed_up_to: None,
                        error: None,
                    }
                })
                .collect()
        };
        let mut full_report = records.clone();
        full_report.extend(kept_records);

        let reports_dir = crate::core::config_manager::config_dir().join("reports");
        let _ = std::fs::create_dir_all(&reports_dir);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let json_path = reports_dir.join(format!("cleanup_report_{timestamp}.json"));
        let csv_path = reports_dir.join(format!("cleanup_report_{timestamp}.csv"));
        let _ = report_generator::write_json_report(&full_report, &json_path);
        let _ = report_generator::write_csv_report(&full_report, &csv_path);

        {
            let mut guard = state.lock().unwrap();
            for record in &records {
                if record.error.is_none() {
                    guard.dedup_remove.remove(&record.name);
                    if let Some(scanned) = guard.rom_set.entries.get_mut(&record.name) {
                        scanned.file_path = None;
                        scanned.status = RomStatus::Missing;
                    }
                }
            }
        }

        window.set_cleanup_status_text(
            format!(
                "Nettoyage terminé : {success_count} ROM(s) traitées, {failure_count} échec(s). Rapport : {}",
                json_path.display()
            )
            .into(),
        );

        refresh_results(&window, &state);

        window.set_integrity_status_text("Vérification d'intégrité en cours...".into());
        let weak_verify = weak.clone();
        let state_verify = Arc::clone(&state);
        std::thread::spawn(move || {
            let text = run_post_cleanup_verification(&state_verify);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak_verify.upgrade() {
                    window.set_integrity_status_text(text.into());
                    refresh_results(&window, &state_verify);
                }
            });
        });
    });
}

fn status_kind(status: PluginStatus) -> i32 {
    match status {
        PluginStatus::NotInstalled => 0,
        PluginStatus::Installed => 1,
        PluginStatus::UpdateAvailable => 2,
    }
}

fn status_text(status: PluginStatus) -> &'static str {
    match status {
        PluginStatus::NotInstalled => "Non installé",
        PluginStatus::Installed => "Installé",
        PluginStatus::UpdateAvailable => "Mise à jour disponible",
    }
}

/// Rebuilds the plugin list shown in the UI from the currently known remote
/// plugins (last fetched from GitHub) combined with what is actually
/// installed locally, filtered by the "Plugins" view's search box (matching
/// against name or console family, case-insensitive) and grouped into
/// per-family sections. Grouping and search were added once the catalog
/// reached ~90 systems, at which point a flat list was no longer usable.
fn refresh_plugin_rows(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let guard = state.lock().unwrap();
    let installed = registry::list_installed(&guard.plugins_dir);
    let installed_by_id: HashMap<&str, &StoredManifest> =
        installed.iter().map(|m| (m.id.as_str(), m)).collect();

    let mut entries: Vec<PluginRow> = guard
        .remote_plugins
        .values()
        .map(|remote| {
            let local = installed_by_id.get(remote.manifest.id.as_str()).copied();
            let status = registry::compare_status(local, &remote.manifest.version);
            PluginRow {
                id: remote.manifest.id.clone().into(),
                name: remote.manifest.name.clone().into(),
                console_family: remote.manifest.console_family.clone().into(),
                version: remote.manifest.version.clone().into(),
                status_text: status_text(status).into(),
                status_kind: status_kind(status),
                is_header: false,
            }
        })
        .collect();

    for local in &installed {
        if !guard.remote_plugins.contains_key(&local.id) {
            entries.push(PluginRow {
                id: local.id.clone().into(),
                name: local.name.clone().into(),
                console_family: local.console_family.clone().into(),
                version: local.version.clone().into(),
                status_text: status_text(PluginStatus::Installed).into(),
                status_kind: status_kind(PluginStatus::Installed),
                is_header: false,
            });
        }
    }

    let search = window.get_plugins_search_text().to_string().to_ascii_lowercase();
    if !search.is_empty() {
        entries.retain(|row| {
            row.name.to_ascii_lowercase().contains(&search)
                || row.console_family.to_ascii_lowercase().contains(&search)
        });
    }

    entries.sort_by(|a, b| {
        a.console_family
            .cmp(&b.console_family)
            .then_with(|| a.name.cmp(&b.name))
    });

    let mut rows: Vec<PluginRow> = Vec::with_capacity(entries.len());
    let mut current_family: Option<String> = None;
    for entry in entries {
        let family = entry.console_family.to_string();
        if current_family.as_deref() != Some(family.as_str()) {
            rows.push(PluginRow {
                id: String::new().into(),
                name: String::new().into(),
                console_family: family.clone().into(),
                version: String::new().into(),
                status_text: String::new().into(),
                status_kind: 0,
                is_header: true,
            });
            current_family = Some(family);
        }
        rows.push(entry);
    }

    drop(guard);

    window.set_plugin_rows(ModelRc::new(VecModel::from(rows)));
    refresh_available_systems(window, state);
}

/// Rebuilds the "Système actif" selector shown in Settings: the built-in
/// MAME support plus every plugin currently installed on disk, grouped by
/// console family (the same scaling need as the "Plugins" view, once the
/// number of installed systems grows beyond a handful).
fn refresh_available_systems(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let plugins_dir = state.lock().unwrap().plugins_dir.clone();

    let mut entries = vec![SystemOption {
        id: "mame".into(),
        name: "MAME".into(),
        console_family: "MAME".into(),
        is_header: false,
    }];
    entries.extend(
        registry::list_installed(&plugins_dir)
            .into_iter()
            .map(|manifest| SystemOption {
                id: manifest.id.into(),
                name: manifest.name.into(),
                console_family: manifest.console_family.into(),
                is_header: false,
            }),
    );

    entries.sort_by(|a, b| {
        a.console_family
            .cmp(&b.console_family)
            .then_with(|| a.name.cmp(&b.name))
    });

    let mut systems: Vec<SystemOption> = Vec::with_capacity(entries.len());
    let mut current_family: Option<String> = None;
    for entry in entries {
        let family = entry.console_family.to_string();
        if current_family.as_deref() != Some(family.as_str()) {
            systems.push(SystemOption {
                id: String::new().into(),
                name: String::new().into(),
                console_family: family.clone().into(),
                is_header: true,
            });
            current_family = Some(family);
        }
        systems.push(entry);
    }

    window.set_available_systems(ModelRc::new(VecModel::from(systems)));
}

/// Guards against a saved (or just-changed) `selected_system` pointing at a
/// plugin that is not actually installed on disk — e.g. a `config.json`
/// left over from a plugin that was later removed, or removed by hand
/// outside the app. Without this, `load_reference_database` would fail with
/// a low-level, unhelpful OS error (`LoadLibraryExW failed`) on every scan
/// attempt, and the user would have no way to recover except manually
/// reselecting a system in Settings. If the selected system is missing,
/// this falls back to the built-in `"mame"` system and explains why.
fn reset_selected_system_if_uninstalled(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let mut guard = state.lock().unwrap();
    if guard.config.selected_system == "mame" {
        return;
    }

    let installed = registry::list_installed(&guard.plugins_dir);
    let still_installed = installed.iter().any(|m| m.id == guard.config.selected_system);
    if still_installed {
        return;
    }

    let missing_id = guard.config.selected_system.clone();
    guard.config.selected_system = "mame".to_string();
    drop(guard);

    window.set_selected_system_id("mame".into());
    window.set_settings_status_text(
        format!(
            "Le système « {missing_id} » n'est plus installé : MAME a été sélectionné automatiquement."
        )
        .into(),
    );
}

/// Fetches the list of plugin manifests published in the repository's
/// `plugins` directory. Pairs each `<id>.json` manifest with its sibling
/// `<id>.dll` download URL.
fn fetch_remote_plugins(api_url: &str) -> Result<HashMap<String, RemotePlugin>, GitHubClientError> {
    let files = github_client::fetch_repository_contents(api_url)?;
    let dll_urls: HashMap<String, String> = files
        .iter()
        .filter(|f| f.kind == "file" && f.name.ends_with(".dll"))
        .filter_map(|f| {
            let stem = f.name.strip_suffix(".dll")?;
            let url = f.download_url.clone()?;
            Some((stem.to_string(), url))
        })
        .collect();

    let mut remote_plugins = HashMap::new();
    for file in files.iter().filter(|f: &&RemoteFile| f.name.ends_with(".json")) {
        let Some(stem) = file.name.strip_suffix(".json") else {
            continue;
        };
        let Some(json_url) = &file.download_url else {
            continue;
        };
        let Some(dll_url) = dll_urls.get(stem) else {
            continue;
        };

        let content = github_client::fetch_text(json_url)?;
        let manifest: StoredManifest =
            serde_json::from_str(&content).map_err(|err| GitHubClientError::Json(err.to_string()))?;

        remote_plugins.insert(
            manifest.id.clone(),
            RemotePlugin {
                manifest,
                dll_download_url: dll_url.clone(),
            },
        );
    }

    Ok(remote_plugins)
}

fn setup_plugins(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state_refresh = Arc::clone(state);
    window.on_refresh_plugins(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        window.set_plugins_loading(true);
        window.set_plugins_status_text("Récupération de la liste des plugins...".into());

        let weak_thread = weak.clone();
        let state_thread = Arc::clone(&state_refresh);
        std::thread::spawn(move || {
            let result = fetch_remote_plugins(PLUGIN_REPO_CONTENTS_URL);
            let _ = slint::invoke_from_event_loop(move || {
                let Some(window) = weak_thread.upgrade() else {
                    return;
                };

                window.set_plugins_loading(false);
                match result {
                    Ok(remote_plugins) => {
                        let count = remote_plugins.len();
                        {
                            let mut guard = state_thread.lock().unwrap();
                            guard.remote_plugins = remote_plugins;
                        }
                        window.set_plugins_status_text(
                            if count == 0 {
                                "Aucun plugin publié pour le moment.".to_string()
                            } else {
                                format!("{count} plugin(s) disponible(s).")
                            }
                            .into(),
                        );
                        refresh_plugin_rows(&window, &state_thread);
                    }
                    Err(err) => {
                        window.set_plugins_status_text(
                            format!("Impossible de récupérer la liste des plugins : {err}").into(),
                        );
                        refresh_plugin_rows(&window, &state_thread);
                    }
                }
            });
        });
    });

    let weak = window.as_weak();
    let state_search = Arc::clone(state);
    window.on_search_plugins(move || {
        if let Some(window) = weak.upgrade() {
            refresh_plugin_rows(&window, &state_search);
        }
    });

    let weak = window.as_weak();
    let state_install = Arc::clone(state);
    window.on_install_plugin(move |id| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let id = id.to_string();

        let remote = {
            let guard = state_install.lock().unwrap();
            guard.remote_plugins.get(&id).map(|remote| {
                (
                    guard.plugins_dir.clone(),
                    remote.manifest.clone(),
                    remote.dll_download_url.clone(),
                )
            })
        };

        let Some((plugins_dir, manifest, dll_url)) = remote else {
            return;
        };

        window.set_plugin_install_active(true);
        window.set_plugin_install_progress(0.0);
        window.set_plugin_install_status_text(format!("Téléchargement de {}...", manifest.name).into());

        let weak_thread = weak.clone();
        let state_thread = Arc::clone(&state_install);
        std::thread::spawn(move || {
            let weak_progress = weak_thread.clone();
            let plugin_name = manifest.name.clone();
            let result = registry::install_plugin_from_url_with_progress(
                &plugins_dir,
                &dll_url,
                &manifest,
                move |downloaded, total| {
                    let ratio = total
                        .filter(|&t| t > 0)
                        .map(|t| downloaded as f32 / t as f32)
                        .unwrap_or(0.0);
                    let status = format!("Téléchargement de {plugin_name}... ({downloaded} octets)");
                    let weak_inner = weak_progress.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = weak_inner.upgrade() {
                            window.set_plugin_install_progress(ratio);
                            window.set_plugin_install_status_text(status.into());
                        }
                    });
                },
            );

            let _ = slint::invoke_from_event_loop(move || {
                let Some(window) = weak_thread.upgrade() else {
                    return;
                };
                window.set_plugin_install_active(false);
                match result {
                    Ok(()) => {
                        window.set_plugin_install_status_text(
                            format!("{} installé avec succès.", manifest.name).into(),
                        );
                    }
                    Err(err) => {
                        window.set_plugin_install_status_text(
                            format!("Échec de l'installation : {err}").into(),
                        );
                    }
                }
                refresh_plugin_rows(&window, &state_thread);
            });
        });
    });

    let weak = window.as_weak();
    let state_remove = Arc::clone(state);
    window.on_remove_plugin(move |id| {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let plugins_dir = state_remove.lock().unwrap().plugins_dir.clone();
        match registry::remove_plugin(&plugins_dir, &id) {
            Ok(()) => {
                window.set_plugin_install_status_text(format!("{id} supprimé.").into());
            }
            Err(err) => {
                window.set_plugin_install_status_text(
                    format!("Échec de la suppression de {id} : {err}").into(),
                );
            }
        }
        refresh_plugin_rows(&window, &state_remove);
        reset_selected_system_if_uninstalled(&window, &state_remove);
    });
}

/// Re-scanne le dossier de ROMs après un nettoyage et vérifie que toutes
/// les ROMs qui devaient être conservées (plan 1G1R) sont bien présentes
/// et intactes sur le disque final.
fn run_post_cleanup_verification(state: &Arc<Mutex<AppState>>) -> String {
    let (rom_set_path, dat_entries, dedup_keep) = {
        let guard = state.lock().unwrap();
        (
            guard.config.rom_set_path.clone(),
            guard.dat_entries.clone(),
            guard.dedup_keep.clone(),
        )
    };

    let Some(rom_set_path) = rom_set_path else {
        return String::new();
    };

    let cancel_flag = AtomicBool::new(false);
    let rescanned = match rom_scanner::scan_rom_directory(
        Path::new(&rom_set_path),
        &dat_entries,
        &cancel_flag,
        |_| {},
    ) {
        Ok(rom_set) => rom_set,
        Err(err) => return format!("Vérification d'intégrité impossible : {err}"),
    };

    let report = rom_scanner::verify_integrity(&rescanned, &dedup_keep);

    {
        let mut guard = state.lock().unwrap();
        guard.rom_set = rescanned;
    }

    if report.problems.is_empty() {
        format!(
            "Vérification d'intégrité : {} ROM(s) conservée(s) confirmée(s) intactes.",
            report.verified_ok.len()
        )
    } else {
        format!(
            "Vérification d'intégrité : {} ROM(s) confirmée(s), {} problème(s) détecté(s) ({}).",
            report.verified_ok.len(),
            report.problems.len(),
            report.problems.join(", ")
        )
    }
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

fn selected_values(options: &ModelRc<FilterOption>) -> Vec<String> {
    options
        .iter()
        .filter(|opt| opt.selected)
        .map(|opt| opt.value.to_string())
        .collect()
}

fn build_filter_options(
    dat_entries: &HashMap<String, RomEntry>,
    currently_selected: &[String],
    extract: impl Fn(&RomEntry) -> Vec<String>,
) -> ModelRc<FilterOption> {
    let mut distinct: Vec<String> = dat_entries
        .values()
        .flat_map(&extract)
        .filter(|value| !value.is_empty())
        .collect();
    distinct.sort();
    distinct.dedup();

    let options: Vec<FilterOption> = distinct
        .into_iter()
        .map(|value| {
            let selected = currently_selected
                .iter()
                .any(|selected| selected.eq_ignore_ascii_case(&value));
            FilterOption {
                value: value.into(),
                selected,
            }
        })
        .collect();

    ModelRc::new(VecModel::from(options))
}

fn refresh_filter_options(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let guard = state.lock().unwrap();
    let profile = &guard.filter_profile;

    let category_options = build_filter_options(&guard.dat_entries, &profile.categories, |entry| {
        entry.category.clone().into_iter().collect()
    });
    let language_options = build_filter_options(&guard.dat_entries, &profile.languages, |entry| {
        entry.languages.clone()
    });
    let region_options = build_filter_options(&guard.dat_entries, &profile.regions, |entry| {
        dedup_engine::extract_region(&entry.description)
            .into_iter()
            .collect()
    });
    let manufacturer_options =
        build_filter_options(&guard.dat_entries, &profile.manufacturers, |entry| {
            vec![entry.manufacturer.clone()]
        });

    drop(guard);

    window.set_category_selected_count(category_options.iter().filter(|o| o.selected).count() as i32);
    window.set_language_selected_count(language_options.iter().filter(|o| o.selected).count() as i32);
    window.set_region_selected_count(region_options.iter().filter(|o| o.selected).count() as i32);
    window
        .set_manufacturer_selected_count(manufacturer_options.iter().filter(|o| o.selected).count() as i32);

    window.set_category_options(category_options);
    window.set_language_options(language_options);
    window.set_region_options(region_options);
    window.set_manufacturer_options(manufacturer_options);
}

fn toggle_filter_option(options: &ModelRc<FilterOption>, value: &str) -> (ModelRc<FilterOption>, i32) {
    let items: Vec<FilterOption> = options
        .iter()
        .map(|mut opt| {
            if opt.value == value {
                opt.selected = !opt.selected;
            }
            opt
        })
        .collect();
    let selected_count = items.iter().filter(|opt| opt.selected).count() as i32;
    (ModelRc::new(VecModel::from(items)), selected_count)
}

fn setup_filter_option_toggles(window: &AppWindow, state: &Arc<Mutex<AppState>>) {
    let weak = window.as_weak();
    let state_toggle = Arc::clone(state);
    window.on_toggle_category(move |value| {
        if let Some(window) = weak.upgrade() {
            let (updated, count) = toggle_filter_option(&window.get_category_options(), &value);
            window.set_category_options(updated);
            window.set_category_selected_count(count);
            recompute_live_filter_preview(&window, &state_toggle);
        }
    });

    let weak = window.as_weak();
    let state_toggle = Arc::clone(state);
    window.on_toggle_language(move |value| {
        if let Some(window) = weak.upgrade() {
            let (updated, count) = toggle_filter_option(&window.get_language_options(), &value);
            window.set_language_options(updated);
            window.set_language_selected_count(count);
            recompute_live_filter_preview(&window, &state_toggle);
        }
    });

    let weak = window.as_weak();
    let state_toggle = Arc::clone(state);
    window.on_toggle_region(move |value| {
        if let Some(window) = weak.upgrade() {
            let (updated, count) = toggle_filter_option(&window.get_region_options(), &value);
            window.set_region_options(updated);
            window.set_region_selected_count(count);
            recompute_live_filter_preview(&window, &state_toggle);
        }
    });

    let weak = window.as_weak();
    let state_toggle = Arc::clone(state);
    window.on_toggle_manufacturer(move |value| {
        if let Some(window) = weak.upgrade() {
            let (updated, count) = toggle_filter_option(&window.get_manufacturer_options(), &value);
            window.set_manufacturer_options(updated);
            window.set_manufacturer_selected_count(count);
            recompute_live_filter_preview(&window, &state_toggle);
        }
    });
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
