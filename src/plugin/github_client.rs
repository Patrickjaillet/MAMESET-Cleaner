use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

const USER_AGENT: &str = "MAMESET-Cleaner";

#[derive(Debug)]
pub enum GitHubClientError {
    Http(String),
    Json(String),
    Io(std::io::Error),
    HashMismatch { expected: String, actual: String },
}

impl fmt::Display for GitHubClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitHubClientError::Http(msg) => write!(f, "erreur réseau : {msg}"),
            GitHubClientError::Json(msg) => write!(f, "réponse GitHub invalide : {msg}"),
            GitHubClientError::Io(err) => write!(f, "erreur de fichier : {err}"),
            GitHubClientError::HashMismatch { expected, actual } => write!(
                f,
                "empreinte SHA-256 invalide (attendue {expected}, obtenue {actual}) — fichier rejeté"
            ),
        }
    }
}

impl std::error::Error for GitHubClientError {}

impl From<std::io::Error> for GitHubClientError {
    fn from(err: std::io::Error) -> Self {
        GitHubClientError::Io(err)
    }
}

/// One entry of a GitHub "contents" API response
/// (`GET /repos/{owner}/{repo}/contents/{path}`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RemoteFile {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub download_url: Option<String>,
}

pub fn parse_contents_response(json: &str) -> Result<Vec<RemoteFile>, GitHubClientError> {
    serde_json::from_str(json).map_err(|err| GitHubClientError::Json(err.to_string()))
}

/// Lists the files at `api_url` (a GitHub contents API URL, e.g.
/// `https://api.github.com/repos/Patrickjaillet/MAMESET-Cleaner/contents/plugins`).
pub fn fetch_repository_contents(api_url: &str) -> Result<Vec<RemoteFile>, GitHubClientError> {
    let body = ureq::get(api_url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| GitHubClientError::Http(err.to_string()))?
        .into_string()
        .map_err(|err| GitHubClientError::Http(err.to_string()))?;
    parse_contents_response(&body)
}

/// Fetches the raw text content at `url` (e.g. a manifest JSON file's
/// `download_url` from a GitHub contents API response).
pub fn fetch_text(url: &str) -> Result<String, GitHubClientError> {
    ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| GitHubClientError::Http(err.to_string()))?
        .into_string()
        .map_err(|err| GitHubClientError::Http(err.to_string()))
}

/// Downloads the file at `url` to `destination`, overwriting it if present.
pub fn download_file(url: &str, destination: &Path) -> Result<(), GitHubClientError> {
    download_file_with_progress(url, destination, |_, _| {})
}

/// Same as [`download_file`], but calls `on_progress(bytes_downloaded,
/// total_bytes)` after each chunk is written. `total_bytes` is `None` when
/// the server does not report a `Content-Length`.
pub fn download_file_with_progress(
    url: &str,
    destination: &Path,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<(), GitHubClientError> {
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|err| GitHubClientError::Http(err.to_string()))?;
    let total_bytes = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());

    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination)?;
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;
        on_progress(downloaded, total_bytes);
    }

    Ok(())
}

pub fn compute_sha256_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn compute_sha256_hex(path: &Path) -> Result<String, GitHubClientError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn verify_sha256(path: &Path, expected_hex: &str) -> Result<(), GitHubClientError> {
    let actual = compute_sha256_hex(path)?;
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(GitHubClientError::HashMismatch {
            expected: expected_hex.to_string(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const SAMPLE_CONTENTS_RESPONSE: &str = r#"[
        {
            "name": "nes.dll",
            "path": "plugins/nes.dll",
            "type": "file",
            "download_url": "https://raw.githubusercontent.com/Patrickjaillet/MAMESET-Cleaner/master/plugins/nes.dll"
        },
        {
            "name": "nes.json",
            "path": "plugins/nes.json",
            "type": "file",
            "download_url": "https://raw.githubusercontent.com/Patrickjaillet/MAMESET-Cleaner/master/plugins/nes.json"
        },
        {
            "name": "archive",
            "path": "plugins/archive",
            "type": "dir",
            "download_url": null
        }
    ]"#;

    #[test]
    fn parses_a_real_shaped_github_contents_response() {
        let files = parse_contents_response(SAMPLE_CONTENTS_RESPONSE).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].name, "nes.dll");
        assert_eq!(files[0].kind, "file");
        assert!(files[0].download_url.is_some());
        assert_eq!(files[2].kind, "dir");
        assert!(files[2].download_url.is_none());
    }

    #[test]
    fn invalid_json_is_reported_without_panicking() {
        let result = parse_contents_response("not json");
        assert!(result.is_err());
    }

    fn temp_file(label: &str, content: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "mameset_cleaner_github_client_{label}_{}",
            std::process::id()
        ));
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    const HELLO_WORLD_SHA256: &str =
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

    #[test]
    fn computes_the_known_sha256_of_a_file() {
        let path = temp_file("sha_known", b"hello world");
        let hash = compute_sha256_hex(&path).unwrap();
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, HELLO_WORLD_SHA256);
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn verify_sha256_accepts_a_matching_hash() {
        let path = temp_file("sha_ok", b"hello world");
        assert!(verify_sha256(&path, HELLO_WORLD_SHA256).is_ok());
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn verify_sha256_rejects_a_mismatching_hash() {
        let path = temp_file("sha_bad", b"hello world");
        let wrong_hash = "0".repeat(64);
        let result = verify_sha256(&path, &wrong_hash);
        assert!(matches!(result, Err(GitHubClientError::HashMismatch { .. })));
        fs::remove_file(&path).unwrap();
    }
}
