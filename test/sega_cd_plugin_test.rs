use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_REDUMP_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Sonic CD (USA)">
        <description>Sonic CD (USA)</description>
        <rom name="Sonic CD (USA) (Track 01).bin" size="12345600" crc="7a3f8b1c"/>
    </game>
    <game name="Sonic CD (Europe)" cloneof="Sonic CD (USA)">
        <description>Sonic CD (Europe)</description>
        <rom name="Sonic CD (Europe) (Track 01).bin" size="12345600" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "sega_cd_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour sega_cd_plugin");
    assert!(status.success(), "cargo build -p sega_cd_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("sega_cd_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_sega_cd_plugin_and_parses_a_sample_redump_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin Sega CD n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin Sega CD doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "sega_cd");
    assert_eq!(manifest.console_family, "Sega");
    assert_eq!(manifest.dat_format, "Redump");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_sega_cd_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_REDUMP_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin Sega CD doit analyser un DAT Redump");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Sonic CD (USA)", 0x7a3f_8b1c);
    assert_eq!(matched.as_deref(), Some("Sonic CD (USA)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn sega_cd_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin Sega CD doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_sega_cd_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_REDUMP_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Sonic CD (Europe)"];
    assert!(clone.is_clone());
    assert_eq!(clone.clone_of.as_deref(), Some("Sonic CD (USA)"));

    fs::remove_file(&dat_path).unwrap();
}
