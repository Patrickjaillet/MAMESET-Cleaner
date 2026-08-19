use std::collections::HashMap;
use std::path::PathBuf;

use crate::models::rom_entry::RomEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomStatus {
    Ok,
    Missing,
    Corrupted,
    Unreferenced,
}

#[derive(Debug, Clone)]
pub struct ScannedEntry {
    pub name: String,
    pub metadata: Option<RomEntry>,
    pub file_path: Option<PathBuf>,
    pub status: RomStatus,
}

#[derive(Debug, Clone, Default)]
pub struct RomSet {
    pub entries: HashMap<String, ScannedEntry>,
}

impl RomSet {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn count_by_status(&self, status: RomStatus) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.status == status)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_entries_by_status() {
        let mut set = RomSet::new();
        set.entries.insert(
            "pacman".to_string(),
            ScannedEntry {
                name: "pacman".to_string(),
                metadata: None,
                file_path: Some(PathBuf::from("pacman.zip")),
                status: RomStatus::Ok,
            },
        );
        set.entries.insert(
            "missing".to_string(),
            ScannedEntry {
                name: "missing".to_string(),
                metadata: None,
                file_path: None,
                status: RomStatus::Missing,
            },
        );

        assert_eq!(set.count_by_status(RomStatus::Ok), 1);
        assert_eq!(set.count_by_status(RomStatus::Missing), 1);
        assert_eq!(set.count_by_status(RomStatus::Corrupted), 0);
    }
}
