use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::RomSystem;

/// `cargo test` compiles library crates as rlibs for the test harness; it
/// does not necessarily also emit the `cdylib` artifact. Explicitly
/// building the fixture crate here guarantees the `.dll` this test loads
/// actually exists, regardless of that detail.
fn ensure_fake_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "fake_plugin_fixture"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour fake_plugin_fixture");
    assert!(status.success(), "cargo build -p fake_plugin_fixture a échoué");
}

/// Locates the dynamic library produced by the `fake_plugin_fixture`
/// workspace member.
fn fake_plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    ensure_fake_plugin_is_built(profile);
    let file_name = format!("fake_plugin_fixture{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

/// Loads and unloads the real fake-plugin dynamic library end to end:
/// ABI version check, manifest retrieval, reference database parsing and
/// local ROM matching, all going through the actual `.dll` boundary.
#[test]
fn loads_a_real_plugin_dynamic_library_and_uses_it() {
    let plugin_path = fake_plugin_path();
    assert!(
        plugin_path.exists(),
        "le plugin factice n'a pas été compilé : {}",
        plugin_path.display()
    );

    let plugin = load_plugin_from_file(&plugin_path).expect("le plugin factice doit se charger");

    let manifest = plugin.manifest();
    assert_eq!(manifest.id, "fake");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_fake_plugin_dat_{}.txt",
        std::process::id()
    ));
    fs::write(&dat_path, "fake reference database").unwrap();

    let entries = plugin
        .parse_reference_database(&dat_path)
        .expect("le plugin factice doit analyser la base de reference");
    assert_eq!(entries.len(), 1);
    assert!(entries.contains_key("fakegame"));

    let matched = plugin.match_local_rom(&entries, "fakegame", 0xDEAD_BEEF);
    assert_eq!(matched.as_deref(), Some("fakegame"));

    let unmatched = plugin.match_local_rom(&entries, "fakegame", 0x1234_5678);
    assert_eq!(unmatched, None);

    fs::remove_file(&dat_path).unwrap();
    // `plugin` is dropped here, exercising the unload path.
}

#[test]
fn loading_a_plugin_from_a_missing_path_fails_without_crashing() {
    let result = load_plugin_from_file(Path::new("this-plugin-does-not-exist.dll"));
    assert!(result.is_err());
}
