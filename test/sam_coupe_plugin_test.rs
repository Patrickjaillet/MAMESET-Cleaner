use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_NO_INTRO_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Prince of Persia (1990)(Krisalis)">
        <description>Prince of Persia (1990)(Krisalis)</description>
        <rom name="Prince of Persia (1990)(Krisalis).bin" size="16384" crc="bcdef023"/>
    </game>
    <game name="Prince of Persia (1990)(Krisalis) (Beta)" cloneof="Prince of Persia (1990)(Krisalis)">
        <description>Prince of Persia (1990)(Krisalis) (Beta)</description>
        <rom name="Prince of Persia (1990)(Krisalis) (Beta).bin" size="16384" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "sam_coupe_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour sam_coupe_plugin");
    assert!(status.success(), "cargo build -p sam_coupe_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("sam_coupe_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_sam_coupe_plugin_and_parses_a_sample_no_intro_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin SAM Coupe n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin SAM Coupe doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "sam_coupe");
    assert_eq!(manifest.console_family, "MGT");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_sam_coupe_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin SAM Coupe doit analyser un DAT No-Intro");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Prince of Persia (1990)(Krisalis)", 0xbcdef023);
    assert_eq!(matched.as_deref(), Some("Prince of Persia (1990)(Krisalis)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn sam_coupe_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin SAM Coupe doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_sam_coupe_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Prince of Persia (1990)(Krisalis) (Beta)"];
    assert!(clone.is_clone());
    assert_eq!(clone.clone_of.as_deref(), Some("Prince of Persia (1990)(Krisalis)"));

    fs::remove_file(&dat_path).unwrap();
}
