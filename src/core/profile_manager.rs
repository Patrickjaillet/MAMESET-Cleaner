use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::config_manager::config_dir;
use crate::models::filter_profile::FilterProfile;

#[derive(Debug)]
pub enum ProfileError {
    Io(std::io::Error),
    Json(serde_json::Error),
    NotFound(String),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileError::Io(err) => write!(f, "erreur de fichier de profil : {err}"),
            ProfileError::Json(err) => write!(f, "erreur de format du profil : {err}"),
            ProfileError::NotFound(name) => write!(f, "profil introuvable : {name}"),
        }
    }
}

impl std::error::Error for ProfileError {}

impl From<std::io::Error> for ProfileError {
    fn from(err: std::io::Error) -> Self {
        ProfileError::Io(err)
    }
}

impl From<serde_json::Error> for ProfileError {
    fn from(err: serde_json::Error) -> Self {
        ProfileError::Json(err)
    }
}

pub fn default_profiles_dir() -> PathBuf {
    config_dir().join("profiles")
}

pub fn save_profile(profile: &FilterProfile) -> Result<(), ProfileError> {
    save_profile_to(&default_profiles_dir(), profile)
}

pub fn load_profile(name: &str) -> Result<FilterProfile, ProfileError> {
    load_profile_from(&default_profiles_dir(), name)
}

pub fn list_profiles() -> Result<Vec<String>, ProfileError> {
    list_profiles_in(&default_profiles_dir())
}

pub fn delete_profile(name: &str) -> Result<(), ProfileError> {
    delete_profile_from(&default_profiles_dir(), name)
}

pub fn save_profile_to(dir: &Path, profile: &FilterProfile) -> Result<(), ProfileError> {
    fs::create_dir_all(dir)?;
    let content = serde_json::to_string_pretty(profile)?;
    fs::write(profile_path(dir, &profile.name), content)?;
    Ok(())
}

pub fn load_profile_from(dir: &Path, name: &str) -> Result<FilterProfile, ProfileError> {
    let path = profile_path(dir, name);
    let content = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ProfileError::NotFound(name.to_string())
        } else {
            ProfileError::Io(err)
        }
    })?;
    Ok(serde_json::from_str(&content)?)
}

pub fn list_profiles_in(dir: &Path) -> Result<Vec<String>, ProfileError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn delete_profile_from(dir: &Path, name: &str) -> Result<(), ProfileError> {
    let path = profile_path(dir, name);
    fs::remove_file(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ProfileError::NotFound(name.to_string())
        } else {
            ProfileError::Io(err)
        }
    })
}

fn profile_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}.json", sanitize_file_name(name)))
}

fn sanitize_file_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "mameset_cleaner_profiles_{label}_{}",
            std::process::id()
        ))
    }

    #[test]
    fn saves_and_loads_a_profile_round_trip() {
        let dir = temp_dir("roundtrip");
        let _ = fs::remove_dir_all(&dir);

        let profile = FilterProfile {
            name: "Arcade FR".to_string(),
            languages: vec!["French".to_string()],
            ..Default::default()
        };

        save_profile_to(&dir, &profile).unwrap();
        let loaded = load_profile_from(&dir, "Arcade FR").unwrap();

        assert_eq!(loaded.name, "Arcade FR");
        assert_eq!(loaded.languages, vec!["French".to_string()]);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn lists_saved_profiles() {
        let dir = temp_dir("list");
        let _ = fs::remove_dir_all(&dir);

        let a = FilterProfile {
            name: "ProfileA".to_string(),
            ..Default::default()
        };
        let b = FilterProfile {
            name: "ProfileB".to_string(),
            ..Default::default()
        };

        save_profile_to(&dir, &a).unwrap();
        save_profile_to(&dir, &b).unwrap();

        let names = list_profiles_in(&dir).unwrap();
        assert_eq!(names, vec!["ProfileA".to_string(), "ProfileB".to_string()]);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn loading_missing_profile_returns_not_found() {
        let dir = temp_dir("missing");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let result = load_profile_from(&dir, "does-not-exist");
        assert!(matches!(result, Err(ProfileError::NotFound(_))));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn deletes_a_saved_profile() {
        let dir = temp_dir("delete");
        let _ = fs::remove_dir_all(&dir);

        let profile = FilterProfile {
            name: "ToDelete".to_string(),
            ..Default::default()
        };
        save_profile_to(&dir, &profile).unwrap();

        delete_profile_from(&dir, "ToDelete").unwrap();
        assert!(list_profiles_in(&dir).unwrap().is_empty());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sanitizes_unsafe_characters_in_profile_names() {
        let dir = temp_dir("sanitize");
        let _ = fs::remove_dir_all(&dir);

        let profile = FilterProfile {
            name: "Set / Spécial: 1".to_string(),
            ..Default::default()
        };
        save_profile_to(&dir, &profile).unwrap();

        let loaded = load_profile_from(&dir, "Set / Spécial: 1").unwrap();
        assert_eq!(loaded.name, "Set / Spécial: 1");

        fs::remove_dir_all(&dir).unwrap();
    }
}
