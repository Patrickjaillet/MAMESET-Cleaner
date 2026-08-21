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

/// Optional tie-breaker used only when the region priority (and the
/// unofficial-release check) leaves two or more candidates still tied. An
/// empty order means "no preference" — every candidate ties, so this never
/// changes the outcome (today's exact behavior when unconfigured).
#[derive(Debug, Clone, Default)]
pub struct LanguagePriority {
    order: Vec<String>,
}

impl LanguagePriority {
    pub fn new(order: Vec<String>) -> Self {
        Self { order }
    }

    pub fn none() -> Self {
        Self::default()
    }

    fn rank(&self, languages: &[String]) -> usize {
        if self.order.is_empty() {
            return 0;
        }
        languages
            .iter()
            .filter_map(|language| {
                self.order
                    .iter()
                    .position(|candidate| candidate.eq_ignore_ascii_case(language))
            })
            .min()
            .unwrap_or(self.order.len())
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

/// How a group of duplicate candidates was identified. `Name` mirrors
/// MAME-style `cloneof` chains (unchanged since v0.1.0): the key is the
/// ultimate parent's own machine name. `Title` is the v5.2.0 fallback used
/// when a DAT never declares `cloneof` at all (No-Intro/TOSEC/Redump) — the
/// key is a normalized title plus, critically, a disc/part identity that
/// must never be stripped, so a 2-disc release's two discs are never
/// treated as duplicates of each other.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum GroupKey {
    Name(String),
    Title(String, Option<String>),
}

fn group_key_display_name(key: &GroupKey) -> String {
    match key {
        GroupKey::Name(name) => name.clone(),
        GroupKey::Title(title, Some(disc)) => format!("{title} ({disc})"),
        GroupKey::Title(title, None) => title.clone(),
    }
}

pub struct DedupOptions<'a> {
    pub region_priority: &'a RegionPriority,
    pub language_priority: &'a LanguagePriority,
    /// When true, prototypes/betas/demos/unlicensed releases are treated as
    /// equally valid keep-candidates as an official release. When false
    /// (the default), an official release always beats them regardless of
    /// region/language priority.
    pub treat_unofficial_as_official: bool,
}

impl<'a> DedupOptions<'a> {
    pub fn new(region_priority: &'a RegionPriority, language_priority: &'a LanguagePriority) -> Self {
        Self {
            region_priority,
            language_priority,
            treat_unofficial_as_official: false,
        }
    }
}

pub fn build_dedup_plan(entries: &HashMap<String, RomEntry>, options: &DedupOptions) -> DedupPlan {
    // Names that ARE referenced as a `cloneof` target somewhere in the
    // dataset — i.e. real MAME-style parents that actually have clones.
    // Anything else with no `cloneof` of its own is either a standalone
    // MAME game (grouping by its own name is harmless either way) or, far
    // more commonly across the 87 supported systems, a No-Intro/TOSEC/
    // Redump entry that never declares `cloneof` at all and needs the
    // title-based fallback to group with its regional siblings.
    let clone_targets: HashSet<&str> = entries
        .values()
        .filter_map(|entry| entry.clone_of.as_deref())
        .collect();

    let mut groups_map: HashMap<GroupKey, Vec<String>> = HashMap::new();

    for (name, entry) in entries {
        if entry.is_device {
            continue;
        }
        let key = group_key_for(entries, &clone_targets, name, entry);
        groups_map.entry(key).or_default().push(name.clone());
    }

    let mut groups: Vec<DedupGroup> = groups_map
        .into_iter()
        .map(|(key, mut members)| {
            members.sort();
            let keep = select_best(entries, &members, options);
            let remove = members
                .iter()
                .filter(|member| **member != keep)
                .cloned()
                .collect();
            DedupGroup {
                parent_name: group_key_display_name(&key),
                members,
                keep,
                remove,
            }
        })
        .collect();

    groups.sort_by(|a, b| a.parent_name.cmp(&b.parent_name));

    DedupPlan { groups }
}

fn group_key_for(
    entries: &HashMap<String, RomEntry>,
    clone_targets: &HashSet<&str>,
    name: &str,
    entry: &RomEntry,
) -> GroupKey {
    if entry.clone_of.is_some() {
        return GroupKey::Name(resolve_root_parent(entries, name));
    }
    if clone_targets.contains(name) {
        return GroupKey::Name(name.to_string());
    }
    let (title, disc_identity) = normalize_title(&entry.description);
    GroupKey::Title(title, disc_identity)
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

fn select_best(entries: &HashMap<String, RomEntry>, members: &[String], options: &DedupOptions) -> String {
    members
        .iter()
        .min_by(|a, b| compare_candidates(&entries[*a], &entries[*b], options))
        .cloned()
        .expect("un groupe de doublons ne peut pas être vide")
}

fn compare_candidates(a: &RomEntry, b: &RomEntry, options: &DedupOptions) -> Ordering {
    if !options.treat_unofficial_as_official {
        let unofficial_a = is_unofficial_release(&a.description);
        let unofficial_b = is_unofficial_release(&b.description);
        let by_official = unofficial_a.cmp(&unofficial_b);
        if by_official != Ordering::Equal {
            return by_official;
        }
    }

    let region_a = extract_region(&a.description);
    let region_b = extract_region(&b.description);

    options
        .region_priority
        .rank(region_a.as_deref())
        .cmp(&options.region_priority.rank(region_b.as_deref()))
        .then_with(|| {
            options
                .language_priority
                .rank(&a.languages)
                .cmp(&options.language_priority.rank(&b.languages))
        })
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

/// Splits a description like `"Super Mario Bros. (World) (Rev 1)"` into a
/// normalized base title (every bracketed tag group stripped) and, when one
/// of those tag groups is a disc/part/side/tape identity (`"Disc 1"`,
/// `"Tape 2 of 3"`, `"Side A"`), that identity kept separately. Used as the
/// 1G1R grouping key for DATs that don't declare `cloneof` at all (see
/// `group_key_for`) — critically, the disc identity is never dropped, so a
/// multi-disc release's discs are never merged into one "duplicate" group.
fn normalize_title(description: &str) -> (String, Option<String>) {
    let title_end = description.find('(').unwrap_or(description.len());
    let title = description[..title_end].trim().to_string();

    let mut disc_identity = None;
    let mut rest = description;
    while let Some(start) = rest.find('(') {
        let Some(end_rel) = rest[start..].find(')') else {
            break;
        };
        let inside = rest[start + 1..start + end_rel].trim();
        if is_disc_identity_tag(inside) {
            disc_identity = Some(inside.to_string());
        }
        rest = &rest[start + end_rel + 1..];
    }

    (title, disc_identity)
}

fn is_disc_identity_tag(tag: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    ["disc ", "disk ", "tape ", "side ", "part "]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

/// True when a description's bracketed tags mark it as a non-final release
/// (prototype, beta, demo, sample, unlicensed, pirate/aftermarket copy).
/// Common No-Intro/TOSEC naming convention.
fn is_unofficial_release(description: &str) -> bool {
    let mut rest = description;
    while let Some(start) = rest.find('(') {
        let Some(end_rel) = rest[start..].find(')') else {
            break;
        };
        let inside = rest[start + 1..start + end_rel].trim().to_ascii_lowercase();
        for tag in inside.split(',') {
            let tag = tag.trim();
            if matches!(
                tag,
                "proto" | "prototype" | "beta" | "demo" | "sample" | "unl" | "unlicensed"
                    | "pirate" | "aftermarket" | "alt"
            ) {
                return true;
            }
        }
        rest = &rest[start + end_rel + 1..];
    }
    false
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

    fn default_options() -> DedupOptions<'static> {
        // Leaked once per call for test convenience — negligible, tests
        // only, never reachable in the running application.
        let region: &'static RegionPriority = Box::leak(Box::new(RegionPriority::default_profile()));
        let language: &'static LanguagePriority = Box::leak(Box::new(LanguagePriority::none()));
        DedupOptions::new(region, language)
    }

    fn plan(entries: &HashMap<String, RomEntry>) -> DedupPlan {
        build_dedup_plan(entries, &default_options())
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

        let result = plan(&entries);
        assert_eq!(result.groups.len(), 1);

        let group = &result.groups[0];
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

        let result = plan(&entries);
        let group = &result.groups[0];
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

        let result = plan(&entries);
        let group = &result.groups[0];
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

        let result = plan(&entries);
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].parent_name, "root");
        assert_eq!(result.groups[0].members.len(), 3);
    }

    #[test]
    fn excludes_devices_from_grouping() {
        let mut entries = HashMap::new();
        let mut device = entry("somedevice", "Some Device", None);
        device.is_device = true;
        entries.insert("somedevice".into(), device);
        entries.insert("gamea".into(), entry("gamea", "Game A (USA)", None));

        let result = plan(&entries);
        assert_eq!(result.groups.len(), 1);
        // "gamea" has no cloneof relationship at all, so it falls through
        // to the v5.2.0 title-based grouping key — the display name is the
        // normalized title ("Game A"), not the raw machine name.
        assert_eq!(result.groups[0].parent_name, "Game A");
    }

    #[test]
    fn dry_run_plan_exposes_keep_and_remove_lists_without_deleting_anything() {
        let mut entries = HashMap::new();
        entries.insert("puckman".into(), entry("puckman", "PuckMan (Japan)", None));
        entries.insert(
            "pacman".into(),
            entry("pacman", "Pac-Man (World)", Some("puckman")),
        );

        let result = plan(&entries);
        assert_eq!(result.roms_to_keep(), vec!["pacman".to_string()]);
        assert_eq!(result.roms_to_remove(), vec!["puckman".to_string()]);
    }

    // ---- v5.2.0: title-based grouping for DATs without cloneof ----

    #[test]
    fn groups_no_intro_style_regional_releases_that_never_declare_cloneof() {
        let mut entries = HashMap::new();
        entries.insert(
            "smb_usa".into(),
            entry("smb_usa", "Super Mario Bros. (USA)", None),
        );
        entries.insert(
            "smb_eur".into(),
            entry("smb_eur", "Super Mario Bros. (Europe)", None),
        );
        entries.insert(
            "smb_jpn".into(),
            entry("smb_jpn", "Super Mario Bros. (Japan)", None),
        );

        let result = plan(&entries);
        assert_eq!(result.groups.len(), 1, "regional variants must group even with no cloneof data");
        let group = &result.groups[0];
        assert_eq!(group.keep, "smb_usa");
        assert_eq!(group.members.len(), 3);
    }

    #[test]
    fn never_merges_different_discs_of_the_same_multi_disc_release() {
        let mut entries = HashMap::new();
        entries.insert(
            "ff7_usa_d1".into(),
            entry("ff7_usa_d1", "Final Fantasy VII (USA) (Disc 1)", None),
        );
        entries.insert(
            "ff7_usa_d2".into(),
            entry("ff7_usa_d2", "Final Fantasy VII (USA) (Disc 2)", None),
        );
        entries.insert(
            "ff7_eur_d1".into(),
            entry("ff7_eur_d1", "Final Fantasy VII (Europe) (Disc 1)", None),
        );

        let result = plan(&entries);
        assert_eq!(result.groups.len(), 2, "disc 1 and disc 2 must never be treated as duplicates");

        let disc1 = result
            .groups
            .iter()
            .find(|g| g.members.contains(&"ff7_usa_d1".to_string()))
            .unwrap();
        assert_eq!(disc1.members.len(), 2, "the two regional Disc 1 releases must group together");
        assert!(disc1.members.contains(&"ff7_eur_d1".to_string()));
        assert_eq!(disc1.keep, "ff7_usa_d1");

        let disc2 = result
            .groups
            .iter()
            .find(|g| g.members.contains(&"ff7_usa_d2".to_string()))
            .unwrap();
        assert_eq!(disc2.members.len(), 1, "disc 2 has no regional sibling here, stays its own group");
        assert_eq!(disc2.keep, "ff7_usa_d2");
    }

    #[test]
    fn mame_clone_relationships_are_unaffected_by_the_title_fallback() {
        // A real MAME parent with a clone must keep using name-based
        // grouping, not accidentally fall into the title-based path.
        let mut entries = HashMap::new();
        entries.insert("puckman".into(), entry("puckman", "PuckMan (Japan)", None));
        entries.insert(
            "pacman".into(),
            entry("pacman", "Pac-Man (USA)", Some("puckman")),
        );
        // A standalone MAME game with a totally different title: must not
        // be pulled into the above group.
        entries.insert("dkong".into(), entry("dkong", "Donkey Kong (World)", None));

        let result = plan(&entries);
        assert_eq!(result.groups.len(), 2);
    }

    // ---- v5.1.0: configurable region and language priority ----

    #[test]
    fn a_custom_region_order_changes_which_copy_is_kept() {
        let mut entries = HashMap::new();
        entries.insert("gamea".into(), entry("gamea", "Game (USA)", None));
        entries.insert(
            "gameb".into(),
            entry("gameb", "Game (Japan)", Some("gamea")),
        );

        let region = RegionPriority::new(vec!["Japan".to_string(), "USA".to_string()]);
        let language = LanguagePriority::none();
        let options = DedupOptions::new(&region, &language);
        let result = build_dedup_plan(&entries, &options);

        assert_eq!(result.groups[0].keep, "gameb");
    }

    #[test]
    fn language_preference_breaks_a_region_tie() {
        let mut entries = HashMap::new();
        let mut english = entry("gamea", "Game (Europe)", None);
        english.languages = vec!["English".to_string()];
        entries.insert("gamea".into(), english);

        let mut french = entry("gameb", "Game (Europe)", Some("gamea"));
        french.languages = vec!["French".to_string()];
        entries.insert("gameb".into(), french);

        let region = RegionPriority::default_profile();
        let language = LanguagePriority::new(vec!["French".to_string()]);
        let options = DedupOptions::new(&region, &language);
        let result = build_dedup_plan(&entries, &options);

        assert_eq!(result.groups[0].keep, "gameb");
    }

    // ---- v5.3.0: unofficial-release awareness ----

    #[test]
    fn an_official_release_beats_a_proto_even_when_region_priority_favors_the_proto() {
        let mut entries = HashMap::new();
        // The proto is tagged World, the highest region priority — it
        // would win on region alone, but must still lose to the official
        // USA release by default.
        entries.insert(
            "gamea_proto".into(),
            entry("gamea_proto", "Game (World) (Proto)", None),
        );
        entries.insert(
            "gamea_usa".into(),
            entry("gamea_usa", "Game (USA)", Some("gamea_proto")),
        );

        let result = plan(&entries);
        assert_eq!(result.groups[0].keep, "gamea_usa");
    }

    #[test]
    fn the_opt_out_toggle_lets_a_proto_win_on_region_priority_again() {
        let mut entries = HashMap::new();
        entries.insert(
            "gamea_proto".into(),
            entry("gamea_proto", "Game (World) (Proto)", None),
        );
        entries.insert(
            "gamea_usa".into(),
            entry("gamea_usa", "Game (USA)", Some("gamea_proto")),
        );

        let region = RegionPriority::default_profile();
        let language = LanguagePriority::none();
        let options = DedupOptions {
            region_priority: &region,
            language_priority: &language,
            treat_unofficial_as_official: true,
        };
        let result = build_dedup_plan(&entries, &options);

        assert_eq!(result.groups[0].keep, "gamea_proto");
    }

    #[test]
    fn recognizes_common_unofficial_tags() {
        for tag in ["Proto", "Beta", "Demo", "Sample", "Unl", "Pirate", "Aftermarket", "Alt"] {
            assert!(
                is_unofficial_release(&format!("Game (USA) ({tag})")),
                "expected {tag} to be recognized as unofficial"
            );
        }
        assert!(!is_unofficial_release("Game (USA)"));
        assert!(!is_unofficial_release("Game (USA) (Rev 1)"));
    }

    #[test]
    fn invalid_xml_style_description_without_parentheses_is_never_unofficial() {
        assert!(!is_unofficial_release("Just A Title"));
    }
}
