use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_REDUMP_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Shadow of the Colossus (USA)">
        <description>Shadow of the Colossus (USA)</description>
        <rom name="Shadow of the Colossus (USA).iso" size="4489560064" crc="3a2b1c0d"/>
    </game>
    <game name="Shadow of the Colossus (Europe)" cloneof="Shadow of the Colossus (USA)">
        <description>Shadow of the Colossus (Europe)</description>
        <rom name="Shadow of the Colossus (Europe).iso" size="4489560064" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "ps2_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour ps2_plugin");
    assert!(status.success(), "cargo build -p ps2_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("ps2_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_ps2_plugin_and_parses_a_sample_redump_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin PlayStation 2 n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin PlayStation 2 doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "ps2");
    assert_eq!(manifest.console_family, "Sony");
    assert_eq!(manifest.dat_format, "Redump");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_ps2_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_REDUMP_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin PlayStation 2 doit analyser un DAT Redump");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Shadow of the Colossus (USA)", 0x3a2b_1c0d);
    assert_eq!(matched.as_deref(), Some("Shadow of the Colossus (USA)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn ps2_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin PlayStation 2 doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_ps2_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_REDUMP_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Shadow of the Colossus (Europe)"];
    assert!(clone.is_clone());
    assert_eq!(
        clone.clone_of.as_deref(),
        Some("Shadow of the Colossus (USA)")
    );

    fs::remove_file(&dat_path).unwrap();
}
