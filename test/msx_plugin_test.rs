use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_TOSEC_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Metal Gear (1987)(Konami)">
        <description>Metal Gear (1987)(Konami)</description>
        <rom name="Metal Gear (1987)(Konami).rom" size="131072" crc="c5d6e7f8"/>
    </game>
    <game name="Metal Gear (1987)(Konami)[a]" cloneof="Metal Gear (1987)(Konami)">
        <description>Metal Gear (1987)(Konami)[a]</description>
        <rom name="Metal Gear (1987)(Konami)[a].rom" size="131072" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "msx_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour msx_plugin");
    assert!(status.success(), "cargo build -p msx_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("msx_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_msx_plugin_and_parses_a_sample_tosec_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin MSX n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin MSX doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "msx");
    assert_eq!(manifest.console_family, "Microsoft/ASCII");
    assert_eq!(manifest.dat_format, "TOSEC");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_msx_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_TOSEC_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin MSX doit analyser un DAT TOSEC");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Metal Gear (1987)(Konami)", 0xc5d6_e7f8);
    assert_eq!(matched.as_deref(), Some("Metal Gear (1987)(Konami)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn msx_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin MSX doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_msx_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_TOSEC_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Metal Gear (1987)(Konami)[a]"];
    assert!(clone.is_clone());
    assert_eq!(clone.clone_of.as_deref(), Some("Metal Gear (1987)(Konami)"));

    fs::remove_file(&dat_path).unwrap();
}
