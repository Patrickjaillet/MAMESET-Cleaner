use std::fmt;

use quick_xml::events::Event;
use quick_xml::Reader;

/// A `<rom>` entry inside a Logiqx-style DAT (`<datafile><game><rom .../></game></datafile>`),
/// the schema shared by No-Intro, Redump and TOSEC reference databases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogiqxRom {
    pub name: String,
    pub size: u64,
    pub crc32: Option<u32>,
}

/// A `<game>` entry inside a Logiqx-style DAT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogiqxGame {
    pub name: String,
    pub description: String,
    pub clone_of: Option<String>,
    pub roms: Vec<LogiqxRom>,
}

#[derive(Debug)]
pub enum LogiqxDatError {
    Xml(quick_xml::Error),
    NoGamesFound,
    Panicked,
}

impl fmt::Display for LogiqxDatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogiqxDatError::Xml(err) => write!(f, "XML parsing error in the DAT file: {err}"),
            LogiqxDatError::NoGamesFound => {
                write!(f, "the DAT file is readable but contains no recognized game")
            }
            LogiqxDatError::Panicked => {
                write!(f, "an internal error (panic) occurred while parsing the DAT file")
            }
        }
    }
}

impl std::error::Error for LogiqxDatError {}

impl From<quick_xml::Error> for LogiqxDatError {
    fn from(err: quick_xml::Error) -> Self {
        LogiqxDatError::Xml(err)
    }
}

/// Same as [`parse_logiqx_dat`], but additionally catches a panic raised
/// while parsing and turns it into a plain error message instead of letting
/// it propagate.
///
/// This exists because every console plugin in this workspace is a dynamic
/// library exposing `extern "C"` functions: a panic that unwinds across an
/// `extern "C"` FFI boundary without being caught first is undefined
/// behavior, and in practice aborts the whole host process rather than
/// being observable as an error (`thread caused non-unwinding panic.
/// aborting.`) — confirmed directly while building the panic-isolation test
/// for v1.8.0. Catching the panic here, entirely on the plugin's own side
/// of the boundary and before any FFI-safe value is returned, is what
/// actually keeps a misbehaving reference database from crashing the host;
/// a `catch_unwind` added on the host side after the call cannot help,
/// since the abort already happens while unwinding through the plugin's own
/// `extern "C"` frame. Every plugin shipped in this workspace calls this
/// function rather than [`parse_logiqx_dat`] directly for that reason.
pub fn parse_logiqx_dat_panic_safe(xml: &str) -> Result<Vec<LogiqxGame>, String> {
    std::panic::catch_unwind(|| parse_logiqx_dat(xml))
        .unwrap_or(Err(LogiqxDatError::Panicked))
        .map_err(|err| err.to_string())
}

/// Parses a Logiqx-style DAT (`<datafile><game name=".." cloneof="..">
/// <description>..</description><rom name=".." size=".." crc=".."/></game>
/// </datafile>`), the format used by No-Intro, Redump and TOSEC reference
/// databases. Unlike MAME's `-listxml` format, this schema carries no
/// `year`/`manufacturer`/`driver status` fields, so those are simply absent
/// from the result.
pub fn parse_logiqx_dat(xml: &str) -> Result<Vec<LogiqxGame>, LogiqxDatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut games: Vec<LogiqxGame> = Vec::new();
    let mut current: Option<LogiqxGame> = None;
    let mut in_description = false;

    loop {
        match reader.read_event()? {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => {
                let name = e.name();
                let local = name.local_name();
                let local = local.as_ref();

                if local == b"game" || local == b"machine" {
                    current = Some(build_game_from_attributes(&e)?);
                    in_description = false;
                } else if local == b"description" {
                    in_description = true;
                } else if local == b"rom" {
                    if let Some(game) = current.as_mut() {
                        game.roms.push(parse_rom_attributes(&e)?);
                    }
                }
            }
            Event::Text(text) => {
                if in_description {
                    if let Some(game) = current.as_mut() {
                        game.description = text.unescape()?.trim().to_string();
                    }
                }
            }
            Event::End(e) => {
                let name = e.name();
                let local = name.local_name();
                let local = local.as_ref();

                if local == b"game" || local == b"machine" {
                    if let Some(game) = current.take() {
                        games.push(game);
                    }
                    in_description = false;
                } else if local == b"description" {
                    in_description = false;
                }
            }
            _ => {}
        }
    }

    if games.is_empty() {
        return Err(LogiqxDatError::NoGamesFound);
    }

    Ok(games)
}

fn attr_value(e: &quick_xml::events::BytesStart, key: &str) -> Result<Option<String>, LogiqxDatError> {
    for attr in e.attributes() {
        let attr = attr.map_err(quick_xml::Error::InvalidAttr)?;
        if attr.key.local_name().as_ref() == key.as_bytes() {
            return Ok(Some(attr.unescape_value()?.into_owned()));
        }
    }
    Ok(None)
}

fn build_game_from_attributes(e: &quick_xml::events::BytesStart) -> Result<LogiqxGame, LogiqxDatError> {
    let name = attr_value(e, "name")?.unwrap_or_default();
    let clone_of = attr_value(e, "cloneof")?;

    Ok(LogiqxGame {
        name,
        description: String::new(),
        clone_of,
        roms: Vec::new(),
    })
}

fn parse_rom_attributes(e: &quick_xml::events::BytesStart) -> Result<LogiqxRom, LogiqxDatError> {
    let name = attr_value(e, "name")?.unwrap_or_default();
    let size = attr_value(e, "size")?
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let crc32 = attr_value(e, "crc")?.and_then(|v| u32::from_str_radix(v.trim(), 16).ok());

    Ok(LogiqxRom { name, size, crc32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_NO_INTRO_DAT: &str = r#"<?xml version="1.0"?>
<!DOCTYPE datafile PUBLIC "-//Logiqx//DTD ROM Management Datafile//EN" "http://www.logiqx.com/Dats/datafile.dtd">
<datafile>
    <header>
        <name>Nintendo - Nintendo Entertainment System</name>
        <version>20240101</version>
    </header>
    <game name="Super Mario Bros. (World)">
        <description>Super Mario Bros. (World)</description>
        <rom name="Super Mario Bros. (World).nes" size="40976" crc="d445f698"/>
    </game>
    <game name="Super Mario Bros. (World) (Rev 1)" cloneof="Super Mario Bros. (World)">
        <description>Super Mario Bros. (World) (Rev 1)</description>
        <rom name="Super Mario Bros. (World) (Rev 1).nes" size="40976" crc="abc12345"/>
    </game>
</datafile>"#;

    #[test]
    fn parses_games_and_their_roms() {
        let games = parse_logiqx_dat(SAMPLE_NO_INTRO_DAT).unwrap();
        assert_eq!(games.len(), 2);

        let parent = &games[0];
        assert_eq!(parent.name, "Super Mario Bros. (World)");
        assert_eq!(parent.description, "Super Mario Bros. (World)");
        assert_eq!(parent.clone_of, None);
        assert_eq!(parent.roms.len(), 1);
        assert_eq!(parent.roms[0].crc32, Some(0xd445_f698));
        assert_eq!(parent.roms[0].size, 40976);
    }

    #[test]
    fn parses_clone_relationship() {
        let games = parse_logiqx_dat(SAMPLE_NO_INTRO_DAT).unwrap();
        let clone = &games[1];
        assert_eq!(
            clone.clone_of.as_deref(),
            Some("Super Mario Bros. (World)")
        );
    }

    #[test]
    fn invalid_xml_is_reported_without_panicking() {
        let result = parse_logiqx_dat("<datafile><game name=\"broken\"></datafile>");
        assert!(matches!(result, Err(LogiqxDatError::Xml(_))));
    }

    #[test]
    fn empty_file_is_reported_clearly() {
        let result = parse_logiqx_dat("");
        assert!(matches!(result, Err(LogiqxDatError::NoGamesFound)));
    }

    #[test]
    fn valid_xml_without_any_game_is_reported_clearly() {
        let result = parse_logiqx_dat(r#"<?xml version="1.0"?><datafile><header/></datafile>"#);
        assert!(matches!(result, Err(LogiqxDatError::NoGamesFound)));
    }

    #[test]
    fn panic_safe_variant_behaves_like_the_regular_parser_on_valid_input() {
        let games = parse_logiqx_dat_panic_safe(SAMPLE_NO_INTRO_DAT).unwrap();
        assert_eq!(games.len(), 2);
    }

    #[test]
    fn panic_safe_variant_reports_errors_without_panicking() {
        let result = parse_logiqx_dat_panic_safe("not xml at all");
        assert!(result.is_err());
    }
}
