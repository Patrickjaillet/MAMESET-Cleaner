use std::fs;

use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait, std_types::*};
use plugin_interface::{
    PluginManifest, PluginRomEntry, PluginRomFile, RomSystemPlugin_Ref, RomSystemPluginMod,
    PLUGIN_ABI_VERSION,
};

extern "C" fn plugin_abi_version() -> u32 {
    PLUGIN_ABI_VERSION
}

extern "C" fn get_manifest() -> PluginManifest {
    PluginManifest {
        id: "fake".into(),
        name: "Fake System (test fixture)".into(),
        emulator_frontend: "Batocera".into(),
        console_family: "Test".into(),
        version: "1.1.0".into(),
        dat_format: "fake".into(),
        sha256: RString::new(),
        min_app_version: "1.1.0".into(),
    }
}

extern "C" fn parse_reference_database(path: RString) -> RResult<RVec<PluginRomEntry>, RString> {
    match fs::read_to_string(&*path) {
        Ok(_) => {
            let entry = PluginRomEntry {
                name: "fakegame".into(),
                description: "Fake Game".into(),
                year: "2026".into(),
                manufacturer: "Fixture".into(),
                clone_of: RString::new(),
                roms: RVec::from(vec![PluginRomFile {
                    name: "fakegame.bin".into(),
                    size: 4,
                    crc32: 0xDEAD_BEEF,
                    has_crc32: true,
                }]),
            };
            ROk(RVec::from(vec![entry]))
        }
        Err(err) => RErr(err.to_string().into()),
    }
}

extern "C" fn match_local_roms(
    entries: RVec<PluginRomEntry>,
    _local_name: RString,
    local_crc32: u32,
) -> ROption<RString> {
    for entry in entries.iter() {
        for rom in entry.roms.iter() {
            if rom.has_crc32 && rom.crc32 == local_crc32 {
                return RSome(entry.name.clone());
            }
        }
    }
    RNone
}

#[export_root_module]
pub fn get_root_module() -> RomSystemPlugin_Ref {
    RomSystemPluginMod {
        plugin_abi_version,
        get_manifest,
        parse_reference_database,
        match_local_roms,
    }
    .leak_into_prefix()
}
