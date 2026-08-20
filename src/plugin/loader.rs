use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use abi_stable::library::{lib_header_from_raw_library, RawLibrary, RootModule};
use abi_stable::utils::leak_value;

use plugin_interface::{PluginManifest, PluginRomEntry, RomSystemPlugin_Ref, PLUGIN_ABI_VERSION};

use super::RomSystem;

#[derive(Debug)]
pub enum PluginLoadError {
    Library(String),
    IncompatibleAbiVersion { found: u32, expected: u32 },
    IdMismatch { expected: String, found: String },
}

impl fmt::Display for PluginLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginLoadError::Library(msg) => write!(f, "impossible de charger le plugin : {msg}"),
            PluginLoadError::IncompatibleAbiVersion { found, expected } => write!(
                f,
                "version d'interface de plugin incompatible (trouvée {found}, attendue {expected})"
            ),
            PluginLoadError::IdMismatch { expected, found } => write!(
                f,
                "le plugin déclare l'identifiant « {found} » alors que « {expected} » était attendu — fichier rejeté pour éviter tout conflit entre systèmes"
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
///
/// # Why this does not use `RomSystemPlugin_Ref::load_from_file`
///
/// `abi_stable`'s `RootModule::load_from_file` caches the loaded module in a
/// `static` keyed by the *module type*, not by path: "once the root module
/// is loaded, this will return the already loaded root module" regardless
/// of which path is passed on later calls. Since every plugin in this
/// workspace shares the same `RomSystemPlugin_Ref` type, calling it a
/// second time with a *different* plugin's path would silently keep
/// returning the *first* plugin ever loaded in this process — discovered
/// directly while building the tool that publishes all shipped plugins at
/// once (`examples/publish_plugins.rs`), which loads dozens of different
/// `.dll`s in a single process. This function instead replicates
/// `load_from_file`'s own steps manually (see `abi_stable::library`'s
/// module docs) without going through that per-type cache, so every call
/// genuinely loads the library at `path`.
pub fn load_plugin_from_file(path: &Path) -> Result<LoadedPlugin, PluginLoadError> {
    let raw_library =
        RawLibrary::load_at(path).map_err(|err| PluginLoadError::Library(err.to_string()))?;
    // Leaking is deliberate and matches `abi_stable`'s own loading code: the
    // root module loader may do anything incompatible with sound library
    // unloading, so the library is never unloaded for the process's lifetime.
    let raw_library: &'static RawLibrary = leak_value(raw_library);

    let header = unsafe { lib_header_from_raw_library(raw_library) }
        .map_err(|err| PluginLoadError::Library(err.to_string()))?;
    header
        .ensure_layout::<RomSystemPlugin_Ref>()
        .map_err(|err| PluginLoadError::Library(err.to_string()))?;
    let module: RomSystemPlugin_Ref = unsafe { header.init_root_module_with_unchecked_layout() }
        .and_then(RootModule::initialization)
        .map_err(|err| PluginLoadError::Library(err.to_string()))?;

    let found_version = (module.plugin_abi_version())();
    check_abi_version(found_version)?;

    Ok(LoadedPlugin { module })
}

/// Same as [`load_plugin_from_file`], but additionally checks that the
/// plugin's own declared [`PluginManifest::id`] matches `expected_id` (the
/// id its file was located by, e.g. from the local plugin registry). This
/// rejects a plugin that would otherwise silently impersonate another
/// system's identity, which would let two different plugins conflict over
/// the same `system_id`.
pub fn load_plugin_from_file_expecting_id(
    path: &Path,
    expected_id: &str,
) -> Result<LoadedPlugin, PluginLoadError> {
    let plugin = load_plugin_from_file(path)?;
    let declared_id = plugin.manifest().id.to_string();
    if declared_id != expected_id {
        return Err(PluginLoadError::IdMismatch {
            expected: expected_id.to_string(),
            found: declared_id,
        });
    }
    Ok(plugin)
}

/// # Panic safety across the FFI boundary
///
/// Unlike an ordinary Rust function call, a panic that unwinds across an
/// `extern "C"` boundary without being caught first is undefined behavior:
/// in practice, on this toolchain, it aborts the whole host process
/// (`thread caused non-unwinding panic. aborting.`) rather than being
/// observable here as an error. A `catch_unwind` added on the host side,
/// after calling into the plugin, cannot help — by the time control would
/// return here, the process has already aborted while unwinding through the
/// plugin's own `extern "C"` frame. This was confirmed directly while
/// building the v1.8.0 panic-isolation test.
///
/// This means panic containment has to happen on the *plugin's* side of the
/// boundary, before it returns an FFI-safe value. Every plugin shipped in
/// this workspace follows that contract: reference-database parsing goes
/// through `dat_common::parse_logiqx_dat_panic_safe`, which wraps the actual
/// parsing in `catch_unwind` entirely within the plugin's own stack frame.
/// A third-party plugin that does not follow this contract can still crash
/// the host — full protection against that would require process-level
/// sandboxing (a separate plugin host process), which is out of scope here.
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
