use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_TOSEC_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Shadow of the Beast (1989)(Psygnosis)">
        <description>Shadow of the Beast (1989)(Psygnosis)</description>
        <rom name="Shadow of the Beast (1989)(Psygnosis).adf" size="901120" crc="92839394"/>
    </game>
    <game name="Shadow of the Beast (1989)(Psygnosis)[cr FLT]" cloneof="Shadow of the Beast (1989)(Psygnosis)">
        <description>Shadow of the Beast (1989)(Psygnosis)[cr FLT]</description>
        <rom name="Shadow of the Beast (1989)(Psygnosis)[cr FLT].adf" size="901120" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "amiga_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour amiga_plugin");
    assert!(status.success(), "cargo build -p amiga_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("amiga_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_amiga_plugin_and_parses_a_sample_tosec_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin Amiga n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin Amiga doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "amiga");
    assert_eq!(manifest.console_family, "Commodore");
    assert_eq!(manifest.dat_format, "TOSEC");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_amiga_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_TOSEC_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin Amiga doit analyser un DAT TOSEC");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(
        &entries,
        "Shadow of the Beast (1989)(Psygnosis)",
        0x9283_9394,
    );
    assert_eq!(
        matched.as_deref(),
        Some("Shadow of the Beast (1989)(Psygnosis)")
    );

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn amiga_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin Amiga doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_amiga_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_TOSEC_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Shadow of the Beast (1989)(Psygnosis)[cr FLT]"];
    assert!(clone.is_clone());
    assert_eq!(
        clone.clone_of.as_deref(),
        Some("Shadow of the Beast (1989)(Psygnosis)")
    );

    fs::remove_file(&dat_path).unwrap();
}
