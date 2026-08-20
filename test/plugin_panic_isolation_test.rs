use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::RomSystem;

/// `cargo test` compiles library crates as rlibs for the test harness; it
/// does not necessarily also emit the `cdylib` artifact. Explicitly building
/// the fixture crate here guarantees the `.dll` this test loads actually
/// exists, regardless of that detail.
fn ensure_panicking_plugin_is_built(profile: &str) {
    let mut args = vec!["build", "-p", "panicking_plugin_fixture"];
    if profile == "release" {
        args.push("--release");
    }
    let status = Command::new(env!("CARGO"))
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("échec du lancement de cargo build pour panicking_plugin_fixture");
    assert!(
        status.success(),
        "cargo build -p panicking_plugin_fixture a échoué"
    );
}

fn panicking_plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    ensure_panicking_plugin_is_built(profile);
    let file_name = format!("panicking_plugin_fixture{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

/// A plugin that panics internally, but catches its own panic before
/// returning across the FFI boundary (the contract every plugin in this
/// workspace follows — see `dat_common::parse_logiqx_dat_panic_safe`), must
/// not crash the host process: the call should come back as a clean `Err`,
/// and the host must remain able to keep calling into the (still loaded)
/// plugin afterwards. An uncaught panic crossing an `extern "C"` boundary
/// is a different story — it aborts the whole process regardless of
/// anything the host does, which is exactly why this contract exists.
#[test]
fn a_panicking_plugin_does_not_crash_the_host_process() {
    let plugin_path = panicking_plugin_path();
    assert!(
        plugin_path.exists(),
        "le plugin de test panicking_plugin_fixture n'a pas été compilé : {}",
        plugin_path.display()
    );

    let plugin =
        load_plugin_from_file(&plugin_path).expect("le plugin de test doit se charger");
    assert_eq!(plugin.manifest().id, "panicking");

    let dat_path = std::env::temp_dir().join(format!(
        "mameset_cleaner_panicking_plugin_dat_{}.txt",
        std::process::id()
    ));
    std::fs::write(&dat_path, "irrelevant content").unwrap();

    let result = plugin.parse_reference_database(&dat_path);
    assert!(
        result.is_err(),
        "un plugin qui panique doit renvoyer une erreur, pas faire planter l'hôte"
    );

    let entries = std::collections::HashMap::new();
    let matched = plugin.match_local_rom(&entries, "anything", 0);
    assert_eq!(
        matched, None,
        "un plugin qui panique dans match_local_rom doit renvoyer None sans planter l'hôte"
    );

    // The host process is still alive and can keep making calls: proof
    // that the earlier panics were contained rather than propagated.
    assert_eq!(plugin.manifest().id, "panicking");

    std::fs::remove_file(&dat_path).unwrap();
}
