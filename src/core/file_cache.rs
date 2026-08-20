use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Identifies a specific version of a file on disk: its path, size, and
/// modification time. Two fingerprints are equal only if all three match,
/// which is enough to treat "same fingerprint" as "same parsed content"
/// without hashing the (potentially huge) file contents themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileFingerprint {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
    /// Extra context beyond the file itself that the cached value depends
    /// on (e.g. which plugin system a DAT was parsed with). Empty string
    /// when the cached value depends only on the file's contents.
    discriminator: String,
}

impl FileFingerprint {
    fn of(path: &Path, discriminator: &str) -> std::io::Result<Self> {
        let metadata = std::fs::metadata(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified()?,
            discriminator: discriminator.to_string(),
        })
    }
}

/// A single-slot, in-session cache keyed by [`FileFingerprint`]: as long as
/// the same file (same path, size, and modified time) and discriminator are
/// requested again, the previously parsed value is returned instead of
/// re-parsing. Any change — a different path, an edited file, or a
/// different discriminator (e.g. switching the active system) — triggers a
/// real re-parse and replaces the cached value.
#[derive(Default)]
pub struct FileCache<T> {
    fingerprint: Option<FileFingerprint>,
    value: Option<T>,
}

impl<T: Clone> FileCache<T> {
    pub fn get_or_parse<E>(
        &mut self,
        path: &Path,
        discriminator: &str,
        parse: impl FnOnce(&Path) -> Result<T, E>,
        io_err: impl FnOnce(std::io::Error) -> E,
    ) -> Result<T, E> {
        let fingerprint = match FileFingerprint::of(path, discriminator) {
            Ok(fingerprint) => fingerprint,
            Err(err) => return Err(io_err(err)),
        };

        if self.fingerprint.as_ref() == Some(&fingerprint) {
            if let Some(value) = &self.value {
                return Ok(value.clone());
            }
        }

        let value = parse(path)?;
        self.fingerprint = Some(fingerprint);
        self.value = Some(value.clone());
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn returns_cached_value_when_the_file_is_unchanged() {
        let dir = std::env::temp_dir().join(format!("mameset_cleaner_cache_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("unchanged.txt");
        std::fs::write(&path, "hello").unwrap();

        let mut cache: FileCache<String> = FileCache::default();
        let calls = Cell::new(0);

        let parse = |p: &Path| -> Result<String, std::io::Error> {
            calls.set(calls.get() + 1);
            std::fs::read_to_string(p)
        };

        let first = cache.get_or_parse(&path, "", parse, |e| e).unwrap();
        let second = cache.get_or_parse(&path, "", parse, |e| e).unwrap();

        assert_eq!(first, "hello");
        assert_eq!(second, "hello");
        assert_eq!(calls.get(), 1, "second call should have hit the cache");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reparses_when_the_file_content_changes() {
        let dir = std::env::temp_dir().join(format!("mameset_cleaner_cache_test_{}", std::process::id() + 1));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("changed.txt");
        std::fs::write(&path, "hello").unwrap();

        let mut cache: FileCache<String> = FileCache::default();
        let calls = Cell::new(0);
        let parse = |p: &Path| -> Result<String, std::io::Error> {
            calls.set(calls.get() + 1);
            std::fs::read_to_string(p)
        };

        let first = cache.get_or_parse(&path, "", parse, |e| e).unwrap();

        // Ensure the modified-time actually changes even on filesystems
        // with coarse mtime resolution.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, "hello world").unwrap();

        let second = cache.get_or_parse(&path, "", parse, |e| e).unwrap();

        assert_eq!(first, "hello");
        assert_eq!(second, "hello world");
        assert_eq!(calls.get(), 2, "changed file must trigger a real re-parse");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reparses_when_the_discriminator_changes() {
        let dir = std::env::temp_dir().join(format!("mameset_cleaner_cache_test_{}", std::process::id() + 2));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("same.txt");
        std::fs::write(&path, "hello").unwrap();

        let mut cache: FileCache<String> = FileCache::default();
        let calls = Cell::new(0);
        let parse = |p: &Path| -> Result<String, std::io::Error> {
            calls.set(calls.get() + 1);
            std::fs::read_to_string(p)
        };

        cache.get_or_parse(&path, "mame", parse, |e| e).unwrap();
        cache.get_or_parse(&path, "psx", parse, |e| e).unwrap();

        assert_eq!(calls.get(), 2, "a different discriminator must trigger a real re-parse");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
