use std::collections::HashMap;
use std::path::Path;

use plugin_interface::{PluginManifest, PluginRomEntry, PluginRomFile};

use crate::core::dat_parser;
use crate::models::rom_entry::RomEntry;

use super::RomSystem;

/// MAME support converted into a built-in native plugin: it implements the
/// same [`RomSystem`] abstraction as any dynamically loaded plugin, but is
/// linked directly into the application, so it is always available with no
/// download required.
pub struct MameSystem;

impl RomSystem for MameSystem {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "mame".into(),
            name: "MAME".into(),
            emulator_frontend: "Batocera/Lakka/Recalbox".into(),
            console_family: "Arcade".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            dat_format: "MAME -listxml".into(),
            sha256: String::new().into(),
            min_app_version: "1.1.0".into(),
        }
    }

    fn parse_reference_database(
        &self,
        path: &Path,
    ) -> Result<HashMap<String, PluginRomEntry>, String> {
        let entries = dat_parser::parse_dat_file(path).map_err(|err| err.to_string())?;
        Ok(entries
            .into_iter()
            .map(|(name, entry)| (name, to_plugin_entry(&entry)))
            .collect())
    }

    fn match_local_rom(
        &self,
        entries: &HashMap<String, PluginRomEntry>,
        local_name: &str,
        local_crc32: u32,
    ) -> Option<String> {
        entries.values().find_map(|entry| {
            let matches = entry
                .roms
                .iter()
                .any(|rom| rom.has_crc32 && rom.crc32 == local_crc32);
            let name_matches: &str = entry.name.as_ref();
            (matches && name_matches == local_name).then(|| entry.name.to_string())
        })
    }
}

fn to_plugin_entry(entry: &RomEntry) -> PluginRomEntry {
    PluginRomEntry {
        name: entry.name.clone().into(),
        description: entry.description.clone().into(),
        year: entry.year.clone().into(),
        manufacturer: entry.manufacturer.clone().into(),
        clone_of: entry.clone_of.clone().unwrap_or_default().into(),
        roms: entry
            .roms
            .iter()
            .map(|rom| PluginRomFile {
                name: rom.name.clone().into(),
                size: rom.size,
                crc32: rom.crc32.unwrap_or(0),
                has_crc32: rom.crc32.is_some(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rom_entry::{DriverStatus, RomFile};

    fn sample_entry() -> RomEntry {
        RomEntry {
            name: "puckman".to_string(),
            description: "PuckMan (Japan)".to_string(),
            year: "1980".to_string(),
            manufacturer: "Namco".to_string(),
            clone_of: None,
            rom_of: None,
            is_bios: false,
            is_device: false,
            is_mechanical: false,
            runnable: true,
            driver_status: DriverStatus::Good,
            category: None,
            languages: Vec::new(),
            roms: vec![RomFile {
                name: "pm1.bin".to_string(),
                size: 4096,
                crc32: Some(0x0c94_4964),
                sha1: None,
            }],
        }
    }

    #[test]
    fn manifest_identifies_the_mame_system() {
        let manifest = MameSystem.manifest();
        assert_eq!(manifest.id, "mame");
    }

    #[test]
    fn converts_rom_entry_into_plugin_rom_entry() {
        let entry = to_plugin_entry(&sample_entry());
        assert_eq!(entry.name, "puckman");
        assert_eq!(entry.roms.len(), 1);
        assert_eq!(entry.roms[0].crc32, 0x0c94_4964);
        assert!(entry.roms[0].has_crc32);
    }

    #[test]
    fn matches_local_rom_by_name_and_crc32() {
        let mut entries = HashMap::new();
        entries.insert("puckman".to_string(), to_plugin_entry(&sample_entry()));

        let matched = MameSystem.match_local_rom(&entries, "puckman", 0x0c94_4964);
        assert_eq!(matched.as_deref(), Some("puckman"));

        let mismatched = MameSystem.match_local_rom(&entries, "puckman", 0xdead_beef);
        assert_eq!(mismatched, None);
    }
}
