//! Data files for collection runs: CSV with a header row, or a JSON array of objects. Each row
//! is one pass of the run; its columns fill `{{variables}}` and `pm.iterationData`.

use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::error::AppError;
use crate::request::rules::{MAX_KEY_VALUE_LENGTH, MAX_KEY_VALUE_ROWS};
use crate::request::uploads::file_label;
use crate::storage::validate::MAX_NAME_CHARS;

const MAX_DATA_FILE_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_ROWS: usize = 5_000;
const BOM: char = '\u{feff}';

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunData {
    /// The file name only; the folder it came from stays on this device.
    pub name: String,
    pub columns: Vec<String>,
    /// One value per column, in column order.
    pub rows: Vec<Vec<String>>,
}

/// Reads a data file from disk, within the size limit.
pub fn read(path: &Path) -> Result<RunData, AppError> {
    let size = fs::metadata(path).map_err(|_| AppError::FileAccess)?.len();
    if size > MAX_DATA_FILE_BYTES {
        return Err(AppError::invalid("Data files can be up to 5 MB"));
    }
    let bytes = fs::read(path).map_err(|_| AppError::FileAccess)?;
    parse(&file_label(&path.to_string_lossy()), &bytes)
}

pub fn parse(name: &str, bytes: &[u8]) -> Result<RunData, AppError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AppError::invalid("The data file must be UTF-8 text"))?
        .trim_start_matches(BOM);
    let is_json =
        name.to_ascii_lowercase().ends_with(".json") || text.trim_start().starts_with('[');
    let (columns, rows) = if is_json {
        from_json(text)?
    } else {
        from_csv(text)?
    };
    check(&columns, &rows)?;
    Ok(RunData {
        name: name.to_owned(),
        columns,
        rows,
    })
}

type Table = (Vec<String>, Vec<Vec<String>>);

fn from_csv(text: &str) -> Result<Table, AppError> {
    let mut records = csv_records(text)?.into_iter();
    let columns: Vec<String> = records
        .next()
        .ok_or_else(|| AppError::invalid("The data file is empty"))?
        .into_iter()
        .map(|column| column.trim().to_owned())
        .collect();
    let mut rows = Vec::new();
    for (index, mut record) in records.enumerate() {
        if record.len() > columns.len() {
            return Err(AppError::InvalidInput(format!(
                "Row {} has more values than there are columns",
                index + 1
            )));
        }
        record.resize(columns.len(), String::new());
        rows.push(record);
        if rows.len() > MAX_ROWS {
            break;
        }
    }
    Ok((columns, rows))
}

/// RFC 4180 records: commas between fields, quotes around fields that hold commas, quotes or
/// line breaks, and `""` for a quote inside them. Empty lines are skipped.
fn csv_records(text: &str) -> Result<Vec<Vec<String>>, AppError> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut field_start = true;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = false,
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' if field_start => quoted = true,
            ',' => {
                record.push(std::mem::take(&mut field));
                field_start = true;
                continue;
            }
            '\r' | '\n' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                end_record(&mut records, &mut record, &mut field);
                field_start = true;
                continue;
            }
            _ => field.push(c),
        }
        field_start = false;
    }
    if quoted {
        return Err(AppError::invalid(
            "A quoted value in the data file is not closed",
        ));
    }
    end_record(&mut records, &mut record, &mut field);
    Ok(records)
}

fn end_record(records: &mut Vec<Vec<String>>, record: &mut Vec<String>, field: &mut String) {
    record.push(std::mem::take(field));
    let line = std::mem::take(record);
    if !(line.len() == 1 && line[0].is_empty()) {
        records.push(line);
    }
}

/// Columns are listed as their keys first appear; within one object, keys come in name order.
fn from_json(text: &str) -> Result<Table, AppError> {
    let items: Vec<Value> = serde_json::from_str(text)
        .map_err(|_| AppError::invalid("The data file must be a JSON array of objects"))?;
    let mut columns: Vec<String> = Vec::new();
    for item in items.iter().take(MAX_ROWS + 1) {
        let Value::Object(object) = item else {
            return Err(AppError::invalid(
                "Each item in the JSON array must be an object",
            ));
        };
        for key in object.keys() {
            if !columns.contains(key) {
                columns.push(key.clone());
            }
        }
    }
    let rows = items
        .iter()
        .take(MAX_ROWS + 1)
        .map(|item| {
            columns
                .iter()
                .map(|column| item.get(column).map(value_text).unwrap_or_default())
                .collect()
        })
        .collect();
    Ok((columns, rows))
}

fn value_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn check(columns: &[String], rows: &[Vec<String>]) -> Result<(), AppError> {
    if columns.len() > MAX_KEY_VALUE_ROWS {
        return Err(AppError::invalid("The data file has more than 100 columns"));
    }
    for (index, column) in columns.iter().enumerate() {
        if column.is_empty() || column.chars().count() > MAX_NAME_CHARS {
            return Err(AppError::InvalidInput(format!(
                "Column {} needs a name of up to 120 characters",
                index + 1
            )));
        }
        if columns[..index].contains(column) {
            return Err(AppError::InvalidInput(format!(
                "The column “{column}” appears twice"
            )));
        }
    }
    if rows.is_empty() {
        return Err(AppError::invalid("The data file has no rows"));
    }
    if rows.len() > MAX_ROWS {
        return Err(AppError::invalid("The data file has more than 5000 rows"));
    }
    if rows
        .iter()
        .flatten()
        .any(|value| value.len() > MAX_KEY_VALUE_LENGTH)
    {
        return Err(AppError::invalid(
            "A value in the data file is longer than 8 KB",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(data: &RunData) -> (Vec<&str>, Vec<Vec<&str>>) {
        (
            data.columns.iter().map(String::as_str).collect(),
            data.rows
                .iter()
                .map(|row| row.iter().map(String::as_str).collect())
                .collect(),
        )
    }

    #[test]
    fn reads_csv_with_quotes_line_breaks_and_short_rows() {
        let text = "\u{feff}id, email ,note\r\n1,ada@example.test,\"Hello, \"\"world\"\"\"\n\n2,bo@example.test,\"two\nlines\"\n3,cy@example.test\n";
        let data = parse("users.csv", text.as_bytes()).expect("parsed");
        let (columns, rows) = table(&data);
        assert_eq!(columns, ["id", "email", "note"]);
        assert_eq!(
            rows,
            [
                vec!["1", "ada@example.test", "Hello, \"world\""],
                vec!["2", "bo@example.test", "two\nlines"],
                vec!["3", "cy@example.test", ""],
            ]
        );
    }

    #[test]
    fn reads_json_arrays_of_objects() {
        let text = r#"[{"id": 1, "email": "ada@example.test"}, {"id": 2, "admin": true, "tags": ["a"], "email": null}]"#;
        let data = parse("users.json", text.as_bytes()).expect("parsed");
        let (columns, rows) = table(&data);
        assert_eq!(columns, ["email", "id", "admin", "tags"]);
        assert_eq!(
            rows,
            [
                vec!["ada@example.test", "1", "", ""],
                vec!["", "2", "true", "[\"a\"]"],
            ]
        );
    }

    #[test]
    fn refuses_files_it_cannot_use() {
        let error =
            |name: &str, text: &str| parse(name, text.as_bytes()).err().map(|e| e.to_string());
        assert_eq!(
            error("a.csv", "id,id\n1,2"),
            Some("The column “id” appears twice".to_owned())
        );
        assert_eq!(
            error("a.csv", "id\n1,2"),
            Some("Row 1 has more values than there are columns".to_owned())
        );
        assert_eq!(
            error("a.csv", "id\n"),
            Some("The data file has no rows".to_owned())
        );
        assert_eq!(
            error("a.csv", "id\n\"open"),
            Some("A quoted value in the data file is not closed".to_owned())
        );
        assert_eq!(
            error("a.json", "[1, 2]"),
            Some("Each item in the JSON array must be an object".to_owned())
        );
        assert!(parse("a.csv", &[0xff, 0xfe]).is_err());
    }
}
