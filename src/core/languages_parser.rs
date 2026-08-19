use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum LanguagesError {
    Io(std::io::Error),
}

impl fmt::Display for LanguagesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguagesError::Io(err) => {
                write!(f, "erreur de lecture du fichier languages.ini : {err}")
            }
        }
    }
}

impl std::error::Error for LanguagesError {}

impl From<std::io::Error> for LanguagesError {
    fn from(err: std::io::Error) -> Self {
        LanguagesError::Io(err)
    }
}

pub fn parse_languages_file(path: &Path) -> Result<HashMap<String, Vec<String>>, LanguagesError> {
    let content = fs::read_to_string(path)?;
    Ok(parse_languages_str(&content))
}

/// `languages.ini` ne suit pas la syntaxe classique `clé=valeur` : chaque
/// section `[Langue]` liste simplement les noms de ROMs qui lui
/// appartiennent, un par ligne. Le résultat associe chaque nom de ROM à
/// la (ou les) langue(s) sous lesquelles il apparaît.
pub fn parse_languages_str(content: &str) -> HashMap<String, Vec<String>> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_language: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_language = Some(line[1..line.len() - 1].trim().to_string());
            continue;
        }

        if let Some(language) = &current_language {
            result
                .entry(line.to_string())
                .or_default()
                .push(language.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_LANGUAGES: &str = "\
[English]
pacman
dkong

[Japanese]
puckman
; commentaire ignoré
dkong
";

    #[test]
    fn associates_games_to_their_language_sections() {
        let languages = parse_languages_str(SAMPLE_LANGUAGES);
        assert_eq!(languages.get("pacman"), Some(&vec!["English".to_string()]));
        assert_eq!(
            languages.get("puckman"),
            Some(&vec!["Japanese".to_string()])
        );
    }

    #[test]
    fn a_game_can_belong_to_multiple_languages() {
        let languages = parse_languages_str(SAMPLE_LANGUAGES);
        assert_eq!(
            languages.get("dkong"),
            Some(&vec!["English".to_string(), "Japanese".to_string()])
        );
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let languages = parse_languages_str(SAMPLE_LANGUAGES);
        assert!(!languages.contains_key("; commentaire ignoré"));
    }
}
