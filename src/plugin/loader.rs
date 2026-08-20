use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use abi_stable::library::RootModule;

use plugin_interface::{PluginManifest, PluginRomEntry, RomSystemPlugin_Ref, PLUGIN_ABI_VERSION};

use super::RomSystem;

#[derive(Debug)]
pub enum PluginLoadError {
    Library(String),
    IncompatibleAbiVersion { found: u32, expected: u32 },
}

impl fmt::Display for PluginLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginLoadError::Library(msg) => write!(f, "impossible de charger le plugin : {msg}"),
            PluginLoadError::IncompatibleAbiVersion { found, expected } => write!(
                f,
                "version d'interface de plugin incompatible (trouvée {found}, attendue {expected})"
            ),
        }
    }
}

impl std::error::Error for PluginLoadError {}

/// Checks a plugin's reported ABI version against the version this build of
/// the host expects, without touching the filesystem. Kept as a pure
/// function so the rejection logic itself can be unit tested without
/// needing an actual dynamic library.
pub fn check_abi_version(found: u32) -> Result<(), PluginLoadError> {
    if found == PLUGIN_ABI_VERSION {
        Ok(())
    } else {
        Err(PluginLoadError::IncompatibleAbiVersion {
            found,
            expected: PLUGIN_ABI_VERSION,
        })
    }
}

pub struct LoadedPlugin {
    module: RomSystemPlugin_Ref,
}

/// Loads a plugin dynamic library from an exact file path and verifies its
/// ABI version before returning it. A plugin that fails either the
/// structural `abi_stable` layout check or the explicit ABI version check
/// is rejected cleanly (an `Err` is returned, the host never crashes).
pub fn load_plugin_from_file(path: &Path) -> Result<LoadedPlugin, PluginLoadError> {
    let module =
        RomSystemPlugin_Ref::load_from_file(path).map_err(|err| PluginLoadError::Library(err.to_string()))?;

    let found_version = (module.plugin_abi_version())();
    check_abi_version(found_version)?;

    Ok(LoadedPlugin { module })
}

impl RomSystem for LoadedPlugin {
    fn manifest(&self) -> PluginManifest {
        (self.module.get_manifest())()
    }

    fn parse_reference_database(
        &self,
        path: &Path,
    ) -> Result<HashMap<String, PluginRomEntry>, String> {
        let path_str = path.display().to_string();
        let result = (self.module.parse_reference_database())(path_str.into());
        result
            .into_result()
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|entry| (entry.name.to_string(), entry))
                    .collect()
            })
            .map_err(|err| err.to_string())
    }

    fn match_local_rom(
        &self,
        entries: &HashMap<String, PluginRomEntry>,
        local_name: &str,
        local_crc32: u32,
    ) -> Option<String> {
        let all_entries: abi_stable::std_types::RVec<PluginRomEntry> =
            entries.values().cloned().collect();
        let matched: Option<abi_stable::std_types::RString> =
            (self.module.match_local_roms())(all_entries, local_name.into(), local_crc32).into();
        matched.map(|name| name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_matching_abi_version() {
        assert!(check_abi_version(PLUGIN_ABI_VERSION).is_ok());
    }

    #[test]
    fn rejects_mismatched_abi_version_without_panicking() {
        let result = check_abi_version(PLUGIN_ABI_VERSION + 1);
        assert!(matches!(
            result,
            Err(PluginLoadError::IncompatibleAbiVersion { .. })
        ));
    }

    #[test]
    fn loading_a_nonexistent_file_fails_cleanly() {
        let result = load_plugin_from_file(Path::new("does-not-exist.dll"));
        assert!(result.is_err());
    }
}
