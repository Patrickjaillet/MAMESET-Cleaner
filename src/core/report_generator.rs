use std::fmt;
use std::fs;
use std::path::Path;

use crate::core::cleanup_engine::CleanupRecord;

#[derive(Debug)]
pub enum ReportError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportError::Io(err) => write!(f, "erreur d'écriture du rapport : {err}"),
            ReportError::Json(err) => write!(f, "erreur de format du rapport : {err}"),
        }
    }
}

impl std::error::Error for ReportError {}

impl From<std::io::Error> for ReportError {
    fn from(err: std::io::Error) -> Self {
        ReportError::Io(err)
    }
}

impl From<serde_json::Error> for ReportError {
    fn from(err: serde_json::Error) -> Self {
        ReportError::Json(err)
    }
}

pub fn write_json_report(records: &[CleanupRecord], path: &Path) -> Result<(), ReportError> {
    let content = serde_json::to_string_pretty(records)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn write_csv_report(records: &[CleanupRecord], path: &Path) -> Result<(), ReportError> {
    let mut content =
        String::from("name,file_path,reason,action,backed_up_to,error\n");

    for record in records {
        content.push_str(&csv_escape(&record.name));
        content.push(',');
        content.push_str(&csv_escape(&record.file_path));
        content.push(',');
        content.push_str(&csv_escape(&record.reason));
        content.push(',');
        content.push_str(&csv_escape(&record.action));
        content.push(',');
        content.push_str(&csv_escape(record.backed_up_to.as_deref().unwrap_or("")));
        content.push(',');
        content.push_str(&csv_escape(record.error.as_deref().unwrap_or("")));
        content.push('\n');
    }

    fs::write(path, content)?;
    Ok(())
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_records() -> Vec<CleanupRecord> {
        vec![
            CleanupRecord {
                name: "gamea".to_string(),
                file_path: "C:/roms/gamea.zip".to_string(),
                reason: "doublon (1G1R)".to_string(),
                action: "corbeille".to_string(),
                backed_up_to: None,
                error: None,
            },
            CleanupRecord {
                name: "gameb, spécial".to_string(),
                file_path: "C:/roms/gameb.zip".to_string(),
                reason: "filtre \"genre\"".to_string(),
                action: "supprimé".to_string(),
                backed_up_to: Some("C:/backup/gameb.zip".to_string()),
                error: None,
            },
        ]
    }

    fn temp_path(label: &str, extension: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "mameset_cleaner_report_{label}_{}.{extension}",
            std::process::id()
        ))
    }

    #[test]
    fn writes_a_valid_json_report() {
        let path = temp_path("json", "json");
        let records = sample_records();

        write_json_report(&records, &path).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let parsed: Vec<CleanupRecord> = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "gamea");
        assert_eq!(parsed[1].backed_up_to.as_deref(), Some("C:/backup/gameb.zip"));

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn writes_a_csv_report_with_header_and_escaping() {
        let path = temp_path("csv", "csv");
        let records = sample_records();

        write_csv_report(&records, &path).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let mut lines = content.lines();

        assert_eq!(
            lines.next(),
            Some("name,file_path,reason,action,backed_up_to,error")
        );
        assert_eq!(lines.next(), Some("gamea,C:/roms/gamea.zip,doublon (1G1R),corbeille,,"));
        assert_eq!(
            lines.next(),
            Some("\"gameb, spécial\",C:/roms/gameb.zip,\"filtre \"\"genre\"\"\",supprimé,C:/backup/gameb.zip,")
        );

        fs::remove_file(&path).unwrap();
    }
}
