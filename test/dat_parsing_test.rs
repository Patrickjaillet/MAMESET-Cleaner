use std::path::PathBuf;

use mameset_cleaner::core::catver_parser::parse_catver_file;
use mameset_cleaner::core::dat_parser::{extract_dat_build_version, merge_metadata, parse_dat_file};
use mameset_cleaner::core::languages_parser::parse_languages_file;
use mameset_cleaner::models::rom_entry::DriverStatus;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test")
        .join("fixtures")
        .join(name)
}

#[test]
fn parses_and_merges_reference_fixture_files_into_unified_rom_entries() {
    let dat_path = fixture("sample_dat.xml");
    let catver_path = fixture("sample_catver.ini");
    let languages_path = fixture("sample_languages.ini");

    let mut entries = parse_dat_file(&dat_path).unwrap();
    let categories = parse_catver_file(&catver_path).unwrap();
    let languages = parse_languages_file(&languages_path).unwrap();

    merge_metadata(&mut entries, &categories, &languages);

    assert_eq!(entries.len(), 3);

    let parent = &entries["puckman"];
    assert_eq!(parent.description, "PuckMan (Japan set 1)");
    assert_eq!(parent.manufacturer, "Namco");
    assert_eq!(parent.category.as_deref(), Some("Maze"));
    assert_eq!(parent.languages, vec!["Japanese".to_string()]);
    assert!(parent.is_parent());
    assert_eq!(parent.driver_status, DriverStatus::Good);
    assert_eq!(parent.roms[0].crc32, Some(0x0c944964));

    let clone = &entries["pacman"];
    assert!(clone.is_clone());
    assert_eq!(clone.clone_of.as_deref(), Some("puckman"));
    assert_eq!(clone.category.as_deref(), Some("Maze"));
    assert_eq!(clone.languages, vec!["English".to_string()]);

    let bios = &entries["pacmanbios"];
    assert!(bios.is_bios);
    assert!(!bios.runnable);
    assert!(bios.category.is_none());
}

#[test]
fn extracts_build_version_from_fixture_dat() {
    let content = std::fs::read_to_string(fixture("sample_dat.xml")).unwrap();
    assert_eq!(
        extract_dat_build_version(&content).as_deref(),
        Some("0.260 (mame0260)")
    );
}
