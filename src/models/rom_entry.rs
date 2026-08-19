use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverStatus {
    Good,
    Imperfect,
    Preliminary,
    Unknown,
}

impl DriverStatus {
    pub fn parse_status(value: &str) -> Self {
        match value {
            "good" => DriverStatus::Good,
            "imperfect" => DriverStatus::Imperfect,
            "preliminary" => DriverStatus::Preliminary,
            _ => DriverStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RomFile {
    pub name: String,
    pub size: u64,
    pub crc32: Option<u32>,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RomEntry {
    pub name: String,
    pub description: String,
    pub year: String,
    pub manufacturer: String,
    pub clone_of: Option<String>,
    pub rom_of: Option<String>,
    pub is_bios: bool,
    pub is_device: bool,
    pub is_mechanical: bool,
    pub runnable: bool,
    pub driver_status: DriverStatus,
    pub category: Option<String>,
    pub languages: Vec<String>,
    pub roms: Vec<RomFile>,
}

impl RomEntry {
    pub fn is_clone(&self) -> bool {
        self.clone_of.is_some()
    }

    pub fn is_parent(&self) -> bool {
        self.clone_of.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_status_parses_known_values() {
        assert_eq!(DriverStatus::parse_status("good"), DriverStatus::Good);
        assert_eq!(DriverStatus::parse_status("imperfect"), DriverStatus::Imperfect);
        assert_eq!(
            DriverStatus::parse_status("preliminary"),
            DriverStatus::Preliminary
        );
        assert_eq!(DriverStatus::parse_status("weird"), DriverStatus::Unknown);
    }

    #[test]
    fn clone_and_parent_detection() {
        let mut entry = RomEntry {
            name: "puckman".into(),
            description: "PuckMan".into(),
            year: "1980".into(),
            manufacturer: "Namco".into(),
            clone_of: None,
            rom_of: None,
            is_bios: false,
            is_device: false,
            is_mechanical: false,
            runnable: true,
            driver_status: DriverStatus::Good,
            category: None,
            languages: vec![],
            roms: vec![],
        };
        assert!(entry.is_parent());
        assert!(!entry.is_clone());

        entry.clone_of = Some("puckman".into());
        assert!(entry.is_clone());
        assert!(!entry.is_parent());
    }
}
