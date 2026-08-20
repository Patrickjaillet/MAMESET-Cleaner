use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_NO_INTRO_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Astrosmash (USA)">
        <description>Astrosmash (USA)</description>
        <rom name="Astrosmash (USA).bin" size="16384" crc="d2e3f4a5"/>
    </game>
    <game name="Astrosmash (USA) (Beta)" cloneof="Astrosmash (USA)">
        <description>Astrosmash (USA) (Beta)</description>
        <rom name="Astrosmash (USA) (Beta).bin" size="16384" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "intellivision_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour intellivision_plugin");
    assert!(status.success(), "cargo build -p intellivision_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("intellivision_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_intellivision_plugin_and_parses_a_sample_no_intro_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin Mattel Intellivision n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin Mattel Intellivision doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "intellivision");
    assert_eq!(manifest.console_family, "Mattel");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_intellivision_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin Mattel Intellivision doit analyser un DAT No-Intro");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Astrosmash (USA)", 0xd2e3f4a5);
    assert_eq!(matched.as_deref(), Some("Astrosmash (USA)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn intellivision_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin Mattel Intellivision doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_intellivision_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Astrosmash (USA) (Beta)"];
    assert!(clone.is_clone());
    assert_eq!(clone.clone_of.as_deref(), Some("Astrosmash (USA)"));

    fs::remove_file(&dat_path).unwrap();
}
