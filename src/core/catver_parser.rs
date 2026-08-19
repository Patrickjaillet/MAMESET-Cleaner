use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

use configparser::ini::Ini;

const CATEGORY_SECTION: &str = "Category";

#[derive(Debug)]
pub enum CatverError {
    Io(std::io::Error),
    Parse(String),
    MissingCategorySection,
}

impl fmt::Display for CatverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatverError::Io(err) => write!(f, "erreur de lecture du fichier catver.ini : {err}"),
            CatverError::Parse(err) => write!(f, "erreur de parsing catver.ini : {err}"),
            CatverError::MissingCategorySection => {
                write!(f, "la section [Category] est absente du fichier catver.ini")
            }
        }
    }
}

impl std::error::Error for CatverError {}

impl From<std::io::Error> for CatverError {
    fn from(err: std::io::Error) -> Self {
        CatverError::Io(err)
    }
}

pub fn parse_catver_file(path: &Path) -> Result<HashMap<String, String>, CatverError> {
    let content = fs::read_to_string(path)?;
    parse_catver_str(&content)
}

pub fn parse_catver_str(content: &str) -> Result<HashMap<String, String>, CatverError> {
    let mut ini = Ini::new_cs();
    let map = ini
        .read(content.to_string())
        .map_err(CatverError::Parse)?;

    let section = map
        .get(CATEGORY_SECTION)
        .ok_or(CatverError::MissingCategorySection)?;

    let mut categories = HashMap::new();
    for (game_name, category) in section.iter() {
        if let Some(category) = category {
            categories.insert(game_name.to_string(), category.trim().to_string());
        }
    }

    Ok(categories)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CATVER: &str = "\
[CATVER.INI]

[VERSION]
Version=20240101

[Category]
puckman=Maze
pacman=Maze
raiden=Shooter / Flying Vertical * Mature *
";

    #[test]
    fn parses_category_section() {
        let categories = parse_catver_str(SAMPLE_CATVER).unwrap();
        assert_eq!(categories.get("puckman"), Some(&"Maze".to_string()));
        assert_eq!(categories.get("pacman"), Some(&"Maze".to_string()));
        assert_eq!(
            categories.get("raiden"),
            Some(&"Shooter / Flying Vertical * Mature *".to_string())
        );
    }

    #[test]
    fn missing_category_section_is_reported() {
        let result = parse_catver_str("[VERSION]\nVersion=1\n");
        assert!(matches!(result, Err(CatverError::MissingCategorySection)));
    }
}
