use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_NO_INTRO_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Sonic the Hedgehog 2 (World)">
        <description>Sonic the Hedgehog 2 (World)</description>
        <rom name="Sonic the Hedgehog 2 (World).md" size="1048576" crc="c9dbc328"/>
    </game>
    <game name="Sonic the Hedgehog 2 (World) (Beta)" cloneof="Sonic the Hedgehog 2 (World)">
        <description>Sonic the Hedgehog 2 (World) (Beta)</description>
        <rom name="Sonic the Hedgehog 2 (World) (Beta).md" size="1048576" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "genesis_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour genesis_plugin");
    assert!(status.success(), "cargo build -p genesis_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("genesis_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_genesis_plugin_and_parses_a_sample_no_intro_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin Mega Drive/Genesis n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin Mega Drive/Genesis doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "genesis");
    assert_eq!(manifest.console_family, "Sega");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_genesis_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin Mega Drive/Genesis doit analyser un DAT No-Intro");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Sonic the Hedgehog 2 (World)", 0xc9db_c328);
    assert_eq!(matched.as_deref(), Some("Sonic the Hedgehog 2 (World)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn genesis_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin Mega Drive/Genesis doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_genesis_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Sonic the Hedgehog 2 (World) (Beta)"];
    assert!(clone.is_clone());
    assert_eq!(
        clone.clone_of.as_deref(),
        Some("Sonic the Hedgehog 2 (World)")
    );

    fs::remove_file(&dat_path).unwrap();
}
