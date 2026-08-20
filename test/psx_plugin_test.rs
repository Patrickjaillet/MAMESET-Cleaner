use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_REDUMP_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Final Fantasy VII (USA) (Disc 1)">
        <description>Final Fantasy VII (USA) (Disc 1)</description>
        <rom name="Final Fantasy VII (USA) (Disc 1) (Track 01).bin" size="734003200" crc="2f6419bd"/>
    </game>
    <game name="Final Fantasy VII (Europe) (Disc 1)" cloneof="Final Fantasy VII (USA) (Disc 1)">
        <description>Final Fantasy VII (Europe) (Disc 1)</description>
        <rom name="Final Fantasy VII (Europe) (Disc 1) (Track 01).bin" size="734003200" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "psx_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour psx_plugin");
    assert!(status.success(), "cargo build -p psx_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("psx_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_psx_plugin_and_parses_a_sample_redump_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin PlayStation n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin PlayStation doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "psx");
    assert_eq!(manifest.console_family, "Sony");
    assert_eq!(manifest.dat_format, "Redump");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_psx_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_REDUMP_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin PlayStation doit analyser un DAT Redump");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(&entries, "Final Fantasy VII (USA) (Disc 1)", 0x2f64_19bd);
    assert_eq!(matched.as_deref(), Some("Final Fantasy VII (USA) (Disc 1)"));

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn psx_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin PlayStation doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_psx_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_REDUMP_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Final Fantasy VII (Europe) (Disc 1)"];
    assert!(clone.is_clone());
    assert_eq!(
        clone.clone_of.as_deref(),
        Some("Final Fantasy VII (USA) (Disc 1)")
    );

    fs::remove_file(&dat_path).unwrap();
}
