pub mod loader;
pub mod mame_plugin;

use std::collections::HashMap;
use std::path::Path;

pub use plugin_interface::{PluginManifest, PluginRomEntry, PluginRomFile, PLUGIN_ABI_VERSION};

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
