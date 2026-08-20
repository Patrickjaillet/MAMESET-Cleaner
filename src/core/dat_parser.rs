use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::Instant;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::models::rom_entry::{DriverStatus, RomEntry, RomFile};

#[derive(Debug)]
pub enum DatError {
    Io(std::io::Error),
    Xml(quick_xml::Error),
    Http(String),
    NoMachinesFound,
}

impl fmt::Display for DatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatError::Io(err) => write!(f, "erreur de lecture du fichier DAT : {err}"),
            DatError::Xml(err) => write!(f, "erreur de parsing XML du DAT : {err}"),
            DatError::Http(msg) => write!(f, "erreur de téléchargement du DAT : {msg}"),
            DatError::NoMachinesFound => write!(
                f,
                "le fichier DAT est lisible mais ne contient aucune machine reconnue"
            ),
        }
    }
}

impl std::error::Error for DatError {}

impl From<std::io::Error> for DatError {
    fn from(err: std::io::Error) -> Self {
        DatError::Io(err)
    }
}

impl From<quick_xml::Error> for DatError {
    fn from(err: quick_xml::Error) -> Self {
        DatError::Xml(err)
    }
}

/// Un fichier `-listxml` MAME utilise `<mame><machine>...`, tandis que
/// certains DAT tiers au format ClrMamePro/officiel utilisent
/// `<datafile><game>...`. Les deux partagent la même structure de champs.
const MACHINE_TAGS: [&[u8]; 2] = [b"machine", b"game"];
const ROOT_TAGS: [&[u8]; 2] = [b"mame", b"datafile"];

// Note: intentionally reads the whole file into memory via
// `fs::read_to_string` and parses it with `Reader::from_str`, rather than
// streaming from a `BufReader`. Measured both against the real 56 MB MAME
// `-listxml` (37 123 machines): `from_str` (zero-copy borrowing directly
// from the in-memory string) took ~1.6s, while a `Reader::from_reader`
// streaming variant using `read_event_into` took ~2.2s — quick_xml's
// `Read`-based API has to copy each chunk into a scratch buffer per event,
// which is slower here than paying for one upfront read + UTF-8 validation.
// See ROADMAP5.md v4.1.0 notes.
pub fn parse_dat_file(path: &Path) -> Result<HashMap<String, RomEntry>, DatError> {
    let started = Instant::now();
    let content = fs::read_to_string(path)?;
    let entries = parse_dat_str(&content)?;
    tracing::info!(
        duration_ms = started.elapsed().as_millis() as u64,
        machine_count = entries.len(),
        path = %path.display(),
        "DAT file parsed"
    );
    Ok(entries)
}

pub fn parse_dat_str(xml: &str) -> Result<HashMap<String, RomEntry>, DatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut entries: HashMap<String, RomEntry> = HashMap::new();
    let mut current: Option<RomEntry> = None;
    let mut current_field: Option<CurrentField> = None;

    loop {
        match reader.read_event()? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => {
                let name = e.name();
                let local = name.local_name();
                let local = local.as_ref();

                if MACHINE_TAGS.contains(&local) {
                    current = Some(build_machine_from_attributes(&e)?);
                    current_field = None;
                } else if local == b"description" {
                    current_field = Some(CurrentField::Description);
                } else if local == b"year" {
                    current_field = Some(CurrentField::Year);
                } else if local == b"manufacturer" {
                    current_field = Some(CurrentField::Manufacturer);
                } else if local == b"rom" {
                    if let Some(entry) = current.as_mut() {
                        entry.roms.push(parse_rom_attributes(&e)?);
                    }
                } else if local == b"driver" {
                    if let Some(entry) = current.as_mut() {
                        entry.driver_status = parse_driver_status(&e)?;
                    }
                }
            }
            Event::Text(text) => {
                if let (Some(field), Some(entry)) = (current_field, current.as_mut()) {
                    let value = text.unescape()?.trim().to_string();
                    match field {
                        CurrentField::Description => entry.description = value,
                        CurrentField::Year => entry.year = value,
                        CurrentField::Manufacturer => entry.manufacturer = value,
                    }
                }
            }
            Event::End(e) => {
                let local = e.name();
                let local = local.local_name();
                let local = local.as_ref();

                if MACHINE_TAGS.contains(&local) {
                    if let Some(entry) = current.take() {
                        entries.insert(entry.name.clone(), entry);
                    }
                    current_field = None;
                } else if local == b"description" || local == b"year" || local == b"manufacturer"
                {
                    current_field = None;
                }
            }
            _ => {}
        }
    }

    if entries.is_empty() {
        return Err(DatError::NoMachinesFound);
    }

    Ok(entries)
}

#[derive(Clone, Copy)]
enum CurrentField {
    Description,
    Year,
    Manufacturer,
}

fn attr_value(e: &quick_xml::events::BytesStart, key: &str) -> Result<Option<String>, DatError> {
    for attr in e.attributes() {
        let attr = attr.map_err(quick_xml::Error::InvalidAttr)?;
        if attr.key.local_name().as_ref() == key.as_bytes() {
            return Ok(Some(attr.unescape_value()?.into_owned()));
        }
    }
    Ok(None)
}

fn build_machine_from_attributes(
    e: &quick_xml::events::BytesStart,
) -> Result<RomEntry, DatError> {
    let name = attr_value(e, "name")?.unwrap_or_default();
    let clone_of = attr_value(e, "cloneof")?;
    let rom_of = attr_value(e, "romof")?;
    let is_bios = attr_value(e, "isbios")?.as_deref() == Some("yes");
    let is_device = attr_value(e, "isdevice")?.as_deref() == Some("yes");
    let is_mechanical = attr_value(e, "ismechanical")?.as_deref() == Some("yes");
    let runnable = attr_value(e, "runnable")?.as_deref() != Some("no");

    Ok(RomEntry {
        name,
        description: String::new(),
        year: String::new(),
        manufacturer: String::new(),
        clone_of,
        rom_of,
        is_bios,
        is_device,
        is_mechanical,
        runnable,
        driver_status: DriverStatus::Unknown,
        category: None,
        languages: Vec::new(),
        roms: Vec::new(),
    })
}

fn parse_rom_attributes(e: &quick_xml::events::BytesStart) -> Result<RomFile, DatError> {
    let name = attr_value(e, "name")?.unwrap_or_default();
    let size = attr_value(e, "size")?
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let crc32 = attr_value(e, "crc")?.and_then(|v| u32::from_str_radix(v.trim(), 16).ok());
    let sha1 = attr_value(e, "sha1")?;

    Ok(RomFile {
        name,
        size,
        crc32,
        sha1,
    })
}

fn parse_driver_status(e: &quick_xml::events::BytesStart) -> Result<DriverStatus, DatError> {
    let status = attr_value(e, "status")?.unwrap_or_default();
    Ok(DriverStatus::parse_status(&status))
}

/// Lit l'attribut de version (`build` pour `-listxml`, `version` pour un
/// DAT ClrMamePro) présent sur l'élément racine, utilisé pour détecter
/// si une nouvelle version du DAT officiel est disponible.
pub fn extract_dat_build_version(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event().ok()? {
            Event::Eof => return None,
            Event::Start(e) | Event::Empty(e) => {
                let name = e.name();
                let local = name.local_name();
                let local = local.as_ref();
                if ROOT_TAGS.contains(&local) {
                    if let Ok(Some(build)) = attr_value(&e, "build") {
                        return Some(build);
                    }
                    if let Ok(Some(version)) = attr_value(&e, "version") {
                        return Some(version);
                    }
                    return None;
                }
            }
            _ => {}
        }
    }
}

pub fn needs_update(current_version: Option<&str>, remote_version: &str) -> bool {
    match current_version {
        Some(current) => current != remote_version,
        None => true,
    }
}

/// Télécharge le fichier DAT depuis `url` (configurée par l'utilisateur
/// dans les Paramètres, cf. v0.6.0) et l'enregistre sur `destination`.
pub fn download_dat_file(url: &str, destination: &Path) -> Result<(), DatError> {
    let response = ureq::get(url)
        .call()
        .map_err(|err| DatError::Http(err.to_string()))?;

    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination)?;
    std::io::copy(&mut reader, &mut file)?;

    Ok(())
}

/// Fusionne les métadonnées de `catver.ini` et `languages.ini` dans les
/// `RomEntry` issus du DAT, produisant le modèle de données unifié.
pub fn merge_metadata(
    entries: &mut HashMap<String, RomEntry>,
    categories: &HashMap<String, String>,
    languages: &HashMap<String, Vec<String>>,
) {
    for (name, entry) in entries.iter_mut() {
        entry.category = categories.get(name).cloned();
        entry.languages = languages.get(name).cloned().unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DAT: &str = r#"<?xml version="1.0"?>
<mame build="0.260 (mame0260)">
    <machine name="puckman" sourcefile="pacman.cpp">
        <description>PuckMan (Japan set 1)</description>
        <year>1980</year>
        <manufacturer>Namco</manufacturer>
        <rom name="pm1_prg1.6e" size="4096" crc="0c944964" sha1="aaaa"/>
        <driver status="good" emulation="good" cocktail="good" savestate="supported"/>
    </machine>
    <machine name="pacman" sourcefile="pacman.cpp" cloneof="puckman" romof="puckman">
        <description>Pac-Man (Midway)</description>
        <year>1980</year>
        <manufacturer>Midway</manufacturer>
        <rom name="pacman.6e" size="4096" crc="c1e6ab10" sha1="bbbb"/>
        <driver status="imperfect" emulation="good" cocktail="good" savestate="supported"/>
    </machine>
    <machine name="pacmanbios" isbios="yes" isdevice="no" ismechanical="no" runnable="no">
        <description>Pac-Man BIOS</description>
        <year>1980</year>
        <manufacturer>Namco</manufacturer>
    </machine>
</mame>"#;

    #[test]
    fn parses_parent_and_clone_machines() {
        let entries = parse_dat_str(SAMPLE_DAT).unwrap();
        assert_eq!(entries.len(), 3);

        let parent = &entries["puckman"];
        assert_eq!(parent.description, "PuckMan (Japan set 1)");
        assert_eq!(parent.year, "1980");
        assert_eq!(parent.manufacturer, "Namco");
        assert!(parent.is_parent());
        assert_eq!(parent.driver_status, DriverStatus::Good);
        assert_eq!(parent.roms.len(), 1);
        assert_eq!(parent.roms[0].crc32, Some(0x0c944964));

        let clone = &entries["pacman"];
        assert!(clone.is_clone());
        assert_eq!(clone.clone_of.as_deref(), Some("puckman"));
        assert_eq!(clone.driver_status, DriverStatus::Imperfect);
    }

    #[test]
    fn parses_bios_flags_and_runnable() {
        let entries = parse_dat_str(SAMPLE_DAT).unwrap();
        let bios = &entries["pacmanbios"];
        assert!(bios.is_bios);
        assert!(!bios.is_device);
        assert!(!bios.is_mechanical);
        assert!(!bios.runnable);
    }

    #[test]
    fn extracts_build_version_from_root_element() {
        let version = extract_dat_build_version(SAMPLE_DAT);
        assert_eq!(version.as_deref(), Some("0.260 (mame0260)"));
    }

    #[test]
    fn detects_when_update_is_needed() {
        assert!(needs_update(None, "0.260"));
        assert!(needs_update(Some("0.259"), "0.260"));
        assert!(!needs_update(Some("0.260"), "0.260"));
    }

    #[test]
    fn merges_category_and_language_metadata_into_entries() {
        let mut entries = parse_dat_str(SAMPLE_DAT).unwrap();

        let mut categories = HashMap::new();
        categories.insert("puckman".to_string(), "Maze".to_string());

        let mut languages = HashMap::new();
        languages.insert("puckman".to_string(), vec!["Japanese".to_string()]);

        merge_metadata(&mut entries, &categories, &languages);

        let parent = &entries["puckman"];
        assert_eq!(parent.category.as_deref(), Some("Maze"));
        assert_eq!(parent.languages, vec!["Japanese".to_string()]);

        let clone = &entries["pacman"];
        assert_eq!(clone.category, None);
        assert!(clone.languages.is_empty());
    }

    #[test]
    fn invalid_xml_is_reported_without_panicking() {
        let result = parse_dat_str("<mame><machine name=\"broken\"></mame>");
        assert!(matches!(result, Err(DatError::Xml(_))));
    }

    #[test]
    fn valid_xml_without_any_machine_is_reported_clearly() {
        let result = parse_dat_str(r#"<?xml version="1.0"?><mame build="0.260"></mame>"#);
        assert!(matches!(result, Err(DatError::NoMachinesFound)));
    }

    #[test]
    fn empty_file_is_reported_clearly() {
        let result = parse_dat_str("");
        assert!(matches!(result, Err(DatError::NoMachinesFound)));
    }
}
