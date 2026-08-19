use serde::{Deserialize, Serialize};

use crate::models::rom_entry::DriverStatus;

/// Un profil de filtrage combine plusieurs critères. À l'intérieur d'un
/// même critère (ex. plusieurs genres), les valeurs sont combinées avec
/// un OU (n'importe laquelle suffit). Entre les différents critères
/// (genre, langue, région, ...), la combinaison est un ET (tous doivent
/// être satisfaits). Une liste vide signifie "aucune restriction" pour
/// ce critère.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterProfile {
    pub name: String,
    pub categories: Vec<String>,
    pub languages: Vec<String>,
    pub regions: Vec<String>,
    pub driver_statuses: Vec<DriverStatus>,
    pub manufacturers: Vec<String>,
    pub year_min: Option<u32>,
    pub year_max: Option<u32>,
    pub include_bios: bool,
    pub include_device: bool,
    pub include_mechanical: bool,
    pub include_adult: bool,
}

impl Default for FilterProfile {
    fn default() -> Self {
        Self {
            name: "Défaut".to_string(),
            categories: Vec::new(),
            languages: Vec::new(),
            regions: Vec::new(),
            driver_statuses: Vec::new(),
            manufacturers: Vec::new(),
            year_min: None,
            year_max: None,
            include_bios: true,
            include_device: true,
            include_mechanical: true,
            include_adult: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_has_no_restriction() {
        let profile = FilterProfile::default();
        assert!(profile.categories.is_empty());
        assert!(profile.include_bios);
        assert!(profile.include_device);
        assert!(profile.include_mechanical);
        assert!(profile.include_adult);
        assert!(profile.year_min.is_none());
        assert!(profile.year_max.is_none());
    }
}
