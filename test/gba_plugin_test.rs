use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_NO_INTRO_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Pokemon - Emerald Version (USA, Europe)">
        <description>Pokemon - Emerald Version (USA, Europe)</description>
        <rom name="Pokemon - Emerald Version (USA, Europe).gba" size="16777216" crc="106f83e2"/>
    </game>
    <game name="Pokemon - Emerald Version (Japan)" cloneof="Pokemon - Emerald Version (USA, Europe)">
        <description>Pokemon - Emerald Version (Japan)</description>
        <rom name="Pokemon - Emerald Version (Japan).gba" size="16777216" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "gba_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour gba_plugin");
    assert!(status.success(), "cargo build -p gba_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("gba_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_gba_plugin_and_parses_a_sample_no_intro_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin GBA n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin GBA doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "gba");
    assert_eq!(manifest.console_family, "Nintendo");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_gba_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin GBA doit analyser un DAT No-Intro");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(
        &entries,
        "Pokemon - Emerald Version (USA, Europe)",
        0x106f_83e2,
    );
    assert_eq!(
        matched.as_deref(),
        Some("Pokemon - Emerald Version (USA, Europe)")
    );

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn gba_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin GBA doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_gba_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Pokemon - Emerald Version (Japan)"];
    assert!(clone.is_clone());
    assert_eq!(
        clone.clone_of.as_deref(),
        Some("Pokemon - Emerald Version (USA, Europe)")
    );

    fs::remove_file(&dat_path).unwrap();
}
