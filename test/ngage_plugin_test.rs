use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_NO_INTRO_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Tony Hawk's Pro Skater (Europe)">
        <description>Tony Hawk's Pro Skater (Europe)</description>
        <rom name="Tony Hawk's Pro Skater (Europe).bin" size="16384" crc="bc123456"/>
    </game>
    <game name="Tony Hawk's Pro Skater (Europe) (Beta)" cloneof="Tony Hawk's Pro Skater (Europe)">
        <description>Tony Hawk's Pro Skater (Europe) (Beta)</description>
        <rom name="Tony Hawk's Pro Skater (Europe) (Beta).bin" size="16384" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "ngage_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour ngage_plugin");
    assert!(status.success(), "cargo build -p ngage_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("ngage_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_ngage_plugin_and_parses_a_sample_no_intro_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin Nokia N-Gage n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin Nokia N-Gage doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "ngage");
    assert_eq!(manifest.console_family, "Nokia");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_ngage_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin Nokia N-Gage doit analyser un DAT No-Intro");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Tony Hawk's Pro Skater (Europe)", 0xbc123456);
    assert_eq!(matched.as_deref(), Some("Tony Hawk's Pro Skater (Europe)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn ngage_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin Nokia N-Gage doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_ngage_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Tony Hawk's Pro Skater (Europe) (Beta)"];
    assert!(clone.is_clone());
    assert_eq!(clone.clone_of.as_deref(), Some("Tony Hawk's Pro Skater (Europe)"));

    fs::remove_file(&dat_path).unwrap();
}
