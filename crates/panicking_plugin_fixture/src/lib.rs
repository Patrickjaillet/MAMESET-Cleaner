use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait, std_types::*};
use plugin_interface::{
    PluginManifest, PluginRomEntry, RomSystemPlugin_Ref, RomSystemPluginMod, PLUGIN_ABI_VERSION,
};

extern "C" fn plugin_abi_version() -> u32 {
    PLUGIN_ABI_VERSION
}

extern "C" fn get_manifest() -> PluginManifest {
    PluginManifest {
        id: "panicking".into(),
        name: "Panicking Fixture (test)".into(),
        emulator_frontend: "Batocera".into(),
        console_family: "Test".into(),
        version: "1.8.0".into(),
        dat_format: "fake".into(),
        sha256: RString::new(),
        min_app_version: "1.8.0".into(),
    }
}

/// Deliberately panics internally, but — like every real plugin in this
/// workspace — catches its own panic with `catch_unwind` before returning
/// across the `extern "C"` FFI boundary. Letting a panic unwind across that
/// boundary uncaught aborts the whole host process (see the panic-safety
/// notes on `dat_common::parse_logiqx_dat_panic_safe` and on
/// `mameset_cleaner::plugin::loader::LoadedPlugin`); this fixture exists to
/// prove that a plugin following the contract cannot bring the host down
/// even when its own logic panics.
extern "C" fn parse_reference_database(_path: RString) -> RResult<RVec<PluginRomEntry>, RString> {
    match std::panic::catch_unwind(|| -> Vec<PluginRomEntry> {
        panic!("this test fixture always panics from parse_reference_database");
    }) {
        Ok(entries) => ROk(RVec::from(entries)),
        Err(_) => RErr("le plugin a rencontré une erreur interne (panique)".into()),
    }
}

extern "C" fn match_local_roms(
    _entries: RVec<PluginRomEntry>,
    _local_name: RString,
    _local_crc32: u32,
) -> ROption<RString> {
    match std::panic::catch_unwind(|| -> Option<RString> {
        panic!("this test fixture always panics from match_local_roms");
    }) {
        Ok(matched) => matched.into(),
        Err(_) => RNone,
    }
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
