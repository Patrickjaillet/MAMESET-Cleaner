use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::{plugin_entries_to_rom_entries, RomSystem};

const SAMPLE_TOSEC_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
    <game name="Great Giana Sisters, The (1987)(Rainbow Arts)">
        <description>Great Giana Sisters, The (1987)(Rainbow Arts)</description>
        <rom name="Great Giana Sisters, The (1987)(Rainbow Arts).d64" size="174848" crc="81928293"/>
    </game>
    <game name="Great Giana Sisters, The (1987)(Rainbow Arts)[cr Titan]" cloneof="Great Giana Sisters, The (1987)(Rainbow Arts)">
        <description>Great Giana Sisters, The (1987)(Rainbow Arts)[cr Titan]</description>
        <rom name="Great Giana Sisters, The (1987)(Rainbow Arts)[cr Titan].d64" size="174848" crc="aabbccdd"/>
    </game>
</datafile>"#;

fn ensure_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "c64_plugin"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour c64_plugin");
    assert!(status.success(), "cargo build -p c64_plugin a échoué");
}

fn plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_plugin_is_built(profile);
    let file_name = format!("c64_plugin{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

#[test]
fn loads_the_real_c64_plugin_and_parses_a_sample_tosec_dat() {
    let path = plugin_path();
    assert!(path.exists(), "le plugin Commodore 64 n'a pas été compilé : {}", path.display());

    let plugin = load_plugin_from_file(&path).expect("le plugin Commodore 64 doit se charger");
    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "c64");
    assert_eq!(manifest.console_family, "Commodore");
    assert_eq!(manifest.dat_format, "TOSEC");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_c64_plugin_dat_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_TOSEC_DAT).unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin Commodore 64 doit analyser un DAT TOSEC");
    assert_eq!(entries.len(), 2);

    let matched = plugin.match_local_rom(
        &entries,
        "Great Giana Sisters, The (1987)(Rainbow Arts)",
        0x8192_8293,
    );
    assert_eq!(
        matched.as_deref(),
        Some("Great Giana Sisters, The (1987)(Rainbow Arts)")
    );

    fs::remove_file(&dat_path).unwrap();
}

#[test]
fn c64_plugin_entries_convert_into_scan_ready_rom_entries() {
    let path = plugin_path();
    let plugin = load_plugin_from_file(&path).expect("le plugin Commodore 64 doit se charger");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_c64_plugin_convert_{}.dat",
        std::process::id()
    ));
    fs::write(&dat_path, SAMPLE_TOSEC_DAT).unwrap();

    let plugin_entries = plugin.parse_reference_database(&dat_path).unwrap();
    let rom_entries = plugin_entries_to_rom_entries(plugin_entries);

    let clone = &rom_entries["Great Giana Sisters, The (1987)(Rainbow Arts)[cr Titan]"];
    assert!(clone.is_clone());
    assert_eq!(
        clone.clone_of.as_deref(),
        Some("Great Giana Sisters, The (1987)(Rainbow Arts)")
    );

    fs::remove_file(&dat_path).unwrap();
}
