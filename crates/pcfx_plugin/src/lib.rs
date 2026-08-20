use std::fs;

use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait, std_types::*};
use dat_common::parse_logiqx_dat_panic_safe;
use plugin_interface::{
    PluginManifest, PluginRomEntry, PluginRomFile, RomSystemPlugin_Ref, RomSystemPluginMod,
    PLUGIN_ABI_VERSION,
};

extern "C" fn plugin_abi_version() -> u32 {
    PLUGIN_ABI_VERSION
}

extern "C" fn get_manifest() -> PluginManifest {
    PluginManifest {
        id: "pcfx".into(),
        name: "NEC PC-FX".into(),
        emulator_frontend: "Batocera/Lakka/Recalbox".into(),
        console_family: "NEC".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        dat_format: "Redump".into(),
        sha256: RString::new(),
        min_app_version: "2.4.0".into(),
    }
}

extern "C" fn parse_reference_database(path: RString) -> RResult<RVec<PluginRomEntry>, RString> {
    let content = match fs::read_to_string(&*path) {
        Ok(content) => content,
        Err(err) => return RErr(err.to_string().into()),
    };

    match parse_logiqx_dat_panic_safe(&content) {
        Ok(games) => ROk(RVec::from(
            games.iter().map(to_plugin_entry).collect::<Vec<_>>(),
        )),
        Err(err) => RErr(err.to_string().into()),
    }
}

fn to_plugin_entry(game: &dat_common::LogiqxGame) -> PluginRomEntry {
    PluginRomEntry {
        name: game.name.clone().into(),
        description: game.description.clone().into(),
        year: RString::new(),
        manufacturer: RString::new(),
        clone_of: game.clone_of.clone().unwrap_or_default().into(),
        roms: RVec::from(
            game.roms
                .iter()
                .map(|rom| PluginRomFile {
                    name: rom.name.clone().into(),
                    size: rom.size,
                    crc32: rom.crc32.unwrap_or(0),
                    has_crc32: rom.crc32.is_some(),
                })
                .collect::<Vec<_>>(),
        ),
    }
}

extern "C" fn match_local_roms(
    entries: RVec<PluginRomEntry>,
    local_name: RString,
    local_crc32: u32,
) -> ROption<RString> {
    for entry in entries.iter() {
        let has_matching_rom = entry
            .roms
            .iter()
            .any(|rom| rom.has_crc32 && rom.crc32 == local_crc32);
        let entry_name: &str = entry.name.as_ref();
        let local_name_str: &str = local_name.as_ref();
        if has_matching_rom && entry_name == local_name_str {
            return RSome(entry.name.clone());
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
