use std::fs;
use std::io::Write;
use std::path::PathBuf;

use mameset_cleaner::core::cleanup_engine::{CleanupOptions, CleanupTarget};
use mameset_cleaner::core::report_generator;
use mameset_cleaner::core::{cleanup_engine, dedup_engine};
use mameset_cleaner::models::rom_entry::{DriverStatus, RomEntry};

fn isolated_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mameset_cleaner_cleanup_integration_{label}_{}",
        std::process::id()
    ))
}

fn write_rom_file(path: &PathBuf, content: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content).unwrap();
}

fn entry(name: &str, description: &str, clone_of: Option<&str>) -> RomEntry {
    RomEntry {
        name: name.to_string(),
        description: description.to_string(),
        year: "1990".to_string(),
        manufacturer: "Test".to_string(),
        clone_of: clone_of.map(String::from),
        rom_of: clone_of.map(String::from),
        is_bios: false,
        is_device: false,
        is_mechanical: false,
        runnable: true,
        driver_status: DriverStatus::Good,
        category: None,
        languages: Vec::new(),
        roms: Vec::new(),
    }
}

/// Simule un cycle complet sur un set de ROMs isolé : détection des
/// doublons (v0.4.0), nettoyage réel avec sauvegarde préalable (v0.7.0)
/// et génération du rapport JSON.
#[test]
fn full_cleanup_cycle_on_an_isolated_rom_set() {
    let romset_dir = isolated_dir("romset");
    let backup_dir = isolated_dir("backup");
    let _ = fs::remove_dir_all(&romset_dir);
    let _ = fs::remove_dir_all(&backup_dir);

    let parent_path = romset_dir.join("puckman.zip");
    let clone_path = romset_dir.join("pacman.zip");
    write_rom_file(&parent_path, b"parent-rom-data");
    write_rom_file(&clone_path, b"clone-rom-data");

    let mut entries = std::collections::HashMap::new();
    entries.insert(
        "puckman".to_string(),
        entry("puckman", "PuckMan (Japan)", None),
    );
    entries.insert(
        "pacman".to_string(),
        entry("pacman", "Pac-Man (World)", Some("puckman")),
    );

    let plan = dedup_engine::build_dedup_plan(&entries, &dedup_engine::RegionPriority::default_profile());
    assert_eq!(plan.roms_to_keep(), vec!["pacman".to_string()]);
    assert_eq!(plan.roms_to_remove(), vec!["puckman".to_string()]);

    let targets = vec![CleanupTarget {
        name: "puckman".to_string(),
        file_path: parent_path.clone(),
        reason: "doublon (1G1R)".to_string(),
    }];

    let options = CleanupOptions {
        use_recycle_bin: false,
        backup_dir: Some(backup_dir.clone()),
        confirmed: true,
    };

    let records = cleanup_engine::run_cleanup(&targets, &options).unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].action, "supprimé");
    assert!(!parent_path.exists(), "le doublon doit avoir été supprimé du set");
    assert!(clone_path.exists(), "le meilleur exemplaire doit rester intact");
    assert!(
        backup_dir.join("puckman.zip").exists(),
        "une copie de sauvegarde doit exister avant suppression"
    );

    let report_path = romset_dir.join("report.json");
    report_generator::write_json_report(&records, &report_path).unwrap();
    let report_content = fs::read_to_string(&report_path).unwrap();
    assert!(report_content.contains("puckman"));
    assert!(report_content.contains("doublon (1G1R)"));

    fs::remove_dir_all(&romset_dir).unwrap();
    fs::remove_dir_all(&backup_dir).unwrap();
}

/// Le verrou de confirmation du moteur doit bloquer toute suppression
/// même si un plan de nettoyage a été calculé.
#[test]
fn cleanup_is_blocked_without_engine_side_confirmation() {
    let romset_dir = isolated_dir("locked");
    let _ = fs::remove_dir_all(&romset_dir);
    let file_path = romset_dir.join("gamea.zip");
    write_rom_file(&file_path, b"rom-data");

    let targets = vec![CleanupTarget {
        name: "gamea".to_string(),
        file_path: file_path.clone(),
        reason: "doublon".to_string(),
    }];
    let options = CleanupOptions::default();

    let result = cleanup_engine::run_cleanup(&targets, &options);
    assert!(result.is_err());
    assert!(file_path.exists(), "aucun fichier ne doit être touché sans confirmation");

    fs::remove_dir_all(&romset_dir).unwrap();
}
