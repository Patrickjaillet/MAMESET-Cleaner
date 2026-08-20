use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::registry::{
    self, install_plugin_from_bytes, list_installed, PluginStatus, StoredManifest,
};
use mameset_cleaner::plugin::{github_client, RomSystem};

fn built_fake_plugin_path() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
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

    let file_name = format!("fake_plugin_fixture{}", std::env::consts::DLL_SUFFIX);
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile)
        .join(file_name)
}

/// Exercises the full download → detection → activation cycle without
/// touching the network: the "download" step is simulated by reading the
/// real compiled fixture plugin's bytes locally (the same bytes a real
/// download would produce), so every other step — SHA-256 verification,
/// on-disk installation, listing/detection, and finally loading the
/// installed `.dll` and calling it through the real FFI boundary — is
/// exercised for real.
#[test]
fn full_install_detect_and_activate_cycle() {
    let source_dll = built_fake_plugin_path();
    let bytes = fs::read(&source_dll).expect("le plugin factice compilé doit être lisible");
    let sha256 = github_client::compute_sha256_hex_bytes(&bytes);

    let plugins_dir = std::env::temp_dir().join(format!(
        "mameset_cleaner_plugin_download_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&plugins_dir);

    let manifest = StoredManifest {
        id: "fake".to_string(),
        name: "Fake System (test fixture)".to_string(),
        emulator_frontend: "Batocera".to_string(),
        console_family: "Test".to_string(),
        version: "1.2.0".to_string(),
        dat_format: "fake".to_string(),
        sha256: sha256.clone(),
        min_app_version: "1.2.0".to_string(),
    };

    // 1. "Download" (simulated) + integrity-checked installation.
    install_plugin_from_bytes(&plugins_dir, &bytes, &manifest)
        .expect("l'installation doit réussir avec un hash valide");

    // 2. Automatic detection: the freshly installed plugin must be listed
    // with no further setup.
    let installed = list_installed(&plugins_dir);
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id, "fake");
    assert_eq!(
        registry::compare_status(installed.first(), &manifest.version),
        PluginStatus::Installed
    );

    // 3. Activation: load the installed `.dll` for real and call it
    // through the actual FFI boundary.
    let dll_path = registry::plugin_dll_path(&plugins_dir, "fake");
    let plugin = load_plugin_from_file(&dll_path).expect("le plugin installé doit se charger");
    assert_eq!(plugin.manifest().id, "fake");

    // `abi_stable`'s `RootModule::load_from_file` intentionally leaks the
    // loaded library for the lifetime of the process, so Windows keeps the
    // `.dll` file locked from here on. Cleanup is therefore best-effort.
    let _ = fs::remove_dir_all(&plugins_dir);
}

#[test]
fn a_newer_remote_version_is_reported_as_an_update() {
    let plugins_dir = std::env::temp_dir().join(format!(
        "mameset_cleaner_plugin_update_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&plugins_dir);

    let bytes = b"placeholder plugin bytes";
    let sha256 = github_client::compute_sha256_hex_bytes(bytes);
    let manifest = StoredManifest {
        id: "fake".to_string(),
        name: "Fake System".to_string(),
        emulator_frontend: "Batocera".to_string(),
        console_family: "Test".to_string(),
        version: "1.0.0".to_string(),
        dat_format: "fake".to_string(),
        sha256,
        min_app_version: "1.2.0".to_string(),
    };

    install_plugin_from_bytes(&plugins_dir, bytes, &manifest).unwrap();

    let installed = list_installed(&plugins_dir);
    assert_eq!(
        registry::compare_status(installed.first(), "1.1.0"),
        PluginStatus::UpdateAvailable
    );

    fs::remove_dir_all(&plugins_dir).unwrap();
}
