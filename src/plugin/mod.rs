pub mod github_client;
pub mod loader;
pub mod mame_plugin;
pub mod registry;

use std::collections::HashMap;
use std::path::Path;

pub use plugin_interface::{PluginManifest, PluginRomEntry, PluginRomFile, PLUGIN_ABI_VERSION};

use crate::models::rom_entry::{DriverStatus, RomEntry, RomFile};

/// Abstraction over a ROM reference system (MAME, or any console handled by
/// a dynamically loaded plugin), so the rest of the application does not
/// need to know whether it is talking to the built-in MAME support or to a
/// plugin loaded from a `.dll`.
pub trait RomSystem {
    fn manifest(&self) -> PluginManifest;

    fn parse_reference_database(
        &self,
        path: &Path,
    ) -> Result<HashMap<String, PluginRomEntry>, String>;

    /// Returns the name of the reference entry that `local_crc32` matches,
    /// if any.
    fn match_local_rom(
        &self,
        entries: &HashMap<String, PluginRomEntry>,
        local_name: &str,
        local_crc32: u32,
    ) -> Option<String>;
}

/// Converts a plugin's reference database into the [`RomEntry`] model used
/// by the scan/dedup/filter engines, so any loaded plugin can drive the same
/// pipeline that was originally written for MAME. Fields that a plugin's
/// reference format does not carry (driver status, BIOS/device/mechanical
/// flags, category, languages) are set to sensible neutral defaults rather
/// than left unset, since every ROM known to a non-MAME plugin is, by
/// definition, a runnable, non-BIOS, non-device game.
pub fn plugin_entries_to_rom_entries(entries: HashMap<String, PluginRomEntry>) -> HashMap<String, RomEntry> {
    entries
        .into_iter()
        .map(|(name, entry)| (name, plugin_entry_to_rom_entry(&entry)))
        .collect()
}

fn plugin_entry_to_rom_entry(entry: &PluginRomEntry) -> RomEntry {
    let clone_of_str: &str = entry.clone_of.as_ref();

    RomEntry {
        name: entry.name.to_string(),
        description: entry.description.to_string(),
        year: entry.year.to_string(),
        manufacturer: entry.manufacturer.to_string(),
        clone_of: (!clone_of_str.is_empty()).then(|| clone_of_str.to_string()),
        rom_of: None,
        is_bios: false,
        is_device: false,
        is_mechanical: false,
        runnable: true,
        driver_status: DriverStatus::Good,
        category: None,
        languages: Vec::new(),
        roms: entry
            .roms
            .iter()
            .map(|rom| RomFile {
                name: rom.name.to_string(),
                size: rom.size,
                crc32: rom.has_crc32.then_some(rom.crc32),
                sha1: None,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_plugin_entries_into_rom_entries_with_neutral_defaults() {
        let mut entries = HashMap::new();
        entries.insert(
            "smb".to_string(),
            PluginRomEntry {
                name: "smb".into(),
                description: "Super Mario Bros.".into(),
                year: String::new().into(),
                manufacturer: String::new().into(),
                clone_of: String::new().into(),
                roms: vec![PluginRomFile {
                    name: "smb.nes".into(),
                    size: 40976,
                    crc32: 0xd445_f698,
                    has_crc32: true,
                }]
                .into(),
            },
        );

        let converted = plugin_entries_to_rom_entries(entries);
        let entry = &converted["smb"];
        assert_eq!(entry.description, "Super Mario Bros.");
        assert_eq!(entry.clone_of, None);
        assert!(entry.runnable);
        assert!(!entry.is_bios);
        assert_eq!(entry.driver_status, DriverStatus::Good);
        assert_eq!(entry.roms[0].crc32, Some(0xd445_f698));
    }

    #[test]
    fn a_non_empty_clone_of_is_preserved() {
        let mut entries = HashMap::new();
        entries.insert(
            "smb_rev1".to_string(),
            PluginRomEntry {
                name: "smb_rev1".into(),
                description: "Super Mario Bros. (Rev 1)".into(),
                year: String::new().into(),
                manufacturer: String::new().into(),
                clone_of: "smb".into(),
                roms: Vec::new().into(),
            },
        );

        let converted = plugin_entries_to_rom_entries(entries);
        assert_eq!(converted["smb_rev1"].clone_of.as_deref(), Some("smb"));
    }
}
