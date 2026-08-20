use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_NO_INTRO_DAT: &str = r#"<?xml version="1.0"?>
<!DOCTYPE datafile PUBLIC "-//Logiqx//DTD ROM Management Datafile//EN" "http://www.logiqx.com/Dats/datafile.dtd">
<datafile>
    <header>
        <name>Nintendo - Nintendo Entertainment System</name>
        <version>20240101</version>
    </header>
    <game name="Super Mario Bros. (World)">
        <description>Super Mario Bros. (World)</description>
        <rom name="Super Mario Bros. (World).nes" size="40976" crc="d445f698"/>
    </game>
    <game name="Super Mario Bros. (World) (Rev 1)" cloneof="Super Mario Bros. (World)">
        <description>Super Mario Bros. (World) (Rev 1)</description>
        <rom name="Super Mario Bros. (World) (Rev 1).nes" size="40976" crc="abc12345"/>
    </game>
</datafile>"#;

/// `cargo test` compiles library crates as rlibs for the test harness; it
/// does not necessarily also emit the `cdylib` artifact. Explicitly building
/// the plugin crate here guarantees the `.dll` this test loads actually
/// exists, regardless of that detail.
fn ensure_nes_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "nes_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour nes_plugin");
    assert!(status.success(), "cargo build -p nes_plugin a échoué");
}

fn nes_plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    ensure_nes_plugin_is_built(profile);
    let file_name = format!("nes_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_nes_plugin_and_parses_a_sample_no_intro_dat() {
    let plugin_path = nes_plugin_path();
    assert!(
        plugin_path.exists(),
        "le plugin NES n'a pas été compilé : {}",
        plugin_path.display()
    );

    let plugin = load_plugin_from_file(&plugin_path).expect("le plugin NES doit se charger");

    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "nes");
    assert_eq!(manifest.console_family, "Nintendo");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_nes_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin NES doit analyser un DAT No-Intro");
    assert_eq!(entries.len(), 2);
    assert!(entries.contains_key("Super Mario Bros. (World)"));

    let matched = plugin.match_local_rom(&entries, "Super Mario Bros. (World)", 0xd445_f698);
    assert_eq!(matched.as_deref(), Some("Super Mario Bros. (World)"));

    let unmatched = plugin.match_local_rom(&entries, "Super Mario Bros. (World)", 0x0000_0000);
    assert_eq!(unmatched, None);

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn nes_plugin_entries_convert_into_scan_ready_rom_entries() {
    let plugin_path = nes_plugin_path();
    let plugin = load_plugin_from_file(&plugin_path).expect("le plugin NES doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_nes_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_NO_INTRO_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let parent = &rom_entries["Super Mario Bros. (World)"];
    assert!(parent.is_parent());
    assert_eq!(parent.roms[0].crc32, Some(0xd445_f698));

    let clone = &rom_entries["Super Mario Bros. (World) (Rev 1)"];
    assert!(clone.is_clone());
    assert_eq!(
        clone.clone_of.as_deref(),
        Some("Super Mario Bros. (World)")
    );

    fs::remove_file(&dat_path).unwrap();
}
