use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_NO_INTRO_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Pitfall! (USA)">
        <description>Pitfall! (USA)</description>
        <rom name="Pitfall! (USA).a26" size="4096" crc="123abc45"/>
    </game>
    <game name="Pitfall! (USA) (Beta)" cloneof="Pitfall! (USA)">
        <description>Pitfall! (USA) (Beta)</description>
        <rom name="Pitfall! (USA) (Beta).a26" size="4096" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "atari_2600_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour atari_2600_plugin");
    assert!(status.success(), "cargo build -p atari_2600_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("atari_2600_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_atari_2600_plugin_and_parses_a_sample_no_intro_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin Atari 2600 n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin Atari 2600 doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "atari_2600");
    assert_eq!(manifest.console_family, "Atari");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_atari_2600_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin Atari 2600 doit analyser un DAT No-Intro");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Pitfall! (USA)", 0x123a_bc45);
    assert_eq!(matched.as_deref(), Some("Pitfall! (USA)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn atari_2600_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin Atari 2600 doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_atari_2600_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Pitfall! (USA) (Beta)"];
    assert!(clone.is_clone());
    assert_eq!(clone.clone_of.as_deref(), Some("Pitfall! (USA)"));

    fs::remove_file(&dat_path).unwrap();
}
