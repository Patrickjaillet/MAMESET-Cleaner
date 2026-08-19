use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CleanupOptions {
    pub use_recycle_bin: bool,
    pub backup_dir: Option<PathBuf>,
    /// Verrou côté moteur : aucune suppression réelle ne peut avoir lieu
    /// tant que ce champ n'est pas explicitement mis à `true`, en plus de
    /// la confirmation demandée à l'utilisateur dans l'UI.
    pub confirmed: bool,
}

impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            use_recycle_bin: true,
            backup_dir: None,
            confirmed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CleanupTarget {
    pub name: String,
    pub file_path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupRecord {
    pub name: String,
    pub file_path: String,
    pub reason: String,
    pub action: String,
    pub backed_up_to: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum CleanupError {
    NotConfirmed,
}

impl fmt::Display for CleanupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CleanupError::NotConfirmed => write!(
                f,
                "le nettoyage a été refusé : la confirmation utilisateur est manquante"
            ),
        }
    }
}

impl std::error::Error for CleanupError {}

/// Exécute le nettoyage réel sur `targets`. Ne supprime strictement rien
/// tant que `options.confirmed` n'est pas `true` (verrou côté moteur,
/// indépendant de la boîte de dialogue de confirmation affichée par l'UI).
pub fn run_cleanup(
    targets: &[CleanupTarget],
    options: &CleanupOptions,
) -> Result<Vec<CleanupRecord>, CleanupError> {
    if !options.confirmed {
        return Err(CleanupError::NotConfirmed);
    }

    let mut records = Vec::with_capacity(targets.len());

    for target in targets {
        records.push(cleanup_one(target, options));
    }

    Ok(records)
}

fn cleanup_one(target: &CleanupTarget, options: &CleanupOptions) -> CleanupRecord {
    let mut backed_up_to = None;

    if let Some(backup_dir) = &options.backup_dir {
        match backup_file(&target.file_path, backup_dir) {
            Ok(dest) => backed_up_to = Some(dest.display().to_string()),
            Err(err) => {
                return CleanupRecord {
                    name: target.name.clone(),
                    file_path: target.file_path.display().to_string(),
                    reason: target.reason.clone(),
                    action: "échec".to_string(),
                    backed_up_to: None,
                    error: Some(format!("échec de la sauvegarde préalable : {err}")),
                };
            }
        }
    }

    let (action, error) = delete_file(&target.file_path, options.use_recycle_bin);

    CleanupRecord {
        name: target.name.clone(),
        file_path: target.file_path.display().to_string(),
        reason: target.reason.clone(),
        action,
        backed_up_to,
        error,
    }
}

fn backup_file(source: &Path, backup_dir: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(backup_dir)?;
    let file_name = source.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "nom de fichier invalide")
    })?;
    let dest = backup_dir.join(file_name);
    fs::copy(source, &dest)?;
    Ok(dest)
}

fn delete_file(path: &Path, use_recycle_bin: bool) -> (String, Option<String>) {
    if use_recycle_bin {
        match trash::delete(path) {
            Ok(()) => ("corbeille".to_string(), None),
            Err(err) => ("échec".to_string(), Some(err.to_string())),
        }
    } else {
        match fs::remove_file(path) {
            Ok(()) => ("supprimé".to_string(), None),
            Err(err) => ("échec".to_string(), Some(err.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mameset_cleaner_cleanup_{label}_{}",
            std::process::id()
        ))
    }

    fn write_file(path: &Path, content: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content).unwrap();
    }

    #[test]
    fn refuses_to_run_without_confirmation() {
        let target = CleanupTarget {
            name: "gamea".to_string(),
            file_path: PathBuf::from("does-not-matter.zip"),
            reason: "doublon".to_string(),
        };
        let options = CleanupOptions::default();

        let result = run_cleanup(&[target], &options);
        assert!(matches!(result, Err(CleanupError::NotConfirmed)));
    }

    #[test]
    fn deletes_file_permanently_when_recycle_bin_is_disabled() {
        let dir = temp_dir("delete");
        let _ = fs::remove_dir_all(&dir);
        let file_path = dir.join("gamea.zip");
        write_file(&file_path, b"rom-data");

        let target = CleanupTarget {
            name: "gamea".to_string(),
            file_path: file_path.clone(),
            reason: "doublon (1G1R)".to_string(),
        };
        let options = CleanupOptions {
            use_recycle_bin: false,
            backup_dir: None,
            confirmed: true,
        };

        let records = run_cleanup(&[target], &options).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action, "supprimé");
        assert!(records[0].error.is_none());
        assert!(!file_path.exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn backs_up_file_before_deleting_it() {
        let dir = temp_dir("backup");
        let _ = fs::remove_dir_all(&dir);
        let file_path = dir.join("roms").join("gamea.zip");
        write_file(&file_path, b"rom-data");
        let backup_dir = dir.join("backup");

        let target = CleanupTarget {
            name: "gamea".to_string(),
            file_path: file_path.clone(),
            reason: "doublon (1G1R)".to_string(),
        };
        let options = CleanupOptions {
            use_recycle_bin: false,
            backup_dir: Some(backup_dir.clone()),
            confirmed: true,
        };

        let records = run_cleanup(&[target], &options).unwrap();
        assert_eq!(records[0].action, "supprimé");
        assert!(!file_path.exists());
        assert!(backup_dir.join("gamea.zip").exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reports_failure_when_source_file_is_missing() {
        let dir = temp_dir("missing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let target = CleanupTarget {
            name: "ghost".to_string(),
            file_path: dir.join("ghost.zip"),
            reason: "doublon".to_string(),
        };
        let options = CleanupOptions {
            use_recycle_bin: false,
            backup_dir: None,
            confirmed: true,
        };

        let records = run_cleanup(&[target], &options).unwrap();
        assert_eq!(records[0].action, "échec");
        assert!(records[0].error.is_some());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn does_not_delete_when_backup_fails() {
        let dir = temp_dir("backup_fail");
        let _ = fs::remove_dir_all(&dir);
        let file_path = dir.join("gamea.zip");
        write_file(&file_path, b"rom-data");

        // Un fichier existant à la place du dossier de sauvegarde attendu
        // fait échouer `create_dir_all`, ce qui doit annuler la suppression.
        let backup_dir_as_file = dir.join("backup_blocker");
        write_file(&backup_dir_as_file, b"blocker");
        let backup_dir = backup_dir_as_file.join("nested");

        let target = CleanupTarget {
            name: "gamea".to_string(),
            file_path: file_path.clone(),
            reason: "doublon".to_string(),
        };
        let options = CleanupOptions {
            use_recycle_bin: false,
            backup_dir: Some(backup_dir),
            confirmed: true,
        };

        let records = run_cleanup(&[target], &options).unwrap();
        assert_eq!(records[0].action, "échec");
        assert!(file_path.exists(), "le fichier ne doit pas être supprimé si la sauvegarde échoue");

        fs::remove_dir_all(&dir).unwrap();
    }
}
