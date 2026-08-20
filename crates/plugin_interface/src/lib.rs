#![allow(non_camel_case_types)]

use abi_stable::{
    library::RootModule,
    package_version_strings,
    sabi_types::VersionStrings,
    std_types::{ROption, RResult, RString, RVec},
    StableAbi,
};

/// Incremented whenever a change to this interface would break binary
/// compatibility with already-built plugins. Checked explicitly by the
/// host at load time (`RomSystemPlugin::plugin_abi_version`), on top of
/// the structural layout check `abi_stable` already performs.
pub const PLUGIN_ABI_VERSION: u32 = 1;

#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: RString,
    pub name: RString,
    pub emulator_frontend: RString,
    pub console_family: RString,
    pub version: RString,
    pub dat_format: RString,
    pub sha256: RString,
    pub min_app_version: RString,
}

#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Eq)]
pub struct PluginRomFile {
    pub name: RString,
    pub size: u64,
    pub crc32: u32,
    pub has_crc32: bool,
}

#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Eq)]
pub struct PluginRomEntry {
    pub name: RString,
    pub description: RString,
    pub year: RString,
    pub manufacturer: RString,
    pub clone_of: RString,
    pub roms: RVec<PluginRomFile>,
}

/// The root module exported by every ROM system plugin dynamic library.
#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = RomSystemPlugin_Ref)))]
#[sabi(missing_field(panic))]
pub struct RomSystemPluginMod {
    /// Must return [`PLUGIN_ABI_VERSION`]. Checked by the host before any
    /// other function of the plugin is called.
    pub plugin_abi_version: extern "C" fn() -> u32,
    pub get_manifest: extern "C" fn() -> PluginManifest,
    /// Parses the reference database file (DAT) at `path`.
    pub parse_reference_database: extern "C" fn(path: RString) -> RResult<RVec<PluginRomEntry>, RString>,
    /// Given the reference database entries and a local file's name and
    /// CRC32, returns the name of the entry it matches, if any.
    #[sabi(last_prefix_field)]
    pub match_local_roms: extern "C" fn(
        entries: RVec<PluginRomEntry>,
        local_name: RString,
        local_crc32: u32,
    ) -> ROption<RString>,
}

impl RootModule for RomSystemPlugin_Ref {
    abi_stable::declare_root_module_statics! {RomSystemPlugin_Ref}
    const BASE_NAME: &'static str = "rom_system_plugin";
    const NAME: &'static str = "rom_system_plugin";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}
