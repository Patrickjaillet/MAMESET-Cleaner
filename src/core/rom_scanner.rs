use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::core::checksum::crc32_of_reader;
use crate::models::rom_entry::RomEntry;
use crate::models::rom_set::{RomSet, RomStatus, ScannedEntry};

const NON_ROM_EXTENSIONS: [&str; 7] = ["txt", "nfo", "ini", "xml", "dat", "md", "cfg"];

#[derive(Debug)]
pub enum ScanError {
    Io(std::io::Error),
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanError::Io(err) => write!(f, "erreur pendant le scan du dossier de ROMs : {err}"),
        }
    }
}

impl std::error::Error for ScanError {}

impl From<std::io::Error> for ScanError {
    fn from(err: std::io::Error) -> Self {
        ScanError::Io(err)
    }
}

#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub processed: usize,
    pub total: usize,
    pub current_name: String,
}

/// Résultat de la vérification d'intégrité post-nettoyage : compare un
/// `RomSet` fraîchement re-scanné aux ROMs qui devaient être conservées.
#[derive(Debug, Clone, Default)]
pub struct IntegrityReport {
    pub verified_ok: Vec<String>,
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum ArchiveKind {
    Zip,
    SevenZip,
    Directory,
}

#[derive(Debug, Clone)]
struct ScanCandidate {
    name: String,
    kind: ArchiveKind,
    path: PathBuf,
    loose_files: Vec<PathBuf>,
}

struct CandidateResult {
    name: String,
    path: PathBuf,
    files: Vec<(String, u32)>,
    error: Option<String>,
}

/// Scanne récursivement `directory`. `cancel_flag` est vérifié avant le
/// traitement de chaque candidat (archive ou dossier) : dès qu'il passe à
/// `true`, aucun nouveau fichier n'est traité, mais le travail déjà en
/// cours n'est pas interrompu brutalement. Le `RomSet` retourné contient
/// alors uniquement les entrées effectivement scannées (les ROMs pas
/// encore atteintes ne sont pas marquées « manquantes »).
pub fn scan_rom_directory<F>(
    directory: &Path,
    dat_entries: &HashMap<String, RomEntry>,
    cancel_flag: &AtomicBool,
    on_progress: F,
) -> Result<RomSet, ScanError>
where
    F: Fn(ScanProgress) + Sync,
{
    let candidates = collect_candidates(directory)?;
    let total = candidates.len();
    let processed = AtomicUsize::new(0);

    let results: Vec<CandidateResult> = candidates
        .par_iter()
        .filter_map(|candidate| {
            if cancel_flag.load(Ordering::Relaxed) {
                return None;
            }
            let result = process_candidate(candidate);
            let done = processed.fetch_add(1, Ordering::SeqCst) + 1;
            on_progress(ScanProgress {
                processed: done,
                total,
                current_name: candidate.name.clone(),
            });
            Some(result)
        })
        .collect();

    let mut rom_set = RomSet::new();
    let mut found_names: HashSet<String> = HashSet::new();

    for result in results {
        found_names.insert(result.name.clone());
        let metadata = dat_entries.get(&result.name).cloned();
        let status = classify_status(&result, metadata.as_ref());

        rom_set.entries.insert(
            result.name.clone(),
            ScannedEntry {
                name: result.name,
                metadata,
                file_path: Some(result.path),
                status,
            },
        );
    }

    if !cancel_flag.load(Ordering::Relaxed) {
        for (name, entry) in dat_entries {
            if !found_names.contains(name) {
                rom_set.entries.insert(
                    name.clone(),
                    ScannedEntry {
                        name: name.clone(),
                        metadata: Some(entry.clone()),
                        file_path: None,
                        status: RomStatus::Missing,
                    },
                );
            }
        }
    }

    Ok(rom_set)
}

/// Vérifie qu'un `RomSet` re-scanné (typiquement après un nettoyage)
/// contient bien, à l'état `Ok`, toutes les ROMs listées dans
/// `expected_keep` (le plan de conservation calculé par le moteur de
/// dédoublonnage).
pub fn verify_integrity(rom_set: &RomSet, expected_keep: &HashSet<String>) -> IntegrityReport {
    let mut report = IntegrityReport::default();

    for name in expected_keep {
        match rom_set.entries.get(name) {
            Some(scanned) if scanned.status == RomStatus::Ok => {
                report.verified_ok.push(name.clone());
            }
            _ => report.problems.push(name.clone()),
        }
    }

    report
}

fn classify_status(result: &CandidateResult, metadata: Option<&RomEntry>) -> RomStatus {
    if result.error.is_some() {
        return RomStatus::Corrupted;
    }

    match metadata {
        None => RomStatus::Unreferenced,
        Some(entry) => {
            let all_match = entry.roms.iter().all(|expected| {
                result.files.iter().any(|(name, crc)| {
                    *name == expected.name && expected.crc32.map_or(true, |c| c == *crc)
                })
            });

            if all_match {
                RomStatus::Ok
            } else {
                RomStatus::Corrupted
            }
        }
    }
}

fn collect_candidates(root: &Path) -> Result<Vec<ScanCandidate>, ScanError> {
    let mut candidates = Vec::new();
    visit_directory(root, &mut candidates)?;
    Ok(candidates)
}

fn visit_directory(dir: &Path, candidates: &mut Vec<ScanCandidate>) -> Result<(), ScanError> {
    let mut subdirectories = Vec::new();
    let mut loose_files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            subdirectories.push(path);
            continue;
        }

        if !path.is_file() {
            continue;
        }

        match extension_lower(&path).as_deref() {
            Some("zip") => candidates.push(ScanCandidate {
                name: file_stem(&path),
                kind: ArchiveKind::Zip,
                path: path.clone(),
                loose_files: Vec::new(),
            }),
            Some("7z") => candidates.push(ScanCandidate {
                name: file_stem(&path),
                kind: ArchiveKind::SevenZip,
                path: path.clone(),
                loose_files: Vec::new(),
            }),
            Some(ext) if NON_ROM_EXTENSIONS.contains(&ext) => {}
            _ => loose_files.push(path),
        }
    }

    if !loose_files.is_empty() {
        candidates.push(ScanCandidate {
            name: dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string(),
            kind: ArchiveKind::Directory,
            path: dir.to_path_buf(),
            loose_files,
        });
    }

    for sub in subdirectories {
        visit_directory(&sub, candidates)?;
    }

    Ok(())
}

fn process_candidate(candidate: &ScanCandidate) -> CandidateResult {
    let outcome = match candidate.kind {
        ArchiveKind::Zip => process_zip(&candidate.path),
        ArchiveKind::SevenZip => process_seven_zip(&candidate.path),
        ArchiveKind::Directory => process_directory(&candidate.loose_files),
    };

    match outcome {
        Ok(files) => CandidateResult {
            name: candidate.name.clone(),
            path: candidate.path.clone(),
            files,
            error: None,
        },
        Err(err) => CandidateResult {
            name: candidate.name.clone(),
            path: candidate.path.clone(),
            files: Vec::new(),
            error: Some(err),
        },
    }
}

/// Décompresse réellement chaque entrée du zip et recalcule son CRC32 à
/// partir des octets décompressés, plutôt que de faire confiance à la
/// valeur stockée dans l'en-tête. Cela permet de détecter à la fois une
/// ROM différente de celle attendue et une corruption du flux compressé
/// lui-même (en-tête intact mais données illisibles ou modifiées).
fn process_zip(path: &Path) -> Result<Vec<(String, u32)>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut files = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let actual_crc = crc32_of_reader(&mut entry).map_err(|e| e.to_string())?;
        files.push((name, actual_crc));
    }

    Ok(files)
}

/// Décompresse réellement chaque entrée du 7z (le décodeur de blocs de
/// `sevenz-rust` vérifie lui-même l'intégrité interne du flux) et
/// recalcule le CRC32 à partir des octets obtenus.
fn process_seven_zip(path: &Path) -> Result<Vec<(String, u32)>, String> {
    let mut reader = sevenz_rust::SevenZReader::open(path, sevenz_rust::Password::empty())
        .map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    let mut first_error: Option<String> = None;

    reader
        .for_each_entries(|entry, source| {
            if entry.is_directory {
                return Ok(true);
            }
            match crc32_of_reader(source) {
                Ok(crc) => {
                    files.push((entry.name.clone(), crc));
                    Ok(true)
                }
                Err(err) => {
                    first_error = Some(err.to_string());
                    Ok(false)
                }
            }
        })
        .map_err(|e| e.to_string())?;

    if let Some(err) = first_error {
        return Err(err);
    }

    Ok(files)
}

fn process_directory(loose_files: &[PathBuf]) -> Result<Vec<(String, u32)>, String> {
    let mut files = Vec::with_capacity(loose_files.len());
    for path in loose_files {
        let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
        let crc = crc32_of_reader(&mut file).map_err(|e| e.to_string())?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        files.push((name, crc));
    }
    Ok(files)
}

fn extension_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rom_entry::{DriverStatus, RomFile};
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;

    fn make_dat_entry(name: &str, rom_name: &str, crc: u32) -> RomEntry {
        RomEntry {
            name: name.to_string(),
            description: name.to_string(),
            year: "1980".to_string(),
            manufacturer: "Test".to_string(),
            clone_of: None,
            rom_of: None,
            is_bios: false,
            is_device: false,
            is_mechanical: false,
            runnable: true,
            driver_status: DriverStatus::Good,
            category: None,
            languages: Vec::new(),
            roms: vec![RomFile {
                name: rom_name.to_string(),
                size: 4,
                crc32: Some(crc),
                sha1: None,
            }],
        }
    }

    fn write_zip(path: &Path, entry_name: &str, content: &[u8]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer.start_file(entry_name, options).unwrap();
        writer.write_all(content).unwrap();
        writer.finish().unwrap();
    }

    /// Corrompt les octets de données stockées d'une entrée `Stored`
    /// directement sur le disque, sans toucher au CRC déclaré dans
    /// l'en-tête du zip : simule un flux compressé corrompu que la seule
    /// lecture de l'en-tête ne peut pas détecter.
    fn corrupt_stored_content(zip_path: &Path, original_content: &[u8]) {
        let mut bytes = fs::read(zip_path).unwrap();
        let pos = bytes
            .windows(original_content.len())
            .position(|window| window == original_content)
            .expect("contenu non trouvé dans l'archive zip");
        bytes[pos] ^= 0xFF;
        fs::write(zip_path, bytes).unwrap();
    }

    #[test]
    fn detects_ok_missing_corrupted_and_unreferenced_roms() {
        let tmp = std::env::temp_dir().join(format!(
            "mameset_cleaner_scan_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let content = b"data";
        let crc = crc32fast::hash(content);

        write_zip(&tmp.join("gamea.zip"), "gamea.bin", content);
        write_zip(&tmp.join("gameb.zip"), "gameb.bin", b"wrong-content");
        write_zip(&tmp.join("unreferenced.zip"), "unreferenced.bin", content);

        let mut dat = HashMap::new();
        dat.insert("gamea".to_string(), make_dat_entry("gamea", "gamea.bin", crc));
        dat.insert(
            "gameb".to_string(),
            make_dat_entry("gameb", "gameb.bin", crc),
        );
        dat.insert(
            "missinggame".to_string(),
            make_dat_entry("missinggame", "missinggame.bin", crc),
        );

        let cancel = AtomicBool::new(false);
        let rom_set = scan_rom_directory(&tmp, &dat, &cancel, |_| {}).unwrap();

        assert_eq!(rom_set.entries["gamea"].status, RomStatus::Ok);
        assert_eq!(rom_set.entries["gameb"].status, RomStatus::Corrupted);
        assert_eq!(rom_set.entries["missinggame"].status, RomStatus::Missing);
        assert_eq!(
            rom_set.entries["unreferenced"].status,
            RomStatus::Unreferenced
        );

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn detects_real_stream_corruption_that_header_crc_alone_would_miss() {
        let tmp = std::env::temp_dir().join(format!(
            "mameset_cleaner_scan_corrupt_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let content = b"original-data";
        let crc = crc32fast::hash(content);
        let zip_path = tmp.join("gamea.zip");
        write_zip(&zip_path, "gamea.bin", content);
        corrupt_stored_content(&zip_path, content);

        let mut dat = HashMap::new();
        dat.insert("gamea".to_string(), make_dat_entry("gamea", "gamea.bin", crc));

        let cancel = AtomicBool::new(false);
        let rom_set = scan_rom_directory(&tmp, &dat, &cancel, |_| {}).unwrap();

        assert_eq!(rom_set.entries["gamea"].status, RomStatus::Corrupted);

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn pre_cancelled_scan_processes_no_candidate() {
        let tmp = std::env::temp_dir().join(format!(
            "mameset_cleaner_scan_cancel_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let content = b"data";
        let crc = crc32fast::hash(content);
        write_zip(&tmp.join("gamea.zip"), "gamea.bin", content);

        let mut dat = HashMap::new();
        dat.insert("gamea".to_string(), make_dat_entry("gamea", "gamea.bin", crc));

        let cancel = AtomicBool::new(true);
        let rom_set = scan_rom_directory(&tmp, &dat, &cancel, |_| {}).unwrap();

        assert!(
            rom_set.entries.is_empty(),
            "aucun fichier ne doit être traité une fois le scan annulé"
        );

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn verify_integrity_flags_missing_or_corrupted_expected_keep_entries() {
        let mut rom_set = RomSet::new();
        rom_set.entries.insert(
            "gamea".to_string(),
            ScannedEntry {
                name: "gamea".to_string(),
                metadata: None,
                file_path: Some(PathBuf::from("gamea.zip")),
                status: RomStatus::Ok,
            },
        );
        rom_set.entries.insert(
            "gameb".to_string(),
            ScannedEntry {
                name: "gameb".to_string(),
                metadata: None,
                file_path: None,
                status: RomStatus::Missing,
            },
        );

        let mut expected_keep = HashSet::new();
        expected_keep.insert("gamea".to_string());
        expected_keep.insert("gameb".to_string());
        expected_keep.insert("gamec".to_string());

        let report = verify_integrity(&rom_set, &expected_keep);

        assert_eq!(report.verified_ok, vec!["gamea".to_string()]);
        assert_eq!(report.problems.len(), 2);
        assert!(report.problems.contains(&"gameb".to_string()));
        assert!(report.problems.contains(&"gamec".to_string()));
    }

    #[test]
    fn scans_large_rom_set_within_a_reasonable_time() {
        let tmp = std::env::temp_dir().join(format!(
            "mameset_cleaner_scan_perf_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        const ROM_COUNT: usize = 10_000;
        let content = b"rom-content";
        let crc = crc32fast::hash(content);

        let mut dat = HashMap::new();
        for i in 0..ROM_COUNT {
            let name = format!("game{i:05}");
            write_zip(
                &tmp.join(format!("{name}.zip")),
                &format!("{name}.bin"),
                content,
            );
            dat.insert(
                name.clone(),
                make_dat_entry(&name, &format!("{name}.bin"), crc),
            );
        }

        let cancel = AtomicBool::new(false);
        let started = std::time::Instant::now();
        let rom_set = scan_rom_directory(&tmp, &dat, &cancel, |_| {}).unwrap();
        let elapsed = started.elapsed();

        assert_eq!(rom_set.entries.len(), ROM_COUNT);
        assert_eq!(rom_set.count_by_status(RomStatus::Ok), ROM_COUNT);
        assert!(
            elapsed.as_secs() < 60,
            "le scan de {ROM_COUNT} ROMs a pris trop de temps : {elapsed:?}"
        );

        fs::remove_dir_all(&tmp).unwrap();
    }
}
