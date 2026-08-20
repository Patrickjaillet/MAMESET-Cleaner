//! Dev tool: assembles the repository's `plugins/` publishing folder from
//! every `*_plugin.dll` built in `target/release`. For each plugin it loads
//! the real compiled `.dll`, reads its own declared manifest, computes the
//! real SHA-256 of the exact bytes being published, and writes both
//! `plugins/<id>.dll` and `plugins/<id>.json` — the same file layout the
//! GitHub client (`src/plugin/github_client.rs`) expects to find in the
//! repository. Run with `cargo run --release --example publish_plugins`
//! after building the plugin crates in release mode.

use std::fs;
use std::path::Path;

use mameset_cleaner::plugin::github_client::compute_sha256_hex;
use mameset_cleaner::plugin::loader::load_plugin_from_file;
use mameset_cleaner::plugin::registry::StoredManifest;
use mameset_cleaner::plugin::RomSystem;

fn main() {
    let release_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release");
    let plugins_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins");
    fs::create_dir_all(&plugins_dir).expect("impossible de créer le dossier plugins/");

    let mut published = Vec::new();

    for entry in fs::read_dir(&release_dir).expect("impossible de lire target/release") {
        let entry = entry.expect("entrée de dossier invalide");
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !file_name.ends_with("_plugin.dll") {
            continue;
        }

        let plugin = load_plugin_from_file(&path)
            .unwrap_or_else(|err| panic!("échec du chargement de {file_name} : {err}"));
        let manifest = plugin.manifest();
        let id = manifest.id.to_string();

        let dll_dest = plugins_dir.join(format!("{id}.dll"));
        fs::copy(&path, &dll_dest)
            .unwrap_or_else(|err| panic!("échec de la copie de {file_name} : {err}"));

        let sha256 = compute_sha256_hex(&dll_dest)
            .unwrap_or_else(|err| panic!("échec du calcul du SHA-256 pour {id} : {err}"));

        let stored = StoredManifest {
            id: id.clone(),
            name: manifest.name.to_string(),
            emulator_frontend: manifest.emulator_frontend.to_string(),
            console_family: manifest.console_family.to_string(),
            version: manifest.version.to_string(),
            dat_format: manifest.dat_format.to_string(),
            sha256,
            min_app_version: manifest.min_app_version.to_string(),
        };

        let json_dest = plugins_dir.join(format!("{id}.json"));
        let json = serde_json::to_string_pretty(&stored).expect("échec de sérialisation JSON");
        fs::write(&json_dest, json).expect("échec de l'écriture du manifeste JSON");

        println!("publié : {id} ({})", stored.name);
        published.push(id);
    }

    published.sort();
    println!("\n{} plugin(s) publié(s) dans plugins/", published.len());
}
