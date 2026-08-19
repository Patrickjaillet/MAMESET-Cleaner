use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::models::rom_entry::{DriverStatus, RomEntry};

#[derive(Debug, Clone)]
pub struct RegionPriority {
    order: Vec<String>,
}

impl RegionPriority {
    pub fn new(order: Vec<String>) -> Self {
        Self { order }
    }

    /// Priorité par défaut suggérée par le roadmap : World > USA > Europe > Japan.
    pub fn default_profile() -> Self {
        Self::new(
            ["World", "USA", "Europe", "Japan"]
                .into_iter()
                .map(String::from)
                .collect(),
        )
    }

    fn rank(&self, region: Option<&str>) -> usize {
        match region {
            Some(region) => self
                .order
                .iter()
                .position(|candidate| candidate.eq_ignore_ascii_case(region))
                .unwrap_or(self.order.len()),
            None => self.order.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DedupGroup {
    pub parent_name: String,
    pub members: Vec<String>,
    pub keep: String,
    pub remove: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DedupPlan {
    pub groups: Vec<DedupGroup>,
}

impl DedupPlan {
    /// Simulation (dry-run) : la liste des ROMs qui seraient supprimées.
    /// Le moteur ne supprime jamais rien lui-même ; l'action de nettoyage
    /// réelle est effectuée en v0.7.0 à partir de ce plan.
    pub fn roms_to_remove(&self) -> Vec<String> {
        self.groups
            .iter()
            .flat_map(|group| group.remove.iter().cloned())
            .collect()
    }

    pub fn roms_to_keep(&self) -> Vec<String> {
        self.groups.iter().map(|group| group.keep.clone()).collect()
    }
}

pub fn build_dedup_plan(
    entries: &HashMap<String, RomEntry>,
    region_priority: &RegionPriority,
) -> DedupPlan {
    let mut groups_map: HashMap<String, Vec<String>> = HashMap::new();

    for (name, entry) in entries {
        if entry.is_device {
            continue;
        }
        let root = resolve_root_parent(entries, name);
        groups_map.entry(root).or_default().push(name.clone());
    }

    let mut groups: Vec<DedupGroup> = groups_map
        .into_iter()
        .map(|(parent_name, mut members)| {
            members.sort();
            let keep = select_best(entries, &members, region_priority);
            let remove = members
                .iter()
                .filter(|member| **member != keep)
                .cloned()
                .collect();
            DedupGroup {
                parent_name,
                members,
                keep,
                remove,
            }
        })
        .collect();

    groups.sort_by(|a, b| a.parent_name.cmp(&b.parent_name));

    DedupPlan { groups }
}

fn resolve_root_parent(entries: &HashMap<String, RomEntry>, name: &str) -> String {
    let mut current = name.to_string();
    let mut visited = HashSet::new();

    while let Some(entry) = entries.get(&current) {
        let Some(parent) = &entry.clone_of else {
            break;
        };
        if !visited.insert(current.clone()) {
            break;
        }
        current = parent.clone();
    }

    current
}

fn select_best(
    entries: &HashMap<String, RomEntry>,
    members: &[String],
    region_priority: &RegionPriority,
) -> String {
    members
        .iter()
        .min_by(|a, b| compare_candidates(&entries[*a], &entries[*b], region_priority))
        .cloned()
        .expect("un groupe de doublons ne peut pas être vide")
}

fn compare_candidates(a: &RomEntry, b: &RomEntry, region_priority: &RegionPriority) -> Ordering {
    let region_a = extract_region(&a.description);
    let region_b = extract_region(&b.description);

    region_priority
        .rank(region_a.as_deref())
        .cmp(&region_priority.rank(region_b.as_deref()))
        .then_with(|| driver_status_rank(a.driver_status).cmp(&driver_status_rank(b.driver_status)))
        .then_with(|| {
            revision_penalty(&b.description).cmp(&revision_penalty(&a.description))
        })
        .then_with(|| a.name.cmp(&b.name))
}

fn driver_status_rank(status: DriverStatus) -> u8 {
    match status {
        DriverStatus::Good => 0,
        DriverStatus::Imperfect => 1,
        DriverStatus::Preliminary => 2,
        DriverStatus::Unknown => 3,
    }
}

/// Extrait la région d'une description MAME au format `"Nom (Région, ...)"`.
pub fn extract_region(description: &str) -> Option<String> {
    let start = description.find('(')?;
    let end = description[start..].find(')')?;
    let inside = &description[start + 1..start + end];
    let first_token = inside.split(',').next()?.trim();

    if first_token.is_empty() {
        None
    } else {
        Some(first_token.to_string())
    }
}

/// Score de révision : plus la valeur est élevée, plus la révision est
/// récente (`Rev B` > `Rev A`, `v1.2` > `v1.1`). Retourne un score plus
/// petit (donc "moins bon") lorsqu'aucune révision n'est mentionnée.
fn revision_penalty(description: &str) -> i64 {
    let lower = description.to_ascii_lowercase();

    if let Some(pos) = lower.find("rev ") {
        let rest = &lower[pos + 4..];
        if let Some(first) = rest.chars().next() {
            if first.is_ascii_alphabetic() {
                return (first as u8 - b'a' + 1) as i64 * 1000;
            }
            if first.is_ascii_digit() {
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = digits.parse::<i64>() {
                    return n * 1000;
                }
            }
        }
    }

    if let Some(pos) = lower.find('v') {
        let rest = &lower[pos + 1..];
        if rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let version_str: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let mut parts = version_str.split('.');
            let major: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let minor: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            return major * 100 + minor;
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, description: &str, clone_of: Option<&str>) -> RomEntry {
        RomEntry {
            name: name.to_string(),
            description: description.to_string(),
            year: "1990".to_string(),
            manufacturer: "Test".to_string(),
            clone_of: clone_of.map(String::from),
            rom_of: clone_of.map(String::from),
            is_bios: false,
            is_device: false,
            is_mechanical: false,
            runnable: true,
            driver_status: DriverStatus::Good,
            category: None,
            languages: Vec::new(),
            roms: Vec::new(),
        }
    }

    #[test]
    fn extracts_region_from_description() {
        assert_eq!(
            extract_region("Pac-Man (USA)").as_deref(),
            Some("USA")
        );
        assert_eq!(
            extract_region("Pac-Man (Japan, Asia)").as_deref(),
            Some("Japan")
        );
        assert_eq!(extract_region("Pac-Man"), None);
    }

    #[test]
    fn groups_parent_and_clones_and_picks_highest_priority_region() {
        let mut entries = HashMap::new();
        entries.insert("puckman".into(), entry("puckman", "PuckMan (Japan)", None));
        entries.insert(
            "pacman".into(),
            entry("pacman", "Pac-Man (USA)", Some("puckman")),
        );
        entries.insert(
            "pacmanw".into(),
            entry("pacmanw", "Pac-Man (World)", Some("puckman")),
        );

        let plan = build_dedup_plan(&entries, &RegionPriority::default_profile());
        assert_eq!(plan.groups.len(), 1);

        let group = &plan.groups[0];
        assert_eq!(group.parent_name, "puckman");
        assert_eq!(group.keep, "pacmanw");
        assert_eq!(group.remove.len(), 2);
        assert!(group.remove.contains(&"pacman".to_string()));
        assert!(group.remove.contains(&"puckman".to_string()));
    }

    #[test]
    fn falls_back_to_driver_status_when_region_is_tied() {
        let mut entries = HashMap::new();
        let mut parent = entry("gamea", "Game A (USA)", None);
        parent.driver_status = DriverStatus::Imperfect;
        entries.insert("gamea".into(), parent);

        let mut clone = entry("gameb", "Game A (USA)", Some("gamea"));
        clone.driver_status = DriverStatus::Good;
        entries.insert("gameb".into(), clone);

        let plan = build_dedup_plan(&entries, &RegionPriority::default_profile());
        let group = &plan.groups[0];
        assert_eq!(group.keep, "gameb");
    }

    #[test]
    fn falls_back_to_most_recent_revision_when_region_and_status_are_tied() {
        let mut entries = HashMap::new();
        entries.insert(
            "gamea".into(),
            entry("gamea", "Game A (USA, Rev A)", None),
        );
        entries.insert(
            "gameb".into(),
            entry("gameb", "Game A (USA, Rev B)", Some("gamea")),
        );

        let plan = build_dedup_plan(&entries, &RegionPriority::default_profile());
        let group = &plan.groups[0];
        assert_eq!(group.keep, "gameb");
    }

    #[test]
    fn resolves_multi_level_clone_chains_to_the_root_parent() {
        let mut entries = HashMap::new();
        entries.insert("root".into(), entry("root", "Game (World)", None));
        entries.insert(
            "midclone".into(),
            entry("midclone", "Game (USA)", Some("root")),
        );
        entries.insert(
            "leafclone".into(),
            entry("leafclone", "Game (Japan)", Some("midclone")),
        );

        let plan = build_dedup_plan(&entries, &RegionPriority::default_profile());
        assert_eq!(plan.groups.len(), 1);
        assert_eq!(plan.groups[0].parent_name, "root");
        assert_eq!(plan.groups[0].members.len(), 3);
    }

    #[test]
    fn excludes_devices_from_grouping() {
        let mut entries = HashMap::new();
        let mut device = entry("somedevice", "Some Device", None);
        device.is_device = true;
        entries.insert("somedevice".into(), device);
        entries.insert("gamea".into(), entry("gamea", "Game A (USA)", None));

        let plan = build_dedup_plan(&entries, &RegionPriority::default_profile());
        assert_eq!(plan.groups.len(), 1);
        assert_eq!(plan.groups[0].parent_name, "gamea");
    }

    #[test]
    fn dry_run_plan_exposes_keep_and_remove_lists_without_deleting_anything() {
        let mut entries = HashMap::new();
        entries.insert("puckman".into(), entry("puckman", "PuckMan (Japan)", None));
        entries.insert(
            "pacman".into(),
            entry("pacman", "Pac-Man (World)", Some("puckman")),
        );

        let plan = build_dedup_plan(&entries, &RegionPriority::default_profile());
        assert_eq!(plan.roms_to_keep(), vec!["pacman".to_string()]);
        assert_eq!(plan.roms_to_remove(), vec!["puckman".to_string()]);
    }
}
