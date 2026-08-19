use std::collections::HashMap;

use crate::core::dedup_engine::extract_region;
use crate::models::filter_profile::FilterProfile;
use crate::models::rom_entry::RomEntry;

/// Retourne `true` si la catégorie d'une ROM correspond à un contenu
/// classé adulte dans `catver.ini` (suffixe `* Mature *`).
pub fn is_adult_entry(entry: &RomEntry) -> bool {
    entry
        .category
        .as_deref()
        .is_some_and(|category| category.to_ascii_lowercase().contains("mature"))
}

pub fn matches(entry: &RomEntry, profile: &FilterProfile) -> bool {
    matches_categories(entry, profile)
        && matches_languages(entry, profile)
        && matches_regions(entry, profile)
        && matches_driver_status(entry, profile)
        && matches_manufacturers(entry, profile)
        && matches_year_range(entry, profile)
        && matches_type_flags(entry, profile)
}

pub fn apply_filter<'a>(
    entries: &'a HashMap<String, RomEntry>,
    profile: &FilterProfile,
) -> Vec<&'a RomEntry> {
    entries
        .values()
        .filter(|entry| matches(entry, profile))
        .collect()
}

fn matches_categories(entry: &RomEntry, profile: &FilterProfile) -> bool {
    if profile.categories.is_empty() {
        return true;
    }
    match &entry.category {
        Some(category) => profile
            .categories
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(category)),
        None => false,
    }
}

fn matches_languages(entry: &RomEntry, profile: &FilterProfile) -> bool {
    if profile.languages.is_empty() {
        return true;
    }
    entry
        .languages
        .iter()
        .any(|language| profile.languages.iter().any(|allowed| allowed.eq_ignore_ascii_case(language)))
}

fn matches_regions(entry: &RomEntry, profile: &FilterProfile) -> bool {
    if profile.regions.is_empty() {
        return true;
    }
    match extract_region(&entry.description) {
        Some(region) => profile
            .regions
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(&region)),
        None => false,
    }
}

fn matches_driver_status(entry: &RomEntry, profile: &FilterProfile) -> bool {
    if profile.driver_statuses.is_empty() {
        return true;
    }
    profile.driver_statuses.contains(&entry.driver_status)
}

fn matches_manufacturers(entry: &RomEntry, profile: &FilterProfile) -> bool {
    if profile.manufacturers.is_empty() {
        return true;
    }
    profile
        .manufacturers
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(&entry.manufacturer))
}

fn matches_year_range(entry: &RomEntry, profile: &FilterProfile) -> bool {
    if profile.year_min.is_none() && profile.year_max.is_none() {
        return true;
    }
    let Ok(year) = entry.year.trim().parse::<u32>() else {
        return true;
    };
    if let Some(min) = profile.year_min {
        if year < min {
            return false;
        }
    }
    if let Some(max) = profile.year_max {
        if year > max {
            return false;
        }
    }
    true
}

fn matches_type_flags(entry: &RomEntry, profile: &FilterProfile) -> bool {
    if entry.is_bios && !profile.include_bios {
        return false;
    }
    if entry.is_device && !profile.include_device {
        return false;
    }
    if entry.is_mechanical && !profile.include_mechanical {
        return false;
    }
    if is_adult_entry(entry) && !profile.include_adult {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rom_entry::DriverStatus;

    fn base_entry() -> RomEntry {
        RomEntry {
            name: "gamea".to_string(),
            description: "Game A (USA)".to_string(),
            year: "1990".to_string(),
            manufacturer: "Acme".to_string(),
            clone_of: None,
            rom_of: None,
            is_bios: false,
            is_device: false,
            is_mechanical: false,
            runnable: true,
            driver_status: DriverStatus::Good,
            category: Some("Maze".to_string()),
            languages: vec!["English".to_string()],
            roms: Vec::new(),
        }
    }

    #[test]
    fn empty_profile_matches_everything() {
        let entry = base_entry();
        let profile = FilterProfile::default();
        assert!(matches(&entry, &profile));
    }

    #[test]
    fn category_filter_is_an_or_within_the_list() {
        let entry = base_entry();
        let mut profile = FilterProfile::default();
        profile.categories = vec!["Shooter".to_string(), "Maze".to_string()];
        assert!(matches(&entry, &profile));

        profile.categories = vec!["Shooter".to_string()];
        assert!(!matches(&entry, &profile));
    }

    #[test]
    fn multiple_criteria_are_combined_with_and() {
        let entry = base_entry();
        let mut profile = FilterProfile::default();
        profile.categories = vec!["Maze".to_string()];
        profile.manufacturers = vec!["Other".to_string()];

        // La catégorie correspond mais pas le fabricant : le ET doit exclure l'entrée.
        assert!(!matches(&entry, &profile));
    }

    #[test]
    fn year_range_filter() {
        let entry = base_entry();
        let mut profile = FilterProfile::default();
        profile.year_min = Some(1991);
        assert!(!matches(&entry, &profile));

        profile.year_min = Some(1980);
        profile.year_max = Some(2000);
        assert!(matches(&entry, &profile));
    }

    #[test]
    fn region_filter_uses_description_parsing() {
        let entry = base_entry();
        let mut profile = FilterProfile::default();
        profile.regions = vec!["Japan".to_string()];
        assert!(!matches(&entry, &profile));

        profile.regions = vec!["USA".to_string()];
        assert!(matches(&entry, &profile));
    }

    #[test]
    fn driver_status_filter() {
        let entry = base_entry();
        let mut profile = FilterProfile::default();
        profile.driver_statuses = vec![DriverStatus::Preliminary];
        assert!(!matches(&entry, &profile));

        profile.driver_statuses = vec![DriverStatus::Good, DriverStatus::Imperfect];
        assert!(matches(&entry, &profile));
    }

    #[test]
    fn excludes_bios_device_mechanical_and_adult_when_disabled() {
        let mut bios = base_entry();
        bios.is_bios = true;
        let mut device = base_entry();
        device.is_device = true;
        let mut mechanical = base_entry();
        mechanical.is_mechanical = true;
        let mut adult = base_entry();
        adult.category = Some("Mahjong * Mature *".to_string());

        let mut profile = FilterProfile::default();
        profile.include_bios = false;
        profile.include_device = false;
        profile.include_mechanical = false;
        profile.include_adult = false;

        assert!(!matches(&bios, &profile));
        assert!(!matches(&device, &profile));
        assert!(!matches(&mechanical, &profile));
        assert!(!matches(&adult, &profile));
        assert!(matches(&base_entry(), &profile));
    }

    #[test]
    fn apply_filter_returns_only_matching_entries() {
        let mut entries = HashMap::new();
        entries.insert("gamea".to_string(), base_entry());
        let mut other = base_entry();
        other.name = "gameb".to_string();
        other.manufacturer = "Other".to_string();
        entries.insert("gameb".to_string(), other);

        let mut profile = FilterProfile::default();
        profile.manufacturers = vec!["Acme".to_string()];

        let result = apply_filter(&entries, &profile);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "gamea");
    }
}
